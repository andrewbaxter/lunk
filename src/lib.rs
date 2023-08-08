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
                        & $outputtype,
                        $(&[< _ $vname: upper >],) * $(&[< _ $name: upper >],) *
                    ) -> Option <() >,
                }
                // #
                //
                // IMPL
                impl < 
                //. x
                $([< _ $vname: upper >]: Clone + $crate:: core:: UpgradeValue,) * 
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
                    fn call(&self, ctx:& mut $crate:: core:: EventProcessingContext, output:& $outputtype) {
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
            UpgradeValue,
        },
        prim::{
            WeakPrim,
        },
    };

    let ec = EventContext::new();
    let (_a, b, _link) = ec.event(|ctx| {
        let a = new_prim(ctx, 0);
        let b = new_prim(ctx, 0);

        struct LinkAB {
            a: WeakPrim<i32>,
        }

        impl LinkCb<Prim<i32>> for LinkAB {
            fn call(&self, ctx: &mut EventProcessingContext, value: &Prim<i32>) {
                let Some(a) = self.a.upgrade() else {
                    return;
                };
                value.set(ctx, a.borrow().get() + 5);
            }

            fn inputs(&self) -> std::vec::Vec<Value> {
                return [self.a.clone()].into_iter().filter_map(|x| x.upgrade_as_value()).collect();
            }
        }

        let _link = new_link(ctx, b.clone(), LinkAB { a: a.weak() });
        a.set(ctx, 46);
        return (a, b, _link);
    });
    assert_eq!(*b.borrow().get(), 51);
}
