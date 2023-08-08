use std::{
    rc::{
        Rc,
        Weak,
    },
    cell::RefCell,
    collections::{
        HashMap,
    },
};

pub type Null = ();
pub type Id = usize;
pub type PendingCount = i32;

pub trait _ValueTrait {
    fn id(&self) -> Id;
    fn add_next(&mut self, link: WeakLink);
    fn clean(&mut self);
}

pub type Value = Rc<RefCell<dyn _ValueTrait>>;

pub trait _UpgradeValue {
    fn upgrade_as_value(&self) -> Option<Value>;
}

impl<T: _ValueTrait + 'static> _UpgradeValue for Rc<RefCell<T>> {
    fn upgrade_as_value(&self) -> Option<Value> {
        return Some(self.clone() as Value);
    }
}

impl<T: _ValueTrait + 'static> _UpgradeValue for Weak<RefCell<T>> {
    fn upgrade_as_value(&self) -> Option<Value> {
        return self.upgrade().map(|x| x as Value);
    }
}

pub trait LinkCb<V> {
    /// Called when all dirty inputs (dependencies, per `inputs`) have been processed,
    /// if there's at least one dirty input.  Should update `output` based on inputs.
    fn call(&self, ctx: &mut EventProcessingContext, output: &Rc<RefCell<V>>);

    /// Returns inputs (as `Value` trait for generic processing).
    fn inputs(&self) -> Vec<Value>;
}

struct _Link<V: _ValueTrait> {
    value: Rc<RefCell<V>>,
    inner: Box<dyn LinkCb<V>>,
    pending_inputs: PendingCount,
}

pub trait _LinkTrait {
    fn process(&mut self, ctx: &mut EventProcessingContext);
    fn prev(&mut self) -> Vec<Value>;
    fn id(&self) -> Id;
    fn clean(&mut self);
    fn get_prev_pending(&mut self) -> PendingCount;
    fn set_prev_pending(&mut self, count: PendingCount);
    fn dec_prev_pending(&mut self) -> PendingCount;
}

impl<V: _ValueTrait> _LinkTrait for _Link<V> {
    fn process(&mut self, ctx: &mut EventProcessingContext) {
        self.inner.call(ctx, &self.value);
    }

    fn prev(&mut self) -> Vec<Value> {
        return self.inner.inputs();
    }

    fn id(&self) -> Id {
        return self.value.borrow().id();
    }

    fn clean(&mut self) {
        self.pending_inputs = self.prev().len() as PendingCount;
        self.value.borrow_mut().clean();
    }

    fn get_prev_pending(&mut self) -> PendingCount {
        return self.pending_inputs;
    }

    fn set_prev_pending(&mut self, count: PendingCount) {
        self.pending_inputs = count;
    }

    fn dec_prev_pending(&mut self) -> PendingCount {
        self.pending_inputs -= 1;
        return self.pending_inputs;
    }
}

pub type Link = Rc<RefCell<dyn _LinkTrait>>;
pub type WeakLink = Weak<RefCell<dyn _LinkTrait>>;

pub fn new_link<
    V: _ValueTrait + 'static,
>(ctx: &mut EventProcessingContext, value: Rc<RefCell<V>>, inner: impl LinkCb<V> + 'static) -> Link {
    let pending = inner.inputs().len() as PendingCount;
    let id = value.borrow().id();
    let out = Rc::new(RefCell::new(_Link {
        value: value,
        inner: Box::new(inner),
        pending_inputs: pending,
    }));
    ctx.1.new.insert(id, Rc::downgrade(&(out.clone() as Link)));
    return out;
}

#[derive(Default)]
pub struct _Context {
    pub(crate) new: HashMap<Id, WeakLink>,
    pub(crate) queue: Vec<WeakLink>,
    pub(crate) processed: HashMap<Id, Box<dyn FnOnce()>>,
    link_ids: usize,
}

impl _Context {
    pub(crate) fn take_id(&mut self) -> Id {
        let id = self.link_ids;
        self.link_ids += 1;
        return id;
    }
}

/// This manages the graph.  The `event` function is the entrypoint to most graph
/// interactions.
#[derive(Clone)]
pub struct EventContext(Rc<RefCell<_Context>>);

pub struct EventProcessingContext<'a>(pub(crate) &'a EventContext, pub(crate) &'a mut _Context);

impl EventContext {
    pub fn new() -> EventContext {
        return EventContext(Rc::new(RefCell::new(_Context::default())));
    }

    /// This is a wrapper that runs the event graph after the callback finishes. You
    /// should call this whenever an event happens (user input, remote notification,
    /// etc) as well as during initial setup.
    pub fn event<Z>(&self, f: impl FnOnce(&mut EventProcessingContext) -> Z) -> Z {
        let mut s = self.0.borrow_mut();

        // Do initial changes (modifying values, modifying graph)
        let out = f(&mut EventProcessingContext(self, &mut *s));

        // Walk the graph starting from dirty nodes, processing callbacks in order
        while let Some(l) = s.queue.pop().and_then(|l| l.upgrade()) {
            let id = l.borrow().id();
            if s.processed.contains_key(&id) {
                continue;
            }
            l.as_ref().borrow_mut().process(&mut EventProcessingContext(self, &mut *s));
            s.processed.insert(id, Box::new(move || l.borrow_mut().clean()));
        }

        // Walk the (not yet connected) subgraph of new nodes in order to immediately
        // activate all new nodes. This relies on graph additions being offshoots of the
        // existing graph (i.e. nothing in the existing graph depends on the additions).
        let new: Vec<(Id, WeakLink)> = s.new.drain().collect();
        for l in new.into_iter().filter_map(|(_, e)| e.upgrade()) {
            let mut l_inner = l.borrow_mut();
            let mut new_count = 0 as PendingCount;
            for prev in l_inner.prev() {
                // Attach to graph
                prev.borrow_mut().add_next(Rc::downgrade(&l));

                // Assume all non-new inputs are dirty (and processed) - we only want to delay for
                // not-yet processed new tree deps.
                if s.new.contains_key(&prev.borrow().id()) {
                    new_count += 1;
                }
            }
            l_inner.set_prev_pending(new_count);
            if new_count == 0 {
                s.queue.push(Rc::downgrade(&l));
            }
        }
        while let Some(l) = s.queue.pop().and_then(|e| e.upgrade()) {
            let id = l.borrow().id();
            if s.processed.contains_key(&id) {
                continue;
            }
            l.as_ref().borrow_mut().process(&mut EventProcessingContext(self, &mut *s));
            s.processed.insert(id, Box::new(move || l.borrow_mut().clean()));
        }

        // Cleanup
        for (_, p) in s.processed.drain() {
            (p)();
        }
        return out;
    }
}

impl<'a> EventProcessingContext<'a> {
    pub fn ctx(&self) -> EventContext {
        return self.0.clone();
    }
}
