use std::{
    rc::{
        Rc,
        Weak,
    },
    cell::{
        RefCell,
        Ref,
    },
};
use crate::core::{
    Id,
    WeakLink,
    _IntValueTrait,
    ProcessingContext,
    _ExtValueTrait,
    UpgradeValue,
    Value,
};

pub struct Change<T: Clone> {
    pub offset: usize,
    pub remove: usize,
    pub add: std::vec::Vec<T>,
}

pub struct _Vec<T: Clone> {
    id: Id,
    value: std::vec::Vec<T>,
    changes: std::vec::Vec<Change<T>>,
    next: std::vec::Vec<WeakLink>,
}

impl<T: Clone + 'static> _Vec<T> {
    /// The current state of this vec.
    pub fn value(&self) -> &std::vec::Vec<T> {
        return &self.value;
    }

    /// Any changes during the current event handling that occurred to value to get it
    /// to its current state.
    pub fn changes(&self) -> &std::vec::Vec<Change<T>> {
        return &self.changes;
    }

    fn splice(
        &mut self,
        self2: &Vec<T>,
        pc: &mut ProcessingContext,
        offset: usize,
        remove: usize,
        add: std::vec::Vec<T>,
    ) -> std::vec::Vec<T> {
        let first_change = self.changes.is_empty();
        let out = self.value.splice(offset .. offset + remove, add.clone()).collect();
        self.changes.push(Change {
            offset: offset,
            remove: remove,
            add: add,
        });
        if first_change {
            pc.1.processed.insert(self.id, Box::new({
                let s = self2.clone();
                move || s.clean()
            }));
            self.next.retain_mut(|d| {
                let Some(d) = d.upgrade() else {
                    return false;
                };
                let id = d.as_ref().borrow().id();
                if pc.1.processed.contains_key(&id) {
                    return true;
                }
                let mut d_mut = d.as_ref().borrow_mut();
                if d_mut.dec_prev_pending() <= 0 {
                    pc.1.queue.push(Rc::downgrade(&d));
                }
                return true;
            });
        }
        return out;
    }
}

impl<T: Clone> _IntValueTrait for _Vec<T> {
    fn id(&self) -> Id {
        return self.id;
    }

    fn add_next(&mut self, link: WeakLink) {
        self.next.push(link);
    }
}

#[derive(Clone)]
pub struct Vec<T: Clone>(Rc<RefCell<_Vec<T>>>);

#[derive(Clone)]
pub struct WeakVec<T: Clone>(Weak<RefCell<_Vec<T>>>);

impl<T: Clone + 'static> Vec<T> {
    pub fn new(pc: &mut ProcessingContext, initial: std::vec::Vec<T>) -> Self {
        return Vec(Rc::new(RefCell::new(_Vec {
            id: pc.1.take_id(),
            value: initial,
            changes: vec![],
            next: vec![],
        })));
    }

    pub fn weak(&self) -> WeakVec<T> {
        return WeakVec(Rc::downgrade(&self.0));
    }

    /// Modify the value and mark downstream links as needing to be rerun.
    pub fn splice(
        &self,
        pc: &mut ProcessingContext,
        offset: usize,
        remove: usize,
        add: std::vec::Vec<T>,
    ) -> std::vec::Vec<T> {
        return self.0.as_ref().borrow_mut().splice(&self, pc, offset, remove, add);
    }

    pub fn push(&self, pc: &mut ProcessingContext, value: T) {
        let mut self2 = self.0.as_ref().borrow_mut();
        let len = self2.value.len();
        self2.splice(&self, pc, len, 0, vec![value]);
    }

    pub fn extend(&self, pc: &mut ProcessingContext, values: std::vec::Vec<T>) {
        let mut self2 = self.0.as_ref().borrow_mut();
        let len = self2.value.len();
        self2.splice(&self, pc, len, 0, values);
    }

    /// Clears the collection, triggering updates.
    pub fn clear(&self, pc: &mut ProcessingContext) {
        let mut self2 = self.0.as_ref().borrow_mut();
        let len = self2.value.len();
        self2.splice(&self, pc, 0, len, vec![]);
    }

    /// Immutable access to the collection.
    pub fn borrow<'a>(&'a self) -> Ref<'a, _Vec<T>> {
        return self.0.as_ref().borrow();
    }
}

impl<T: Clone + 'static> _ExtValueTrait for Vec<T> {
    fn id(&self) -> Id {
        return self.0.borrow().id;
    }

    fn clean(&self) {
        self.0.borrow_mut().changes.clear();
    }
}

impl<T: Clone + 'static> UpgradeValue for Vec<T> {
    fn upgrade_as_value(&self) -> Option<Value> {
        return Some(self.0.clone() as Value);
    }
}

impl<T: Clone + 'static> WeakVec<T> {
    pub fn upgrade(&self) -> Option<Vec<T>> {
        return Some(Vec(self.0.upgrade()?));
    }
}

impl<T: Clone + 'static> UpgradeValue for WeakVec<T> {
    fn upgrade_as_value(&self) -> Option<Value> {
        return self.0.upgrade().map(|x| x as Value);
    }
}
