use std::{
    ops::{
        Add,
        Mul,
        Sub,
    },
    collections::HashMap,
};
use crate::{
    EventGraph,
    core::{
        Id,
        _ExtValueTrait,
        NULL_ID,
    },
    prim::WeakPrim,
    ProcessingContext,
    Prim,
};

/// Implement this to create custom antimations, including animations that run
/// forever.
pub trait PrimAnimation {
    /// Update the animation state, and then call `set` on the primitive it's animating
    /// like normal.
    fn update(&mut self, pc: &mut ProcessingContext, delta_s: f32) -> bool;

    /// The id of the primitive being animated. This is isued to replace existing
    /// animations for the primitive.
    fn id(&self) -> Id;
}

/// A simple animation easing a primitive to a new value. See `PrimEaseExt` which
/// adds a method to `Prim` to start easings.
pub struct PrimEaseAnimation<
    T: PartialEq + Clone + Add<T, Output = T> + Sub<T, Output = T> + Mul<f32, Output = T> + 'static,
> {
    start: T,
    range: T,
    /// Seconds
    duration: f32,
    /// 0..1, with 1 reached after duration
    at: f32,
    f: fn(f32) -> f32,
    value: WeakPrim<T>,
}

impl<
    T: PartialEq + Clone + Add<T, Output = T> + Sub<T, Output = T> + Mul<f32, Output = T> + 'static,
> PrimEaseAnimation<T> {
    fn new(prim: &Prim<T>, end: T, duration: f32, f: fn(f32) -> f32) -> PrimEaseAnimation<T> {
        let start = prim.borrow().get().clone();
        let range = end - start.clone();
        return PrimEaseAnimation {
            start: start,
            range: range,
            duration: duration,
            f: f,
            at: 0.,
            value: prim.weak(),
        };
    }
}

impl<
    T: PartialEq + Clone + Add<T, Output = T> + Sub<T, Output = T> + Mul<f32, Output = T> + 'static,
> PrimAnimation for PrimEaseAnimation<T> {
    fn update(&mut self, pc: &mut ProcessingContext, delta: f32) -> bool {
        let Some(value) = self.value.upgrade() else {
            return false;
        };
        self.at += delta / self.duration;
        if self.at >= 1. {
            value.set(pc, self.start.clone() + self.range.clone());
            return false;
        }
        value.set(pc, self.start.clone() + self.range.clone() * (self.f)(self.at));
        return true;
    }

    fn id(&self) -> Id {
        return self.value.upgrade().map(|v| v.id()).unwrap_or(NULL_ID);
    }
}

/// Adds the method `set_ease` for animation to `Prim` to parallel `set`.
pub trait PrimEaseExt<
    T: PartialEq + Clone + Add<T, Output = T> + Sub<T, Output = T> + Mul<f32, Output = T> + 'static,
> {
    fn set_ease(&self, a: &mut Animator, end: T, duration: f32, f: fn(f32) -> f32);
}

impl<
    T: PartialEq + Clone + Add<T, Output = T> + Sub<T, Output = T> + Mul<f32, Output = T> + 'static,
> PrimEaseExt<T> for Prim<T> {
    fn set_ease(&self, a: &mut Animator, end: T, duration: f32, f: fn(f32) -> f32) {
        a.start(PrimEaseAnimation::new(self, end, duration, f));
    }
}

/// Manages animations. After creating, start some animations then call `update`
/// regularly.
pub struct Animator {
    interp: HashMap<Id, Box<dyn PrimAnimation>>,
    interp_backbuf: HashMap<Id, Box<dyn PrimAnimation>>,
}

impl Animator {
    /// Start a new animation for the primitive, replacing any existing animation.
    pub fn start(&mut self, animation: impl PrimAnimation + 'static) {
        self.interp.insert(animation.id(), Box::new(animation));
    }

    /// Stop smooth a primitive. If the primitive isn't being smoothed this does
    /// nothing. The primitive will retain the current value.
    pub fn cancel<T: PartialEq + Clone + 'static>(&mut self, prim: &Prim<T>) {
        self.interp.remove(&prim.id());
    }

    /// Stop all current easings.
    pub fn clear(&mut self) {
        self.interp.clear();
    }

    /// Updates interpolating nodes and processes the graph as usual. Call from
    /// `requestAnimationFrame`.
    pub fn update(&mut self, eg: EventGraph, delta_s: f32) {
        eg.event(|pc| {
            for (id, mut l) in self.interp.drain() {
                if l.update(pc, delta_s) {
                    self.interp_backbuf.insert(id, l);
                }
            }
            std::mem::swap(&mut self.interp, &mut self.interp_backbuf);
        });
    }
}
