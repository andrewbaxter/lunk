pub mod core;
pub mod prim;
pub mod list;
pub mod animate;

pub use crate::core::{
    Link,
    LinkTrait,
    EventGraph,
    ProcessingContext,
};
pub use crate::prim::{
    Prim,
};
pub use crate::list::{
    List,
};
pub use crate::animate::{
    Animator,
    PrimEaseExt,
};
pub use paste;

/// Create a link, or mapping function, between a number of input data and a single
/// output.
#[macro_export]
macro_rules! link{
    (
        (
            $pcname: ident = $pcval: expr;
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
                > $crate:: core:: LinkTrait for _Link < 
                //. x
                $([< _ $vname: upper >],) * 
                //. x
                $([< _ $name: upper >],) *
                //. _
                > {
                    fn call(&self, pc:& mut $crate:: core:: ProcessingContext) {
                        (self.f)(pc, $(& self.[< _ $vname >],) * $(& self.[< _ $name >],) *);
                    }
                    fn inputs(&self) -> std:: vec:: Vec < $crate:: core:: Value > {
                        return[
                            $(self.[< _ $vname >].clone().upgrade_as_value(),) *
                        ].into_iter().filter_map(|x| x).collect();
                    }
                }
                // #
                //
                // INST
                $crate:: Link:: new($pcval, _Link {
                    //. x
                    $([< _ $vname >]: $vval,) * 
                    //. x
                    $([< _ $name >]: $val,) * 
                    //. x
                    f:| $pcname,
                    $($vname,) * $($name,) *|-> Option <() > {
                        $body;
                        return None;
                    }
                })
            }
        }
    };
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
        let a = Prim::new(pc, 0);
        let b = Prim::new(pc, 0);

        struct LinkAB {
            a: WeakPrim<i32>,
            value: Prim<i32>,
        }

        impl LinkTrait for LinkAB {
            fn call(&self, pc: &mut ProcessingContext) {
                let Some(a) = self.a.upgrade() else {
                    return;
                };
                self.value.set(pc, a.borrow().get() + 5);
            }

            fn inputs(&self) -> std::vec::Vec<Value> {
                return [self.a.clone()].into_iter().filter_map(|x| x.upgrade_as_value()).collect();
            }
        }

        let _link = Link::new(pc, LinkAB {
            a: a.weak(),
            value: b.clone(),
        });
        a.set(pc, 46);
        return (a, b, _link);
    });
    assert_eq!(*b.borrow().get(), 51);
}
