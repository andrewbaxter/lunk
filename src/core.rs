use std::{
    rc::{
        Rc,
    },
    cell::{
        RefCell,
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

pub trait ValueTrait {
    fn id(&self) -> Id;
    fn next(&self) -> Vec<Link>;
}

pub struct Value(pub(crate) Rc<dyn ValueTrait>);

pub trait IntoValue {
    fn into_value(&self) -> Value;
}

pub(crate) trait Cleanup {
    fn clean(&self);
}

/// Behavior required for manually defining links.
pub trait LinkTrait {
    /// Called when all dirty inputs (dependencies, per `inputs`) have been processed,
    /// if there's at least one dirty input.
    fn call(&self, pc: &mut ProcessingContext);

    /// Returns outputs (downstream values; as `Value` trait for generic processing).
    fn next(&self) -> Vec<Value>;
}

pub(crate) struct Link_ {
    pub(crate) id: Id,
    inner: Box<dyn LinkTrait>,
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
        let id = pc.1.take_id();
        let out = Link(Rc::new(Link_ {
            id: id,
            inner: Box::new(inner),
        }));
        pc.1.stg1_queued_links.push(out.clone());
        return out;
    }
}

pub struct _Context {
    pub(crate) roots: HashMap<Id, Link>,
    pub(crate) stg1_queued_links: Vec<Link>,
    pub(crate) affected_links: HashSet<Id>,
    pub(crate) stg2_leaves: Vec<Link>,
    pub(crate) stg2_up: HashMap<Id, Vec<Link>>,
    pub(crate) stg2_queued_links: Vec<(bool, Link)>,
    pub(crate) stg2_seen_links: HashSet<Id>,
    pub(crate) stg2_seen_up: HashSet<Id>,
    pub(crate) stg2_buf_delay: Vec<Link>,
    pub(crate) cleanup: Vec<Rc<dyn Cleanup>>,
    pub(crate) processing: bool,
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
            roots: Default::default(),
            stg1_queued_links: Default::default(),
            affected_links: Default::default(),
            stg2_leaves: Default::default(),
            stg2_up: Default::default(),
            stg2_queued_links: Default::default(),
            stg2_seen_links: Default::default(),
            stg2_seen_up: Default::default(),
            stg2_buf_delay: Default::default(),
            cleanup: vec![],
            processing: false,
            ids: 1,
        })));
    }

    /// This is a wrapper that runs the event graph after the callback finishes. You
    /// should call this whenever an event happens (user input, remote notification,
    /// etc) as well as during initial setup, and do all graph manipulation from within
    /// the callback.
    pub fn event(&self, f: impl FnOnce(&mut ProcessingContext)) {
        let mut s = self.0.borrow_mut();
        if s.processing {
            return;
        }

        // Do initial changes (modifying values, modifying graph)
        let out = f(&mut ProcessingContext(self, &mut *s));
        s.processing = true;
        let queue_roots: Vec<Link> = s.roots.values().cloned().collect();
        s.stg1_queued_links.extend(queue_roots);

        // Process graph (repeatedly, for new subgraph updates during processing)
        while !s.stg1_queued_links.is_empty() {
            // Walk graph once, starting from (links downstream from) modified values and new
            // links, to:
            //
            // * Identify affected links for next pass
            //
            // * Identify leaves to start next pass
            //
            // * Build deps tree from reverse deps
            while let Some(link) = s.stg1_queued_links.pop() {
                if !s.affected_links.insert(link.0.id) {
                    continue;
                }
                let mut has_next = false;
                for next_val in link.0.inner.next() {
                    for next_link in next_val.0.next() {
                        has_next = true;
                        s.stg1_queued_links.push(next_link.clone());
                        s.stg2_up.entry(next_link.0.id).or_insert_with(Vec::new).push(link.clone());
                    }
                }
                if !has_next {
                    s.stg2_leaves.push(link);
                }
            }

            // Walk deps from leaves, only considering affected nodes
            let queue_leaves: Vec<(bool, Link)> = s.stg2_leaves.drain(0..).map(|l| (true, l)).collect();
            s.stg2_queued_links.extend(queue_leaves);
            while let Some((first, link)) = s.stg2_queued_links.pop() {
                if first {
                    if !s.stg2_seen_links.insert(link.0.id) {
                        continue;
                    }
                    s.stg2_queued_links.push((false, link.clone()));
                    for prev_link in s.stg2_up.remove(&link.0.id).unwrap_or(vec![]) {
                        if !s.stg2_seen_up.insert(prev_link.0.id) {
                            continue;
                        }
                        if !s.affected_links.contains(&prev_link.0.id) {
                            continue;
                        }
                        if s.roots.contains_key(&prev_link.0.id) {
                            // Roots (modified nodes) pushed to the queue last so they're processed first,
                            // breaking cycles
                            s.stg2_buf_delay.push(prev_link);
                        } else {
                            s.stg2_queued_links.push((true, prev_link));
                        }
                    }
                    while let Some(prev_link) = s.stg2_buf_delay.pop() {
                        s.stg2_queued_links.push((true, prev_link));
                    }
                    s.stg2_seen_up.clear();
                } else {
                    (link.0.inner).call(&mut ProcessingContext(self, &mut s));
                }
            }
            s.stg2_seen_links.clear();
        }
        s.affected_links.clear();
        s.roots.clear();
        debug_assert!(s.stg1_queued_links.is_empty());
        debug_assert!(s.stg2_leaves.is_empty());
        debug_assert!(s.stg2_up.is_empty());
        debug_assert!(s.stg2_queued_links.is_empty());
        debug_assert!(s.stg2_seen_links.is_empty());
        debug_assert!(s.stg2_seen_up.is_empty());
        debug_assert!(s.stg2_buf_delay.is_empty());

        // Cleanup
        for p in s.cleanup.drain(0..) {
            p.clean();
        }
        s.processing = false;
        return out;
    }
}

impl<'a> ProcessingContext<'a> {
    /// Get the event graph that created this processing context (so you don't need to
    /// pass it around along with the processing context).
    pub fn eg(&self) -> EventGraph {
        return self.0.clone();
    }
}
