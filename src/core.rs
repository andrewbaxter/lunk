use {
    std::{
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
    },
};

/// A unique id for all items in the graph (links and values). Starts from 1, 0 is
/// invalid.
pub type Id = usize;
pub const NULL_ID: Id = 0;

pub trait ValueTrait {
    fn next_links(&self) -> Vec<Link>;
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
    fn next_values(&self) -> Vec<Value>;
}

pub(crate) struct Link_ {
    pub(crate) id: Id,
    inner: Box<dyn LinkTrait>,
}

/// A link, representing processing taking some inputs and modifying outputs.  This
/// object is just an ownership root, it's not particularly interactive.
#[derive(Clone)]
pub struct Link(pub(crate) Rc<Link_>);

impl std::hash::Hash for Link {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.id.hash(state);
    }
}

impl PartialEq for Link {
    fn eq(&self, other: &Self) -> bool {
        return self.0.id == other.0.id;
    }
}

impl Eq for Link { }

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
        pc.1.step1_stacked_links.push((true, out.clone()));
        return out;
    }
}

pub struct _Context {
    pub(crate) step1_stacked_links: Vec<(bool, Link)>,
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
            step1_stacked_links: Default::default(),
            cleanup: vec![],
            processing: false,
            ids: 1,
        })));
    }

    /// This is a wrapper that runs the event graph after the callback finishes. You
    /// should call this whenever an event happens (user input, remote notification,
    /// etc) as well as during initial setup, and do all graph manipulation from within
    /// the callback.
    ///
    /// If this is called re-entrantly, the latter invocation will be ignored (the
    /// callback) won't be run and it will return `None`.
    pub fn event<R>(&self, f: impl FnOnce(&mut ProcessingContext) -> R) -> Option<R> {
        // On the graph algorithm, and cycles:
        //
        // We start at links and not nodes because we want to trigger from newly added
        // links, whether or not the node is new or not (so links must be tracked
        // independently from nodes).
        //
        // This works well with our definition of a cycle, which is a link that leads to a
        // link that was already run (via some node).
        let Ok(mut s) = self.0.try_borrow_mut() else {
            return None;
        };
        if s.processing {
            return None;
        }

        // Do initial changes (modifying values, modifying graph)
        let out = f(&mut ProcessingContext(self, &mut *s));
        s.processing = true;

        // Process graph (repeatedly, for new subgraph updates during processing)
        let mut involved_links = HashSet::new();
        let mut processed_links = HashSet::new();
        let mut step12_leaves = vec![];
        let mut step2_upstream_dep_tree: HashMap<Id, HashSet<Link>> = HashMap::new();
        let mut step2_stacked_links = vec![];
        let mut step2_seen_up = HashSet::new();
        while !s.step1_stacked_links.is_empty() {
            // Step 1, walk graph once starting from (links downstream from) modified values
            // and new links in order to:
            //
            // * Identify upstream leaves - which become roots for 2nd step reverse traversal
            //
            // * Build upstream dep tree for 2nd step traversal
            //
            // * Identify involved links in affected subgraph to limit 2nd step
            struct Step1PathEntry {
                link: Link,
                downstream: usize,
            }

            let mut path_stack: Vec<Step1PathEntry> = vec![];
            s.step1_stacked_links.reverse();
            'stack_next: while let Some((first, link)) = s.step1_stacked_links.pop() {
                if first {
                    // Merging paths, don't reprocess
                    if !involved_links.insert(link.0.id) {
                        continue;
                    }

                    // Classify by being a cycle link or not
                    let mut outputs = vec![];
                    for next_val in link.0.inner.next_values() {
                        for next_link in next_val.0.next_links() {
                            // Check if next link makes a cycle and skip
                            for path_entry in &path_stack {
                                if path_entry.link.0.id == next_link.0.id {
                                    continue 'stack_next;
                                }
                            }

                            // Not a cycle
                            outputs.push(next_link.clone());
                        }
                    }
                    if let Some(parent) = path_stack.last_mut() {
                        // Add as upstream dep from parent Update parent stats
                        parent.downstream += 1;
                    }

                    // Stack 2nd pass
                    s.step1_stacked_links.push((false, link.clone()));

                    // Stack parent info
                    path_stack.push(Step1PathEntry {
                        link: link.clone(),
                        downstream: 0,
                    });

                    // Stack children and establish child dependencies
                    for next_link in outputs {
                        step2_upstream_dep_tree.entry(next_link.0.id).or_default().insert(link.clone());
                        s.step1_stacked_links.push((true, next_link));
                    }
                } else {
                    // Unwind - use post-processing stats to determine if leaf (by real downstream)
                    let totals = path_stack.pop().unwrap();
                    if totals.downstream == 0 {
                        step12_leaves.push(link);
                    }
                }
            }

            // Walk deps from leaves, only considering affected nodes
            let queue_leaves: Vec<(bool, Link)> = step12_leaves.drain(0..).map(|l| (true, l)).collect();
            step2_stacked_links.extend(queue_leaves);
            while let Some((first, link)) = step2_stacked_links.pop() {
                if first {
                    if !processed_links.insert(link.0.id) {
                        continue;
                    }
                    step2_stacked_links.push((false, link.clone()));
                    for prev_link in step2_upstream_dep_tree.remove(&link.0.id).unwrap_or_default() {
                        if !step2_seen_up.insert(prev_link.0.id) {
                            continue;
                        }
                        if !involved_links.contains(&prev_link.0.id) {
                            continue;
                        }
                        step2_stacked_links.push((true, prev_link));
                    }
                    step2_seen_up.clear();
                } else {
                    (link.0.inner).call(&mut ProcessingContext(self, &mut s));
                }
            }
        }

        // Cleanup
        debug_assert!(step2_seen_up.is_empty());
        for p in s.cleanup.drain(0..) {
            p.clean();
        }
        s.processing = false;
        return Some(out);
    }
}

impl<'a> ProcessingContext<'a> {
    /// Get the event graph that created this processing context (so you don't need to
    /// pass it around along with the processing context).
    pub fn eg(&self) -> EventGraph {
        return self.0.clone();
    }
}
