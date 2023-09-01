use lunk::{
    EventGraph,
    list::List,
};

#[test]
fn basic_primitive() {
    let ec = EventGraph::new();
    let (_a, b, _link) = ec.event(|ctx| {
        let a = lunk::Prim::new(ctx, 0i32);
        let b = lunk::Prim::new(ctx, 0i32);
        let _link = lunk::link!((
            ctx = ctx;
            a = a.weak();
            output = b.clone()
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
fn basic_primitive2x() {
    let ec = EventGraph::new();
    let (a, b, _link) = ec.event(|ctx| {
        let a = lunk::Prim::new(ctx, 0i32);
        let b = lunk::Prim::new(ctx, 0i32);
        let _link = lunk::link!((
            ctx = ctx;
            a = a.weak();
            output = b.clone()
        ) {
            let a = a.upgrade()?;
            output.set(ctx, a.borrow().get() + 5);
        });
        a.set(ctx, 46);
        return (a, b, _link);
    });
    assert_eq!(*b.borrow().get(), 51);
    ec.event(|ctx| {
        a.set(ctx, 13);
    });
    assert_eq!(*b.borrow().get(), 18);
}

#[test]
fn basic_list() {
    let ec = EventGraph::new();
    let (_a, b, _link) = ec.event(|ctx| {
        let a = List::new(ctx, vec![]);
        let b = List::new(ctx, vec![]);
        let _link = lunk::link!((
            ctx = ctx;
            a = a.weak();
            output = b.clone()
        ) {
            let a = a.upgrade()?;
            for change in a.changes().iter() {
                output.splice(ctx, change.offset, change.remove, change.add.iter().map(|x| x + 5).collect());
            }
        });
        a.splice(ctx, 0, 0, vec![46]);
        return (a, b, _link);
    });
    assert_eq!(b.values()[0], 51);
}

#[test]
fn basic_list2x() {
    let ec = EventGraph::new();
    let (a, b, _link) = ec.event(|ctx| {
        let a = List::new(ctx, vec![]);
        let b = List::new(ctx, vec![]);
        let _link = lunk::link!((
            ctx = ctx;
            a = a.weak();
            output = b.clone()
        ) {
            let a = a.upgrade()?;
            for change in a.changes().iter() {
                output.splice(ctx, change.offset, change.remove, change.add.iter().map(|x| x + 5).collect());
            }
        });
        a.splice(ctx, 0, 0, vec![46]);
        return (a, b, _link);
    });
    assert_eq!(b.values()[0], 51);
    ec.event(|ctx| {
        a.splice(ctx, 0, 1, vec![12]);
    });
    assert_eq!(b.values()[0], 17);
}
