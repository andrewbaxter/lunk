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

pub struct PrimMut_<T: PartialEq + Clone> {
    value: T,
    previous_value: Option<T>,
    next: Vec<Weak<Link_>>,
}

impl<T: PartialEq + Clone + 'static> PrimMut_<T> {
    pub fn get(&self) -> &T {
        return &self.value;
    }
}

pub(crate) struct Prim_<T: PartialEq + Clone> {
    pub(crate) id: Id,
    mut_: RefCell<PrimMut_<T>>,
}

impl<T: PartialEq + Clone> ValueTrait for Prim_<T> {
    fn id(&self) -> Id {
        return self.id;
    }

    fn add_next(&self, link: Weak<Link_>) {
        self.mut_.borrow_mut().next.push(link);
    }
}

impl<T: PartialEq + Clone> Cleanup for Prim_<T> {
    fn clean(&self) {
        self.mut_.borrow_mut().previous_value = None;
    }
}

/// This represents a non-collection value, like an int, bool, or struct.
#[derive(Clone)]
pub struct Prim<T: PartialEq + Clone>(pub(crate) Rc<Prim_<T>>);

#[derive(Clone)]
pub struct WeakPrim<T: PartialEq + Clone>(Weak<Prim_<T>>);

impl<T: PartialEq + Clone + 'static> Prim<T> {
    pub fn new(pc: &mut ProcessingContext, initial: T) -> Self {
        let id = pc.1.take_id();
        return Prim(Rc::new(Prim_ {
            id: id,
            mut_: RefCell::new(PrimMut_ {
                value: initial,
                previous_value: None,
                next: vec![],
            }),
        }));
    }

    pub fn id(&self) -> Id {
        return self.0.id;
    }

    /// Get a weak reference to the list.
    pub fn weak(&self) -> WeakPrim<T> {
        return WeakPrim(Rc::downgrade(&self.0));
    }

    /// Modify the value and mark downstream links as needing to be rerun.
    pub fn set(&self, pc: &mut ProcessingContext, mut value: T) {
        let mut self2 = self.0.mut_.borrow_mut();
        if self2.value == value {
            return;
        }
        swap(&mut self2.value, &mut value);
        let first_change = self2.previous_value.is_none();
        self2.previous_value = Some(value);
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
    }

    /// Immutable access to the data via a `Deref` wrapper.
    pub fn borrow<'a>(&'a self) -> ValueRef<'a, T> {
        return ValueRef(self.0.mut_.borrow());
    }

    /// Get the internal value.  This copies/clones the value.
    pub fn get(&self) -> T {
        return self.0.mut_.borrow().value.clone();
    }
}

impl<T: PartialEq + Clone + 'static> UpgradeValue for Prim<T> {
    fn upgrade_as_value(&self) -> Option<Value> {
        return Some(Value(self.0.clone()));
    }
}

impl<T: PartialEq + Clone + 'static> WeakPrim<T> {
    pub fn upgrade(&self) -> Option<Prim<T>> {
        return Some(Prim(self.0.upgrade()?));
    }

    pub fn id(&self) -> Id {
        return self.upgrade().unwrap().id();
    }
}

impl<T: PartialEq + Clone + 'static> UpgradeValue for WeakPrim<T> {
    fn upgrade_as_value(&self) -> Option<Value> {
        return self.0.upgrade().map(|x| Value(x));
    }
}

pub struct ValueRef<'a, T: PartialEq + Clone + 'static>(Ref<'a, PrimMut_<T>>);

impl<'a, T: PartialEq + Clone + 'static> Deref for ValueRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        return &self.0.value;
    }
}
