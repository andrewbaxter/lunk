pub mod core;
pub mod prim;
pub mod vec;

pub use crate::core::{
    new_link,
    LinkCb,
    EventContext,
    EventProcessingContext,
};
pub use prim::{
    Prim,
    new_prim,
};
pub use vec::{
    Vec,
    new_vec,
};
pub use paste;

/// Establish a link that uses the values of a number of link (and non-link)
/// values, performs a computation on those values, and updates a primitive value
/// with the result.  This returns a handle to manage the lifetime of the link - as
/// long as you want this link in the graph to be activated during events, prevent
/// this value from dropping.
///
/// You can do this by implementing `LinkCb` on a struct, or by using this nasty
/// hack of a macro to theoretically replace some benign boilerplate with a
/// powderkeg of generics, anonymous types, and macro fragility.
///
/// The usage is as follows:
///
/// ```
/// let _link = link!((ctx1 = ctx2, output: DTYPE = output1; a=a1, ...; b=b1, ...) {
///     let a = a.upgrade()?;
///     let b = b.upgrade()?;
///     output.set(a.get() + b.get());
/// })
/// ```
///
/// * `ctx1` is the name of the context binding in the context of the link callback,
///   and `ctx2` is the `EventProcessingContext` in the current scope used for setting
///   up the link. This is provided within an invocation of `EventContext`'s `event()`.
///   The `=` here is inappropriate, `ctx1` and `ctx2` are separate values, I just used
///   it to be consistent with the rest of the macro.
///
/// * `output` is the name of the outputination value (for modification) in the context
///   of the link callback.  It must have a type of `DTYPE`. `output1` is the value in
///   the local scope passed in when setting up the link.  For example,
///   `output = Prim<i32> counter`.
///
/// * `a` is the name of the link value caputred from the expression `a1`
///
/// * `b` is the name of the non-link (any extra state that needs to be captured, like
///   database handles, etc) captured from the expression `b1`
///
/// * body -- all the named captures above are available, with the link values being
///   _weak_ references. The code should return a `Some(value)` with the result of the
///   calculation.  If a `None` is returned, the value will not be updated and
///   downstream links will not be triggered here.
#[macro_export]
macro_rules! link{
    (
        (
            $ctxname: ident = $ctxval: expr,
            $outputname: ident: $outputtype: ty = $outputval: expr;
            $($vname: ident = $vval: expr),
            * $(,) ?;
            $($name: ident = $val: expr),
            * $(,) ? $(;) ?
        ) $body: block
    ) => {
        $crate:: paste:: paste ! {
            {
                // #
                //
                // DEF
                struct _Link < 
                //. x
                $([< _ $vname: upper >],) * 
                //. x
                $([< _ $name: upper >],) *
                //. _
                > {
                    //. x
                    $([< _ $vname >]:[< _ $vname: upper >],) * 
                    //. x
                    $([< _ $name >]:[< _ $name: upper >],) * 
                    //. x
                    f: fn(
                        & mut $crate:: core:: EventProcessingContext,
                        & std:: rc:: Rc < std:: cell:: RefCell < $outputtype >>,
                        $(&[< _ $vname: upper >],) * $(&[< _ $name: upper >],) *
                    ) -> Option <() >,
                }
                // #
                //
                // IMPL
                impl < 
                //. x
                $([< _ $vname: upper >]: Clone + $crate:: core:: _UpgradeValue,) * 
                //. x
                $([< _ $name: upper >],) *
                //. _
                > $crate:: core:: LinkCb < $outputtype > for _Link < 
                //. x
                $([< _ $vname: upper >],) * 
                //. x
                $([< _ $name: upper >],) *
                //. _
                > {
                    fn call(
                        &self,
                        ctx:& mut $crate:: core:: EventProcessingContext,
                        output:& std:: rc:: Rc < std:: cell:: RefCell < $outputtype >>
                    ) {
                        (self.f)(ctx, output, $(& self.[< _ $vname >],) * $(& self.[< _ $name >],) *);
                    }
                    fn inputs(&self) -> std:: vec:: Vec < $crate:: core:: Value > {
                        return[
                            $(self.[< _ $vname >].clone(),) *
                        ].into_iter().filter_map(|x| x.upgrade_as_value()).collect();
                    }
                }
                // #
                //
                // INST
                $crate:: core:: new_link($ctxval, $outputval.clone(), _Link {
                    //. x
                    $([< _ $vname >]: $vval,) * 
                    //. x
                    $([< _ $name >]: $val,) * 
                    //. x
                    f:| $ctxname,
                    $outputname,
                    $($vname,) * $($name,) *|-> Option <() > {
                        $body;
                        return None;
                    }
                })
            }
        }
    };
    (
        (
            $ctxname: ident = $ctxval: expr;
            $($vname: ident = $vval: expr),
            * $(,) ?;
            $($name: ident = $val: expr),
            * $(,) ? $(;) ?
        ) $body: block
    ) => {
        {
            let ctx =& mut * $ctxval;
            let _out = $crate:: prim:: new_prim(ctx, ());
            $crate:: link !(
                ($ctxname = ctx, _out: $crate:: prim:: Prim <() >= _out; $($vname = $vval), *; $($name = $val), *) {
                    $body
                }
            )
        }
    }
}

#[test]
fn basic0() {
    use crate::{
        core::{
            Value,
        },
        prim::{
            WeakPrim,
        },
    };

    let mut ec = EventContext::new();
    let a = new_prim(&mut ec, 0);
    let b = new_prim(&mut ec, 0);
    let _d = ec.event(|ctx| {
        struct LinkAB {
            a: WeakPrim<i32>,
        }

        impl LinkCb<Prim<i32>> for LinkAB {
            fn call(&self, ctx: &mut EventProcessingContext, value: &Prim<i32>) {
                let Some(a) = self.a.upgrade() else {
                    return;
                };
                value.set(ctx, a.get() + 5);
            }

            fn inputs(&self) -> std::vec::Vec<Value> {
                return [self.a.clone()].into_iter().filter_map(|x| x.upgrade().map(|x| x.0 as Value)).collect();
            }
        }

        let _link = new_link(ctx, b.clone(), LinkAB { a: a.weak() });
        a.set(ctx, 46);
        return _link;
    });
    assert_eq!(b.get(), 51);
}
