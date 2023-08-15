use std::{
    mem::swap,
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

pub struct _Prim<T: PartialEq + Clone> {
    id: Id,
    value: T,
    previous_value: Option<T>,
    next: Vec<WeakLink>,
}

impl<T: PartialEq + Clone> _IntValueTrait for _Prim<T> {
    fn id(&self) -> Id {
        return self.id;
    }

    fn add_next(&mut self, link: WeakLink) {
        self.next.push(link);
    }
}

impl<T: PartialEq + Clone + 'static> _Prim<T> {
    pub fn get(&self) -> &T {
        return &self.value;
    }
}

#[derive(Clone)]
pub struct Prim<T: PartialEq + Clone>(pub(crate) Rc<RefCell<_Prim<T>>>);

#[derive(Clone)]
pub struct WeakPrim<T: PartialEq + Clone>(Weak<RefCell<_Prim<T>>>);

impl<T: PartialEq + Clone + 'static> Prim<T> {
    pub fn new(pc: &mut ProcessingContext, initial: T) -> Self {
        return Prim(Rc::new(RefCell::new(_Prim {
            id: pc.1.take_id(),
            value: initial,
            previous_value: None,
            next: vec![],
        })));
    }

    pub fn weak(&self) -> WeakPrim<T> {
        return WeakPrim(Rc::downgrade(&self.0));
    }

    /// Modify the value and mark downstream links as needing to be rerun.
    pub fn set(&self, pc: &mut ProcessingContext, mut value: T) {
        let mut self2 = self.0.as_ref().borrow_mut();
        if self2.value == value {
            return;
        }
        swap(&mut self2.value, &mut value);
        let first_change = self2.previous_value.is_none();
        self2.previous_value = Some(value);
        if first_change {
            pc.1.processed.insert(self2.id, Box::new({
                let s = self.clone();
                move || s.clean()
            }));
            self2.next.retain_mut(|d| {
                let Some(d) = d.upgrade() else {
                    return false;
                };
                let id = d.borrow().id();
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
    }

    /// Immutable access to the data.
    pub fn borrow<'a>(&'a self) -> Ref<'a, _Prim<T>> {
        return self.0.as_ref().borrow();
    }
}

impl<T: PartialEq + Clone + 'static> _ExtValueTrait for Prim<T> {
    fn id(&self) -> Id {
        return self.0.borrow().id;
    }

    fn clean(&self) {
        self.0.borrow_mut().previous_value = None;
    }
}

impl<T: PartialEq + Clone + 'static> UpgradeValue for Prim<T> {
    fn upgrade_as_value(&self) -> Option<Value> {
        return Some(self.0.clone() as Value);
    }
}

impl<T: PartialEq + Clone + 'static> WeakPrim<T> {
    pub fn upgrade(&self) -> Option<Prim<T>> {
        return Some(Prim(self.0.upgrade()?));
    }
}

impl<T: PartialEq + Clone + 'static> UpgradeValue for WeakPrim<T> {
    fn upgrade_as_value(&self) -> Option<Value> {
        return self.0.upgrade().map(|x| x as Value);
    }
}
