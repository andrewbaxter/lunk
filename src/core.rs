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

pub type Id = usize;
pub const NULL_ID: Id = 0;
type PendingCount = i32;

pub trait _IntValueTrait {
    fn id(&self) -> Id;
    fn add_next(&mut self, link: WeakLink);
}

pub trait _ExtValueTrait {
    fn id(&self) -> Id;
    fn clean(&self);
}

pub type Value = Rc<RefCell<dyn _IntValueTrait>>;

/// Helper method for implementing `inputs` in `LinkCb`.
pub trait UpgradeValue {
    fn upgrade_as_value(&self) -> Option<Value>;
}

pub trait LinkCb<V> {
    /// Called when all dirty inputs (dependencies, per `inputs`) have been processed,
    /// if there's at least one dirty input.  Should update `output` based on inputs.
    fn call(&self, pc: &mut ProcessingContext, output: &V);

    /// Returns inputs (as `Value` trait for generic processing).
    fn inputs(&self) -> Vec<Value>;
}

struct _Link<V: _ExtValueTrait> {
    value: V,
    inner: Box<dyn LinkCb<V>>,
    pending_inputs: PendingCount,
}

pub trait _LinkTrait {
    fn process(&mut self, pc: &mut ProcessingContext);
    fn prev(&mut self) -> Vec<Value>;
    fn id(&self) -> Id;
    fn clean(&mut self);
    fn get_prev_pending(&mut self) -> PendingCount;
    fn set_prev_pending(&mut self, count: PendingCount);
    fn dec_prev_pending(&mut self) -> PendingCount;
}

impl<V: _ExtValueTrait> _LinkTrait for _Link<V> {
    fn process(&mut self, pc: &mut ProcessingContext) {
        self.inner.call(pc, &self.value);
    }

    fn prev(&mut self) -> Vec<Value> {
        return self.inner.inputs();
    }

    fn id(&self) -> Id {
        return self.value.id();
    }

    fn clean(&mut self) {
        self.pending_inputs = self.prev().len() as PendingCount;
        self.value.clean();
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

/// Create a link with the given `LinkCb` processor and output `value`.  The link
/// will be scheduled to be updated once the current `EventContext` event function
/// invocation ends.
#[must_use]
pub fn new_link<
    V: _ExtValueTrait + 'static,
>(pc: &mut ProcessingContext, value: V, inner: impl LinkCb<V> + 'static) -> Link {
    let pending = inner.inputs().len() as PendingCount;
    let id = value.id();
    let out = Rc::new(RefCell::new(_Link {
        value: value,
        inner: Box::new(inner),
        pending_inputs: pending,
    }));
    pc.1.new.insert(id, Rc::downgrade(&(out.clone() as Link)));
    return out;
}

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
pub struct EventGraph(Rc<RefCell<_Context>>);

/// Context used during event processing.
pub struct ProcessingContext<'a>(pub(crate) &'a EventGraph, pub(crate) &'a mut _Context);

impl EventGraph {
    pub fn new() -> EventGraph {
        return EventGraph(Rc::new(RefCell::new(_Context {
            new: Default::default(),
            queue: Default::default(),
            processed: Default::default(),
            link_ids: 1,
        })));
    }

    /// This is a wrapper that runs the event graph after the callback finishes. You
    /// should call this whenever an event happens (user input, remote notification,
    /// etc) as well as during initial setup.
    pub fn event<Z>(&self, f: impl FnOnce(&mut ProcessingContext) -> Z) -> Z {
        let mut s = self.0.borrow_mut();

        // Do initial changes (modifying values, modifying graph)
        let out = f(&mut ProcessingContext(self, &mut *s));

        // Walk the graph starting from dirty nodes, processing callbacks in order
        while let Some(l) = s.queue.pop().and_then(|l| l.upgrade()) {
            let id = l.borrow().id();
            if s.processed.contains_key(&id) {
                continue;
            }
            l.as_ref().borrow_mut().process(&mut ProcessingContext(self, &mut *s));
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
            l.as_ref().borrow_mut().process(&mut ProcessingContext(self, &mut *s));
            s.processed.insert(id, Box::new(move || l.borrow_mut().clean()));
        }

        // Cleanup
        for (_, p) in s.processed.drain() {
            (p)();
        }
        return out;
    }
}

impl<'a> ProcessingContext<'a> {
    pub fn eg(&self) -> EventGraph {
        return self.0.clone();
    }
}
