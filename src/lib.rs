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
    HistPrim,
};
pub use crate::list::{
    List,
};
pub use crate::animate::{
    Animator,
    HistPrimEaseExt,
};
pub use paste;

/// Define a link, a callback that triggers during event graph processing if any of
/// the inputs change. A link will also be triggered during the event the link was
/// created in, whether inputs changed or not.
///
/// Link specification is like defining a closure, however all captures must be
/// explicitly defined up front. The macro takes the form of a function definition
/// with multiple parentheses for different capture groups.
///
/// ```ignore
/// let _link = link!(
///   (CONTEXT KVS),
///   (INPUT KVS),
///   (OUTPUT KVS),
///   (OTHER CAPTURE KVS) {
///   BODY
/// });
/// ```
///
/// * Each `KVS` group takes values in the form of `x1 = x2` where `x2` is an
///   expression for a value that will be used inside the link callback, and `x1` is
///   the name it's assigned within the callback.
///
/// * `CONTEXT KVS` looks like `pc = pc`, it takes the `ProcessingContext` from the
///   current event.
///
/// * `INPUT KVS` is a list of input values which will trigger this callback to run.
///   This needs at least one item.
///
/// * `OUTPUT KVS` is a list of values that may be modified by this callback.  This can
///   be empty if the callback doesn't cause any downstream processing - for example,
///   in a UI a callback that updates view state.
///
/// * `OTHER CAPTURE KVS` is a list of other values to capture not related to graph
///   processing.  This can be empty.
///
/// * `BODY` is the code of the callback. It should read from the inputs and modify the
///   outputs. It implicitly (no explicit return needed) returns an `Option<()>` to
///   help with short circuiting, for example if you use a weak reference to an input
///   and upgrading that input results in None.
///
/// The link returns a link object. The link callback will only trigger as long as
/// that object exists, so you need to own it as long as it's relevant.
#[macro_export]
macro_rules! link{
    (
        //. .
        ($pcname: ident = $pcval: expr), 
        //. .
        ($($input_name: ident = $input_val: expr), * $(,) ?), 
        //. .
        ($($output_name: ident = $output_val: expr), * $(,) ?), 
        //. .
        ($($name: ident = $val: expr), * $(,) ?) $(,) ? 
        //. .
        $body: block) => {
        $crate:: paste:: paste ! {
            {
                // #
                //
                // DEF
                struct _Link < 
                //. x
                $([< _ $input_name: upper >],) * 
                //. x
                $([< _ $output_name: upper >],) * 
                //. x
                $([< _ $name: upper >],) *
                //. _
                > {
                    //. x
                    $([< _ $input_name >]:[< _ $input_name: upper >],) * 
                    //. x
                    $([< _ $output_name >]:[< _ $output_name: upper >],) * 
                    //. x
                    $([< _ $name >]:[< _ $name: upper >],) * 
                    //. x
                    f: fn(& mut $crate:: core:: ProcessingContext, 
                        //. .
                        $(&[< _ $input_name: upper >],) * 
                        //. .
                        $(&[< _ $output_name: upper >],) * 
                        //. .
                        $(&[< _ $name: upper >],) *) -> Option <() >,
                }
                // #
                //
                // IMPL
                impl < 
                //. x
                $([< _ $input_name: upper >]: Clone,) * 
                //. x
                $([< _ $output_name: upper >]: Clone + $crate:: core:: IntoValue,) * 
                //. x
                $([< _ $name: upper >],) *
                //. _
                > $crate:: core:: LinkTrait for _Link < 
                //. x
                $([< _ $input_name: upper >],) * 
                //. x
                $([< _ $output_name: upper >],) * 
                //. x
                $([< _ $name: upper >],) *
                //. _
                > {
                    fn call(&self, pc:& mut $crate:: core:: ProcessingContext) {
                        (self.f)(pc, 
                            //. .
                            $(& self.[< _ $input_name >],) * 
                            //. .
                            $(& self.[< _ $output_name >],) * 
                            //. .
                            $(& self.[< _ $name >],) *);
                    }
                    fn next(&self) -> std:: vec:: Vec < $crate:: core:: Value > {
                        return vec![
                            $(< dyn $crate:: core:: IntoValue >:: into_value(& self.[< _ $output_name >]),) *
                        ];
                    }
                }
                // #
                //
                // INST
                $(let[< _ $input_name >] = $input_val;) * 
                //. .
                let out = $crate:: Link:: new($pcval, _Link {
                    //. x
                    $([< _ $input_name >]:[< _ $input_name >].clone(),) * 
                    //. x
                    $([< _ $output_name >]: $output_val,) * 
                    //. x
                    $([< _ $name >]: $val,) * 
                    //. x
                    f:| $pcname,
                    //. .
                    $($input_name,) * 
                    //. .
                    $($output_name,) * 
                    //. .
                    $($name,) *
                    //. .
                    |-> Option <() > {
                        $body;
                        return None;
                    }
                });
                //. .
                $([< _ $input_name >].add_next(&out);) * 
                //. .
                out
            }
        }
    };
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

    let eg = EventGraph::new();
    let mut store_a = None;
    let mut store_b = None;
    let mut store_link = None;
    eg.event(|pc| {
        let a = Prim::new(0);
        let b = Prim::new(0);

        struct LinkAB {
            a: WeakPrim<i32>,
            value: Prim<i32>,
        }

        impl LinkTrait for LinkAB {
            fn call(&self, pc: &mut ProcessingContext) {
                let Some(a) = self.a.upgrade() else {
                    return;
                };
                self.value.set(pc, *a.borrow() + 5);
            }

            fn next(&self) -> Vec<Value> {
                return vec![Value(self.value.0.clone())];
            }
        }

        let _link = Link::new(pc, LinkAB {
            a: a.weak(),
            value: b.clone(),
        });
        a.set(pc, 46);
        store_a = Some(a);
        store_b = Some(b);
        store_link = Some(_link);
    });
    assert_eq!(*store_b.unwrap().borrow(), 51);
}
