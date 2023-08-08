use link::{
    EventContext,
    link,
    Prim,
};

#[test]
fn basic_primitive() {
    let mut ec = EventContext::new();
    let a = Prim::new(&mut ec);
    let b = Prim::new(&mut ec);
    let _d = ec.event(|ctx| {
        let _link = link!((
            ctx = ctx,
            output: Prim<i32> = b;
            a = a;
        ) {
            let a = a.upgrade()?;
            output.set(ctx, a.get() + 5);
        });
        a.set(ctx, 46);
        return _link;
    });
    assert_eq!(b.get(), 51);
}

#[test]
fn basic_vec() {
    let mut ec = EventContext::new();
    let a = link::Vec::new(&mut ec);
    let b = link::Vec::new(&mut ec);
    let _d = ec.event(|ctx| {
        let _link = link!((
            ctx = ctx,
            output: link::Vec<i32> = b;
            a = a;
        ) {
            let a = a.upgrade()?;
            for change in &a.get().changes {
                output.splice(ctx, change.offset, change.remove, change.add.iter().map(|x| x + 5).collect());
            }
        });
        a.splice(ctx, 0, 0, vec![46]);
        return _link;
    });
    assert_eq!(b.get().value[0], 51);
}
