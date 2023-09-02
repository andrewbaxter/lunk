use std::{
    rc::{
        Rc,
        Weak,
    },
    cell::{
        RefCell,
        Cell,
    },
    collections::{
        HashMap,
        HashSet,
    },
};

/// A unique id for all items in the graph (links and values). Starts from 1, 0 is
/// invalid.
pub type Id = usize;
pub const NULL_ID: Id = 0;
type PendingCount = i32;

pub(crate) trait ValueTrait {
    fn id(&self) -> Id;
    fn add_next(&self, link: Weak<Link_>);
}

pub struct Value(pub(crate) Rc<dyn ValueTrait>);

pub(crate) trait Cleanup {
    fn clean(&self);
}

/// Helper method for implementing `inputs` in `LinkTrait`.
pub trait UpgradeValue {
    fn upgrade_as_value(&self) -> Option<Value>;
}

/// Behavior required for manually defining links.
pub trait LinkTrait {
    /// Called when all dirty inputs (dependencies, per `inputs`) have been processed,
    /// if there's at least one dirty input.
    fn call(&self, pc: &mut ProcessingContext);

    /// Returns inputs (as `Value` trait for generic processing).
    fn inputs(&self) -> Vec<Value>;
}

pub(crate) struct Link_ {
    pub(crate) id: Id,
    inner: Box<dyn LinkTrait>,
    pending_inputs: Cell<PendingCount>,
}

impl Link_ {
    pub(crate) fn process(&self, pc: &mut ProcessingContext) {
        self.inner.call(pc);
    }

    pub(crate) fn deps(&self) -> Vec<Value> {
        return self.inner.inputs();
    }

    pub(crate) fn set_dep_pending(&self, count: PendingCount) {
        self.pending_inputs.set(count);
    }

    pub(crate) fn dec_dep_pending(&self) -> PendingCount {
        let mut p = self.pending_inputs.get();
        p -= 1;
        self.pending_inputs.set(p);
        return p;
    }
}

impl Cleanup for Link_ {
    fn clean(&self) {
        self.pending_inputs.set(self.deps().len() as PendingCount);
    }
}

/// A link, representing processing taking some inputs and modifying outputs.  This
/// object is just an ownership root, it's not particularly interactive.
#[derive(Clone)]
pub struct Link(pub(crate) Rc<Link_>);

impl Link {
    /// Create a link with the given `LinkCb`.  The new link will immediately be
    /// scheduled to be run once, when the current `EventContext` event function
    /// invocation ends.  To ensure that this is ordered properly during the initial
    /// run, for each output value you should call
    /// `pc.mark_new_output_value(output.id())`.
    ///
    /// The link will only continue to be triggered as long as the `Link` object
    /// exists, dropping it will deactivate that graph path.
    #[must_use]
    pub fn new(pc: &mut ProcessingContext, inner: impl LinkTrait + 'static) -> Self {
        let pending = inner.inputs().len() as PendingCount;
        let id = pc.1.take_id();
        let out = Link(Rc::new(Link_ {
            id: id,
            inner: Box::new(inner),
            pending_inputs: Cell::new(pending),
        }));
        pc.1.new_links.insert(id, Rc::downgrade(&out.0));
        return out;
    }
}

pub struct _Context {
    pub(crate) new_links: HashMap<Id, Weak<Link_>>,
    pub(crate) queued_links: Vec<Weak<Link_>>,
    pub(crate) processed_links: HashSet<Id>,
    pub(crate) new_output_values: HashSet<Id>,
    pub(crate) cleanup: Vec<Rc<dyn Cleanup>>,
    ids: usize,
}

impl _Context {
    pub(crate) fn take_id(&mut self) -> Id {
        let id = self.ids;
        self.ids += 1;
        return id;
    }
}

/// This manages the graph.  The `event` function is the entrypoint to most graph
/// interactions.
#[derive(Clone)]
pub struct EventGraph(Rc<RefCell<_Context>>);

/// Context used during the processing of a single event.  You should pass this
/// around as a `&mut` and probably not store it persistently.
pub struct ProcessingContext<'a>(pub(crate) &'a EventGraph, pub(crate) &'a mut _Context);

impl EventGraph {
    pub fn new() -> EventGraph {
        return EventGraph(Rc::new(RefCell::new(_Context {
            new_links: Default::default(),
            queued_links: Default::default(),
            processed_links: Default::default(),
            new_output_values: Default::default(),
            cleanup: vec![],
            ids: 1,
        })));
    }

    /// This is a wrapper that runs the event graph after the callback finishes. You
    /// should call this whenever an event happens (user input, remote notification,
    /// etc) as well as during initial setup, and do all graph manipulation from within
    /// the callback.
    pub fn event<Z>(&self, f: impl FnOnce(&mut ProcessingContext) -> Z) -> Z {
        let mut s = self.0.borrow_mut();

        // Do initial changes (modifying values, modifying graph)
        let out = f(&mut ProcessingContext(self, &mut *s));

        // Walk the graph starting from dirty nodes, processing callbacks in order
        while let Some(l) = s.queued_links.pop().and_then(|l| l.upgrade()) {
            if !s.processed_links.insert(l.id) {
                continue;
            }
            l.as_ref().process(&mut ProcessingContext(self, &mut *s));
            s.cleanup.push(l);
        }

        // Walk the (not yet connected) subgraph of new nodes in order to immediately
        // activate all new nodes. This relies on graph additions being offshoots of the
        // existing graph (i.e. nothing in the existing graph depends on the additions).
        let new: Vec<(Id, Weak<Link_>)> = s.new_links.drain().collect();
        for l in new.into_iter().filter_map(|(_, e)| e.upgrade()) {
            let mut new_count = 0 as PendingCount;
            for prev in l.deps() {
                // Attach to graph
                prev.0.add_next(Rc::downgrade(&l));

                // For new links, assume all inputs that are already in the graph have changed, as
                // well as root input values for new links.  That means only values that are
                // outputs of new links should be considered pending and therefore ordered after.
                if s.new_output_values.contains(&prev.0.id()) {
                    new_count += 1;
                }
            }
            l.set_dep_pending(new_count);
            if new_count == 0 {
                s.queued_links.push(Rc::downgrade(&l));
            }
        }
        while let Some(l) = s.queued_links.pop().and_then(|e| e.upgrade()) {
            if !s.processed_links.insert(l.id) {
                continue;
            }
            l.as_ref().process(&mut ProcessingContext(self, &mut *s));
            s.cleanup.push(l);
        }

        // Cleanup
        s.new_output_values.clear();
        s.processed_links.clear();
        for p in s.cleanup.drain(0..) {
            p.clean();
        }
        return out;
    }
}

impl<'a> ProcessingContext<'a> {
    /// Get the event graph that created this processing context (so you don't need to
    /// pass it around along with the processing context).
    pub fn eg(&self) -> EventGraph {
        return self.0.clone();
    }

    /// Used for manual link implementation.  See `Link::new` for details.
    pub fn mark_new_output_value(&mut self, id: Id) {
        self.1.new_output_values.insert(id);
    }
}
