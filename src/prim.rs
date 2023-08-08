use std::{
    mem::swap,
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

pub struct Prim<T: PartialEq + Clone> {
    id: Id,
    value: T,
    previous_value: Option<T>,
    next: Vec<WeakLink>,
}

impl<T: PartialEq + Clone> _ValueTrait for Prim<T> {
    fn id(&self) -> Id {
        return self.id;
    }

    fn add_next(&mut self, link: WeakLink) {
        self.next.push(link);
    }

    fn clean(&mut self) {
        self.previous_value = None;
    }
}

impl<T: PartialEq + Clone> Prim<T> {
    pub fn get(&self) -> &T {
        return &self.value;
    }

    pub fn set(&mut self, ctx: &mut EventProcessingContext, mut value: T) {
        if self.value == value {
            return;
        }
        swap(&mut self.value, &mut value);
        let first_change = self.previous_value.is_none();
        self.previous_value = Some(value);
        if first_change {
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
    }
}

pub fn new_prim<T: PartialEq + Clone>(ctx: &mut EventProcessingContext, initial: T) -> Rc<RefCell<Prim<T>>> {
    return Rc::new(RefCell::new(Prim {
        id: ctx.1.take_id(),
        value: initial,
        previous_value: None,
        next: vec![],
    }));
}
