use std::{
    rc::{
        Rc,
    },
    cell::RefCell,
};
use crate::core::{
    Id,
    WeakLink,
    _ValueTrait,
    EventProcessingContext,
};

pub struct Change<T: Clone> {
    pub offset: usize,
    pub remove: usize,
    pub add: std::vec::Vec<T>,
}

pub struct Vec<T: Clone> {
    id: Id,
    value: std::vec::Vec<T>,
    changes: std::vec::Vec<Change<T>>,
    next: std::vec::Vec<WeakLink>,
}

impl<T: Clone> Vec<T> {
    pub fn splice(
        &mut self,
        ctx: &mut EventProcessingContext,
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
            ctx.1.processed.insert(self.id, Box::new({
                let s = self.clone();
                move || s.clean()
            }));
            self.next.retain_mut(|d| {
                let Some(d) = d.upgrade() else {
                    return false;
                };
                let id = d.borrow().id();
                if ctx.1.processed.contains_key(&id) {
                    return true;
                }
                let mut d_mut = d.as_ref().borrow_mut();
                if d_mut.dec_prev_pending() <= 0 {
                    ctx.1.queue.push(Rc::downgrade(&d));
                }
                return true;
            });
        }
        return out;
    }

    pub fn clear(&mut self, ctx: &mut EventProcessingContext) {
        let len = self.value.len();
        self.splice(ctx, 0, len, vec![]);
    }

    /// The current state of this vec.
    pub fn value(&self) -> &std::vec::Vec<T> {
        return &self.value;
    }

    /// Any changes during the current event handling that occurred to value to get it
    /// to its current state.
    pub fn changes(&self) -> &std::vec::Vec<Change<T>> {
        return &self.changes;
    }
}

impl<T: Clone> _ValueTrait for Vec<T> {
    fn id(&self) -> Id {
        return self.id;
    }

    fn add_next(&mut self, link: WeakLink) {
        self.next.push(link);
    }

    fn clean(&mut self) {
        self.changes.clear();
    }
}

pub fn new_vec<T: Clone>(ctx: &mut EventProcessingContext, initial: std::vec::Vec<T>) -> Rc<RefCell<Vec<T>>> {
    return Rc::new(RefCell::new(Vec {
        id: ctx.1.take_id(),
        value: initial,
        changes: vec![],
        next: vec![],
    }));
}
