use lunk::{
    EventGraph,
};

#[test]
fn basic_primitive() {
    let ec = EventGraph::new();
    let (_a, b, _link) = ec.event(|ctx| {
        let a = lunk::new_prim(ctx, 0i32);
        let b = lunk::new_prim(ctx, 0i32);
        let _link = lunk::link!((
            ctx = ctx,
            output: lunk::Prim<i32> = b;
            a = a.weak();
        ) {
            let a = a.upgrade()?;
            output.set(ctx, a.borrow().get() + 5);
        });
        a.set(ctx, 46);
        return (a, b, _link);
    });
    assert_eq!(*b.borrow().get(), 51);
}

#[test]
fn basic_vec() {
    let ec = EventGraph::new();
    let (_a, b, _link) = ec.event(|ctx| {
        let a = lunk::new_vec(ctx, vec![]);
        let b = lunk::new_vec(ctx, vec![]);
        let _link = lunk::link!((
            ctx = ctx,
            output: lunk::Vec<i32> = b;
            a = a.weak();
        ) {
            let a = a.upgrade()?;
            for change in a.borrow().changes() {
                output.splice(ctx, change.offset, change.remove, change.add.iter().map(|x| x + 5).collect());
            }
        });
        a.splice(ctx, 0, 0, vec![46]);
        return (a, b, _link);
    });
    assert_eq!(b.borrow().value()[0], 51);
}
