#[test]
fn basic_primitive() {
    let eg = lunk::EventGraph::new();
    let (_input_a, _input_b, output, _link) = eg.event(|pc| {
        let input_a = lunk::Prim::new(pc, 0i32);
        let input_b = lunk::Prim::new(pc, 1f32);
        let output = lunk::Prim::new(pc, 0f32);
        let _link =
            lunk::link!(
                (ctx = pc),
                (input_a = input_a.weak(), input_b = input_b.weak()),
                (output = output.weak()),
                () {
                    output.upgrade()?.set(ctx, input_a.upgrade()?.get() as f32 * input_b.upgrade()?.get() + 5.);
                }
            );
        input_a.set(pc, 46);
        return (input_a, input_b, output, _link);
    });
    assert_eq!(output.get(), 51.);
}

#[test]
fn basic_primitive2x() {
    let eg = lunk::EventGraph::new();
    let (a, b, _link) = eg.event(|pc| {
        let a = lunk::Prim::new(pc, 0i32);
        let b = lunk::Prim::new(pc, 0i32);
        let _link = lunk::link!((ctx = pc), (a = a.weak()), (b = b.weak()), () {
            b.upgrade()?.set(ctx, a.upgrade()?.get() + 5);
        });
        a.set(pc, 46);
        return (a, b, _link);
    });
    assert_eq!(b.get(), 51);
    eg.event(|ctx| {
        a.set(ctx, 13);
    });
    assert_eq!(b.get(), 18);
}

#[test]
fn basic_list() {
    let eg = lunk::EventGraph::new();
    let (_a, b, _link) = eg.event(|pc: &mut lunk::ProcessingContext<'_>| {
        let a = lunk::List::new(pc, vec![]);
        let b = lunk::List::new(pc, vec![]);
        let _link = lunk::link!((ctx = pc), (a = a.weak()), (b = b.weak()), () {
            let b = b.upgrade()?;
            for change in a.upgrade()?.changes().iter() {
                b.splice(ctx, change.offset, change.remove, change.add.iter().map(|x| x + 5).collect());
            }
        });
        a.splice(pc, 0, 0, vec![46]);
        return (a, b, _link);
    });
    assert_eq!(b.values()[0], 51);
}

#[test]
fn basic_list2x() {
    let ec = lunk::EventGraph::new();
    let (a, b, _link) = ec.event(|ctx| {
        let a = lunk::List::new(ctx, vec![]);
        let b = lunk::List::new(ctx, vec![]);
        let _link = lunk::link!((ctx = ctx), (a = a.weak()), (b = b.weak()), () {
            let b = b.upgrade()?;
            for change in a.upgrade()?.changes().iter() {
                b.splice(ctx, change.offset, change.remove, change.add.iter().map(|x| x + 5).collect());
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
