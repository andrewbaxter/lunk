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
use crate::{
    core::{
        Id,
        ValueTrait,
        ProcessingContext,
        Cleanup,
        Link_,
        Value,
        IntoValue,
    },
    Link,
};

pub struct PrimMut_<T> {
    value: T,
    first_change: bool,
    next: Vec<Weak<Link_>>,
}

impl<T: 'static> PrimMut_<T> {
    pub fn get(&self) -> &T {
        return &self.value;
    }
}

pub(crate) struct Prim_<T> {
    pub(crate) id: Id,
    mut_: RefCell<PrimMut_<T>>,
}

impl<T> ValueTrait for Prim_<T> {
    fn id(&self) -> Id {
        return self.id;
    }

    fn next(&self) -> Vec<crate::Link> {
        let mut out = vec![];
        let mut self2 = self.mut_.borrow_mut();
        out.reserve(self2.next.len());
        self2.next.retain_mut(|e| {
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

impl<T> Cleanup for Prim_<T> {
    fn clean(&self) {
        self.mut_.borrow_mut().first_change = true;
    }
}

/// This represents a non-collection value, like an int, bool, or struct.
#[derive(Clone)]
pub struct Prim<T>(pub(crate) Rc<Prim_<T>>);

#[derive(Clone)]
pub struct WeakPrim<T>(Weak<Prim_<T>>);

impl<T: 'static> Prim<T> {
    pub fn new(pc: &mut ProcessingContext, initial: T) -> Self {
        let id = pc.1.take_id();
        return Prim(Rc::new(Prim_ {
            id: id,
            mut_: RefCell::new(PrimMut_ {
                value: initial,
                first_change: true,
                next: vec![],
            }),
        }));
    }

    pub fn id(&self) -> Id {
        return self.0.id;
    }

    /// Used internally by the `link!` macro to establish graph edges between an input
    /// value and the link.
    pub fn add_next(&self, link: &Link) {
        self.0.mut_.borrow_mut().next.push(Rc::downgrade(&link.0));
    }

    /// Get a weak reference to the value.
    pub fn weak(&self) -> WeakPrim<T> {
        return WeakPrim(Rc::downgrade(&self.0));
    }

    /// Modify the value and mark downstream links as needing to be rerun.
    pub fn set(&self, pc: &mut ProcessingContext, value: T) {
        let first_change;
        {
            let mut self2 = self.0.mut_.borrow_mut();
            first_change = self2.first_change;
            self2.first_change = false;
            self2.value = value;
        }
        if first_change {
            pc.1.cleanup.push(self.0.clone());
            if !pc.1.processing {
                for l in self.0.next() {
                    pc.1.roots.insert(l.0.id, l.clone());
                }
            }
        }
    }

    /// Immutable access to the data via a `Deref` wrapper.
    pub fn borrow<'a>(&'a self) -> ValueRef<'a, T> {
        return ValueRef(self.0.mut_.borrow());
    }
}

impl<T: 'static> IntoValue for Prim<T> {
    fn into_value(&self) -> Value {
        return Value(self.0.clone());
    }
}

impl<T: 'static> WeakPrim<T> {
    pub fn upgrade(&self) -> Option<Prim<T>> {
        return Some(Prim(self.0.upgrade()?));
    }

    pub fn id(&self) -> Id {
        return self.upgrade().unwrap().id();
    }
}

pub struct ValueRef<'a, T: 'static>(Ref<'a, PrimMut_<T>>);

impl<'a, T: 'static> Deref for ValueRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        return &self.0.value;
    }
}

pub struct HistPrimMut_<T: PartialEq + Clone> {
    value: T,
    previous_value: Option<T>,
    next: Vec<Weak<Link_>>,
}

impl<T: PartialEq + Clone + 'static> HistPrimMut_<T> {
    pub fn get(&self) -> &T {
        return &self.value;
    }
}

pub(crate) struct HistPrim_<T: PartialEq + Clone> {
    pub(crate) id: Id,
    mut_: RefCell<HistPrimMut_<T>>,
}

impl<T: PartialEq + Clone> ValueTrait for HistPrim_<T> {
    fn id(&self) -> Id {
        return self.id;
    }

    fn next(&self) -> Vec<crate::Link> {
        let mut out = vec![];
        let mut self2 = self.mut_.borrow_mut();
        out.reserve(self2.next.len());
        self2.next.retain_mut(|e| {
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

impl<T: PartialEq + Clone> Cleanup for HistPrim_<T> {
    fn clean(&self) {
        self.mut_.borrow_mut().previous_value = None;
    }
}

/// Like Prim represents a non-collection value, like an int, bool, or struct. It
/// keeps the old value around, and only triggers updates if the value is changed.
/// The old value is available for links to query if they need to refer to the old
/// value for some reason.  The value must implement `PartialEq` and `Clone`.
#[derive(Clone)]
pub struct HistPrim<T: PartialEq + Clone>(pub(crate) Rc<HistPrim_<T>>);

#[derive(Clone)]
pub struct WeakHistPrim<T: PartialEq + Clone>(Weak<HistPrim_<T>>);

impl<T: PartialEq + Clone + 'static> HistPrim<T> {
    pub fn new(pc: &mut ProcessingContext, initial: T) -> Self {
        let id = pc.1.take_id();
        return HistPrim(Rc::new(HistPrim_ {
            id: id,
            mut_: RefCell::new(HistPrimMut_ {
                value: initial,
                previous_value: None,
                next: vec![],
            }),
        }));
    }

    pub fn id(&self) -> Id {
        return self.0.id;
    }

    /// Used internally by the `link!` macro to establish graph edges between an input
    /// value and the link.
    pub fn add_next(&self, link: &Link) {
        self.0.mut_.borrow_mut().next.push(Rc::downgrade(&link.0));
    }

    /// Get a weak reference to the value.
    pub fn weak(&self) -> WeakHistPrim<T> {
        return WeakHistPrim(Rc::downgrade(&self.0));
    }

    /// Modify the value and mark downstream links as needing to be rerun.
    pub fn set(&self, pc: &mut ProcessingContext, mut value: T) {
        let first_change;
        {
            let mut self2 = self.0.mut_.borrow_mut();
            if self2.value == value {
                return;
            }
            swap(&mut self2.value, &mut value);
            first_change = self2.previous_value.is_none();
            self2.previous_value = Some(value);
        }
        if first_change {
            pc.1.cleanup.push(self.0.clone());
            if !pc.1.processing {
                for l in self.0.next() {
                    pc.1.roots.insert(l.0.id, l.clone());
                }
            }
        }
    }

    /// Immutable access to the data via a `Deref` wrapper.
    pub fn borrow<'a>(&'a self) -> HistValueRef<'a, T> {
        return HistValueRef(self.0.mut_.borrow());
    }

    /// Get the internal value.  This copies/clones the value.
    pub fn get(&self) -> T {
        return self.0.mut_.borrow().value.clone();
    }

    /// Get the previous version of the value.  This copies/clones the value.
    pub fn get_old(&self) -> T {
        let m = self.0.mut_.borrow();
        return m.previous_value.as_ref().unwrap_or_else(|| &m.value).clone();
    }
}

impl<T: PartialEq + Clone + 'static> IntoValue for HistPrim<T> {
    fn into_value(&self) -> Value {
        return Value(self.0.clone());
    }
}

impl<T: PartialEq + Clone + 'static> WeakHistPrim<T> {
    pub fn upgrade(&self) -> Option<HistPrim<T>> {
        return Some(HistPrim(self.0.upgrade()?));
    }

    pub fn id(&self) -> Id {
        return self.upgrade().unwrap().id();
    }
}

pub struct HistValueRef<'a, T: PartialEq + Clone + 'static>(Ref<'a, HistPrimMut_<T>>);

impl<'a, T: PartialEq + Clone + 'static> Deref for HistValueRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        return &self.0.value;
    }
}
