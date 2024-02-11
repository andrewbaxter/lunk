use std::{
    ops::{
        Add,
        Mul,
        Sub,
        AddAssign,
        Div,
    },
    collections::HashMap,
    rc::Rc,
    cell::RefCell,
};
use crate::{
    EventGraph,
    core::{
        Id,
        NULL_ID,
    },
    prim::WeakHistPrim,
    ProcessingContext,
    HistPrim,
};

/// Working around rust infantilism.
pub trait EaseUnit {
    fn to_ease_unit(v: f64) -> Self;
}

impl EaseUnit for f32 {
    fn to_ease_unit(v: f64) -> Self {
        return v as Self;
    }
}

impl EaseUnit for f64 {
    fn to_ease_unit(v: f64) -> Self {
        return v;
    }
}

/// Implement this to create custom antimations, including animations that run
/// forever.
pub trait HistPrimAnimation {
    /// After doing whatever calculations, call `set` on the primitive being animated
    /// like normal.  Return `true` until the animation is done.
    fn update(&mut self, pc: &mut ProcessingContext, delta_ms: f64) -> bool;

    /// The id of the primitive being animated. This is isued to replace existing
    /// animations for the primitive.
    fn id(&self) -> Id;
}

/// A simple animation easing a primitive to a new value. See `HistPrimEaseExt`
/// which adds a method to `HistPrim` to start easings.
pub struct HistPrimEaseAnimation<
    S: Copy + EaseUnit + PartialOrd + AddAssign + Div<Output = S>,
    T: PartialEq + Clone + Add<T, Output = T> + Sub<T, Output = T> + Mul<S, Output = T> + 'static,
> {
    start: T,
    range: T,
    /// Milliseconds
    duration: S,
    /// 0..1, with 1 reached after duration
    at: S,
    f: fn(S) -> S,
    value: WeakHistPrim<T>,
}

impl<
    S: Copy + EaseUnit + PartialOrd + AddAssign + Div<Output = S>,
    T: PartialEq + Clone + Add<T, Output = T> + Sub<T, Output = T> + Mul<S, Output = T> + 'static,
> HistPrimEaseAnimation<S, T> {
    fn new(prim: &HistPrim<T>, end: T, duration: S, f: fn(S) -> S) -> HistPrimEaseAnimation<S, T> {
        let start = prim.get();
        let range = end - start.clone();
        return HistPrimEaseAnimation {
            start: start,
            range: range,
            duration: duration,
            f: f,
            at: S::to_ease_unit(0f64),
            value: prim.weak(),
        };
    }
}

impl<
    S: Copy + EaseUnit + PartialOrd + AddAssign + Div<Output = S>,
    T: PartialEq + Clone + Add<T, Output = T> + Sub<T, Output = T> + Mul<S, Output = T> + 'static,
> HistPrimAnimation for HistPrimEaseAnimation<S, T> {
    fn update(&mut self, pc: &mut ProcessingContext, delta: f64) -> bool {
        let Some(value) = self.value.upgrade() else {
            return false;
        };
        self.at += S::to_ease_unit(delta) / self.duration;
        if self.at >= S::to_ease_unit(1f64) {
            value.set(pc, self.start.clone() + self.range.clone());
            return false;
        }
        value.set(pc, self.start.clone() + self.range.clone() * (self.f)(self.at));
        return true;
    }

    fn id(&self) -> Id {
        return self.value.upgrade().map(|v| v.0.id).unwrap_or(NULL_ID);
    }
}

/// Adds the method `set_ease` for animation to `HistPrim` to parallel `set`.
/// `duration` is in milliseconds. `f` is a function that takes an input of `0..1`
/// representing linear progress of the easing and returns another `0..1`
/// representing the eased visual progress, as the methods in `ezing`.
pub trait HistPrimEaseExt<
    S,
    T: PartialEq + Clone + Add<T, Output = T> + Sub<T, Output = T> + Mul<S, Output = T> + 'static,
> {
    fn set_ease(&self, a: &Animator, end: T, duration: S, f: fn(S) -> S);
}

impl<
    T: PartialEq + Clone + Add<T, Output = T> + Sub<T, Output = T> + Mul<f32, Output = T> + 'static,
> HistPrimEaseExt<f32, T> for HistPrim<T> {
    fn set_ease(&self, a: &Animator, end: T, duration: f32, f: fn(f32) -> f32) {
        a.start(HistPrimEaseAnimation::new(self, end, duration, f));
    }
}

impl<
    T: PartialEq + Clone + Add<T, Output = T> + Sub<T, Output = T> + Mul<f64, Output = T> + 'static,
> HistPrimEaseExt<f64, T> for HistPrim<T> {
    fn set_ease(&self, a: &Animator, end: T, duration: f64, f: fn(f64) -> f64) {
        a.start(HistPrimEaseAnimation::new(self, end, duration, f));
    }
}

/// Manages animations. After creating, start some animations then call `update`
/// regularly.  `trigger_cb` is a callback that's called whenever a new animation
/// is started, which can be used to start real-time updates or whatever.
pub struct Animator_ {
    interp: HashMap<Id, Box<dyn HistPrimAnimation>>,
    interp_backbuf: Option<HashMap<Id, Box<dyn HistPrimAnimation>>>,
    anim_cb: Option<Box<dyn FnMut() -> ()>>,
}

#[derive(Clone)]
pub struct Animator(Rc<RefCell<Animator_>>);

impl Animator {
    pub fn new() -> Animator {
        return Animator(Rc::new(RefCell::new(Animator_ {
            interp: Default::default(),
            interp_backbuf: Some(Default::default()),
            anim_cb: None,
        })));
    }

    pub fn set_start_cb(&self, trigger_cb: impl FnMut() -> () + 'static) {
        self.0.borrow_mut().anim_cb = Some(Box::new(trigger_cb));
    }

    /// Start a new animation for the primitive, replacing any existing animation.
    pub fn start(&self, animation: impl HistPrimAnimation + 'static) {
        self.0.borrow_mut().interp.insert(animation.id(), Box::new(animation));
        if let Some(cb) = &mut self.0.borrow_mut().anim_cb {
            cb();
        }
    }

    /// Stop easing a primitive. If the primitive isn't being smoothed this does
    /// nothing. The primitive will retain the current value.
    pub fn cancel<T: PartialEq + Clone + 'static>(&self, prim: &HistPrim<T>) {
        self.0.borrow_mut().interp.remove(&prim.0.id);
    }

    /// Stop all current easings.
    pub fn clear(&self) {
        self.0.borrow_mut().interp.clear();
    }

    /// Updates interpolating nodes and processes the graph as usual. Call from
    /// `requestAnimationFrame` for example, in a WASM context. Returns true as long as
    /// there are animations to continue.
    pub fn update(&self, eg: &EventGraph, delta_ms: f64) -> bool {
        return eg.event(|pc| {
            let mut interp = {
                let mut self2 = self.0.borrow_mut();
                let mut interp = self2.interp_backbuf.take().unwrap();
                std::mem::swap(&mut self2.interp, &mut interp);
                interp
            };
            let mut alive = false;
            for (id, mut l) in interp.drain() {
                if l.update(pc, delta_ms) {
                    alive = true;
                    self.0.borrow_mut().interp.insert(id, l);
                }
            }
            self.0.borrow_mut().interp_backbuf = Some(interp);
            return alive;
        });
    }
}
