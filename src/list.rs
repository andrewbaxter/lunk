use std::{
    rc::{
        Rc,
        Weak,
    },
    cell::{
        RefCell,
        Ref,
    },
    ops::Deref,
};
use crate::core::{
    Id,
    ValueTrait,
    ProcessingContext,
    UpgradeValue,
    Value,
    Cleanup,
    Link_,
};

pub struct Change<T: Clone> {
    pub offset: usize,
    pub remove: usize,
    pub add: std::vec::Vec<T>,
}

pub struct ListMut_<T: Clone> {
    value: std::vec::Vec<T>,
    changes: std::vec::Vec<Change<T>>,
    next: std::vec::Vec<Weak<Link_>>,
}

struct List_<T: Clone> {
    id: Id,
    mut_: RefCell<ListMut_<T>>,
}

impl<T: Clone> ValueTrait for List_<T> {
    fn id(&self) -> Id {
        return self.id;
    }

    fn add_next(&self, link: Weak<Link_>) {
        self.mut_.borrow_mut().next.push(link);
    }
}

impl<T: Clone> Cleanup for List_<T> {
    fn clean(&self) {
        self.mut_.borrow_mut().changes.clear();
    }
}

/// A value that manages an ordered list of values.
#[derive(Clone)]
pub struct List<T: Clone>(Rc<List_<T>>);

#[derive(Clone)]
pub struct WeakList<T: Clone>(Weak<List_<T>>);

impl<T: Clone + 'static> List<T> {
    pub fn new(pc: &mut ProcessingContext, initial: std::vec::Vec<T>) -> Self {
        let id = pc.1.take_id();
        return List(Rc::new(List_ {
            id: id,
            mut_: RefCell::new(ListMut_ {
                value: initial,
                changes: vec![],
                next: vec![],
            }),
        }));
    }

    pub fn id(&self) -> Id {
        return self.0.id;
    }

    /// Get a weak reference to the list.
    pub fn weak(&self) -> WeakList<T> {
        return WeakList(Rc::downgrade(&self.0));
    }

    fn splice_(
        &self,
        self2: &mut ListMut_<T>,
        pc: &mut ProcessingContext,
        offset: usize,
        remove: usize,
        add: std::vec::Vec<T>,
    ) -> std::vec::Vec<T> {
        if remove == 0 && add.is_empty() {
            return vec![];
        }
        let first_change = self2.changes.is_empty();
        let out = self2.value.splice(offset .. offset + remove, add.clone()).collect();
        self2.changes.push(Change {
            offset: offset,
            remove: remove,
            add: add,
        });
        if first_change {
            pc.1.cleanup.push(self.0.clone());
            self2.next.retain_mut(|l| {
                let Some(l) = l.upgrade() else {
                    return false;
                };
                if pc.1.processed_links.contains(&l.id) {
                    return true;
                }
                if l.dec_dep_pending() <= 0 {
                    pc.1.queued_links.push(Rc::downgrade(&l));
                }
                return true;
            });
        }
        return out;
    }

    /// Modify the value; triggers processing.
    pub fn splice(
        &self,
        pc: &mut ProcessingContext,
        offset: usize,
        remove: usize,
        add: std::vec::Vec<T>,
    ) -> std::vec::Vec<T> {
        let mut self2 = self.0.mut_.borrow_mut();
        return self.splice_(&mut self2, pc, offset, remove, add);
    }

    /// Add one element; triggers processing.
    pub fn push(&self, pc: &mut ProcessingContext, value: T) {
        let mut self2 = self.0.mut_.borrow_mut();
        let len = self2.value.len();
        self.splice_(&mut self2, pc, len, 0, vec![value]);
    }

    /// Remove one element, return the element or None if the list was empty; triggers
    /// processing.
    pub fn pop(&self, pc: &mut ProcessingContext) -> Option<T> {
        let mut self2 = self.0.mut_.borrow_mut();
        let len = self2.value.len();
        if len == 0 {
            return None;
        }
        return self.splice_(&mut self2, pc, len - 1, 1, vec![]).into_iter().next();
    }

    /// Add multiple elements; triggers processing.
    pub fn extend(&self, pc: &mut ProcessingContext, values: std::vec::Vec<T>) {
        let mut self2 = self.0.mut_.borrow_mut();
        let len = self2.value.len();
        self.splice_(&mut self2, pc, len, 0, values);
    }

    /// Clears the collection; triggers processing.
    pub fn clear(&self, pc: &mut ProcessingContext) {
        let mut self2 = self.0.mut_.borrow_mut();
        let len = self2.value.len();
        self.splice_(&mut self2, pc, 0, len, vec![]);
    }

    /// Reduce the length of the collection to len, if longer.  Triggers processing.
    pub fn truncate(&self, pc: &mut ProcessingContext, len: usize) {
        let mut self2 = self.0.mut_.borrow_mut();
        let current_len = self2.value.len();
        if current_len > len {
            self.splice_(&mut self2, pc, len, current_len - len, vec![]);
        }
    }

    /// The current state of this vec.  A `Deref` wrapper around the internal `Vec`. If
    /// you want to iterate them, you'll need to call `.iter()` explicitly due to deref
    /// limitations.
    pub fn values<'a>(&'a self) -> ValuesRef<'a, T> {
        return ValuesRef(self.0.mut_.borrow());
    }

    /// Any changes during the current event handling that occurred to value to get it
    /// to its current state.  You can use them as splice inputs for a second list to
    /// synchronize them.  A `Deref` wrapper around an internal `Vec`.  If you want to
    /// iterate them, you'll need to call `.iter()` explicitly due to deref limitations.
    pub fn changes<'a>(&'a self) -> ChangesRef<'a, T> {
        return ChangesRef(self.0.mut_.borrow());
    }
}

impl<T: Clone + 'static> UpgradeValue for List<T> {
    fn upgrade_as_value(&self) -> Option<Value> {
        return Some(Value(self.0.clone()));
    }
}

impl<T: Clone + 'static> WeakList<T> {
    pub fn upgrade(&self) -> Option<List<T>> {
        return Some(List(self.0.upgrade()?));
    }

    pub fn id(&self) -> Id {
        return self.upgrade().unwrap().id();
    }
}

impl<T: Clone + 'static> UpgradeValue for WeakList<T> {
    fn upgrade_as_value(&self) -> Option<Value> {
        return self.0.upgrade().map(|x| Value(x));
    }
}

pub struct ValuesRef<'a, T: Clone + 'static>(Ref<'a, ListMut_<T>>);

impl<'a, T: Clone + 'static> Deref for ValuesRef<'a, T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        return &self.0.value;
    }
}

pub struct ChangesRef<'a, T: Clone + 'static>(Ref<'a, ListMut_<T>>);

impl<'a, T: Clone + 'static> Deref for ChangesRef<'a, T> {
    type Target = Vec<Change<T>>;

    fn deref(&self) -> &Self::Target {
        return &self.0.changes;
    }
}
