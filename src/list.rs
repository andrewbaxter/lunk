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
use crate::{
    core::{
        ValueTrait,
        ProcessingContext,
        Cleanup,
        Link_,
        IntoValue,
        Value,
    },
    Link,
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

impl<T: Clone> ListMut_<T> {
    fn next(&mut self) -> Vec<crate::Link> {
        let mut out = vec![];
        out.reserve(self.next.len());
        self.next.retain_mut(|e| {
            match e.upgrade() {
                Some(e) => {
                    out.push(Link(e.clone()));
                    return true;
                },
                None => {
                    return false;
                },
            }
        });
        return out;
    }
}

struct List_<T: Clone> {
    mut_: RefCell<ListMut_<T>>,
}

impl<T: Clone> ValueTrait for List_<T> {
    fn next_links(&self) -> Vec<crate::Link> {
        return self.mut_.borrow_mut().next();
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
    pub fn new(initial: std::vec::Vec<T>) -> Self {
        return List(Rc::new(List_ { mut_: RefCell::new(ListMut_ {
            value: initial,
            changes: vec![],
            next: vec![],
        }) }));
    }

    /// Used internally by the `link!` macro to establish graph edges between an input
    /// value and the link.
    pub fn add_next(&self, link: &Link) {
        self.0.mut_.borrow_mut().next.push(Rc::downgrade(&link.0));
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
            if !pc.1.processing {
                for l in self2.next() {
                    pc.1.step1_stacked_links.push((true, l));
                }
            }
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
    ///
    /// Borrows the list, must be released before calling other mutate methods.
    pub fn borrow_values<'a>(&'a self) -> ValuesRef<'a, T> {
        return ValuesRef(self.0.mut_.borrow());
    }

    /// Any changes during the current event handling that occurred to value to get it
    /// to its current state.  You can use them as splice inputs for a second list to
    /// synchronize them.  A `Deref` wrapper around an internal `Vec`.  If you want to
    /// iterate them, you'll need to call `.iter()` explicitly due to deref limitations.
    ///
    /// Borrows the list, must be released before calling other mutate methods.
    pub fn borrow_changes<'a>(&'a self) -> ChangesRef<'a, T> {
        return ChangesRef(self.0.mut_.borrow());
    }
}

impl<T: Clone + 'static> IntoValue for List<T> {
    fn into_value(&self) -> Value {
        return Value(self.0.clone());
    }
}

impl<T: Clone + 'static> WeakList<T> {
    pub fn upgrade(&self) -> Option<List<T>> {
        return Some(List(self.0.upgrade()?));
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
