pub mod core;
pub mod prim;
pub mod vec;
pub mod animate;

pub use crate::core::{
    new_link,
    LinkCb,
    EventGraph,
    ProcessingContext,
};
pub use prim::{
    Prim,
    new_prim,
};
pub use vec::{
    Vec,
    new_vec,
};
pub use animate::{
    Animator,
    PrimEaseExt,
};
pub use paste;

#[macro_export]
macro_rules! link{
    (
        (
            $pcname: ident = $pcval: expr,
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
                        & mut $crate:: core:: ProcessingContext,
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
                    fn call(&self, pc:& mut $crate:: core:: ProcessingContext, output:& $outputtype) {
                        (self.f)(pc, output, $(& self.[< _ $vname >],) * $(& self.[< _ $name >],) *);
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
                $crate:: core:: new_link($pcval, $outputval.clone(), _Link {
                    //. x
                    $([< _ $vname >]: $vval,) * 
                    //. x
                    $([< _ $name >]: $val,) * 
                    //. x
                    f:| $pcname,
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
            $pcname: ident = $pcval: expr;
            $($vname: ident = $vval: expr),
            * $(,) ?;
            $($name: ident = $val: expr),
            * $(,) ? $(;) ?
        ) $body: block
    ) => {
        {
            let pc =& mut * $pcval;
            let _out = $crate:: prim:: new_prim(pc, ());
            $crate:: link !(
                ($pcname = pc, _out: $crate:: prim:: Prim <() >= _out; $($vname = $vval), *; $($name = $val), *) {
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

    let eg = EventGraph::new();
    let (_a, b, _link) = eg.event(|pc| {
        let a = new_prim(pc, 0);
        let b = new_prim(pc, 0);

        struct LinkAB {
            a: WeakPrim<i32>,
        }

        impl LinkCb<Prim<i32>> for LinkAB {
            fn call(&self, pc: &mut ProcessingContext, value: &Prim<i32>) {
                let Some(a) = self.a.upgrade() else {
                    return;
                };
                value.set(pc, a.borrow().get() + 5);
            }

            fn inputs(&self) -> std::vec::Vec<Value> {
                return [self.a.clone()].into_iter().filter_map(|x| x.upgrade_as_value()).collect();
            }
        }

        let _link = new_link(pc, b.clone(), LinkAB { a: a.weak() });
        a.set(pc, 46);
        return (a, b, _link);
    });
    assert_eq!(*b.borrow().get(), 51);
}
