use std::{
    rc::Rc,
    cell::{
        RefCell,
    },
};
use lunk::link;

#[test]
fn init_prim_link_prim() {
    let eg = lunk::EventGraph::new();
    let (_input, output, _link) = eg.event(|pc| {
        let input = lunk::Prim::new(pc, 0i32);
        let output = lunk::Prim::new(pc, 0f32);
        let _link = lunk::link!((pc = pc), (input = input.clone()), (output = output.clone()), () {
            output.set(pc, input.get() as f32 + 5.);
        });
        return (input, output, _link);
    });
    assert_eq!(output.get(), 5.);
}

#[test]
fn second_prim_link_prim() {
    let eg = lunk::EventGraph::new();
    let (input, output, _link) = eg.event(|pc| {
        let input = lunk::Prim::new(pc, 0i32);
        let output = lunk::Prim::new(pc, 0f32);
        let link = lunk::link!((pc = pc), (input = input.clone()), (output = output.clone()), () {
            output.set(pc, input.get() as f32 + 5.);
        });
        return (input, output, link);
    });
    eg.event(|pc| {
        input.set(pc, 17);
    });
    assert_eq!(output.get(), 22.);
}

#[test]
fn second_prim_link_prim_set_twice() {
    let eg = lunk::EventGraph::new();
    let (input, output, _link) = eg.event(|pc| {
        let input = lunk::Prim::new(pc, 0i32);
        let output = lunk::Prim::new(pc, 0f32);
        let link = lunk::link!((pc = pc), (input = input.clone()), (output = output.clone()), () {
            output.set(pc, input.get() as f32 + 5.);
        });
        return (input, output, link);
    });
    eg.event(|pc| {
        input.set(pc, 17);
    });
    assert_eq!(output.get(), 22.);
    eg.event(|pc| {
        input.set(pc, 3);
    });
    assert_eq!(output.get(), 8.);
}

#[test]
fn second_prim_link_prim_link_prim() {
    let eg = lunk::EventGraph::new();
    let (a, _b, _link_ab, c, _link_bc) = eg.event(|pc| {
        let a = lunk::Prim::new(pc, 0i32);
        let b = lunk::Prim::new(pc, 0i32);
        let c = lunk::Prim::new(pc, 0i32);
        let link_ab = lunk::link!((pc = pc), (a = a.clone()), (b = b.clone()), () {
            b.set(pc, a.get() + 5);
        });
        let link_bc = lunk::link!((pc = pc), (b = b.clone()), (c = c.clone()), () {
            c.set(pc, b.get() + 6);
        });
        return (a, b, link_ab, c, link_bc);
    });
    eg.event(|pc| {
        a.set(pc, 17);
    });
    assert_eq!(c.get(), 28);
}

#[test]
fn second_prim_link_prim_link_prim_skiplevel() {
    let eg = lunk::EventGraph::new();
    let (a, _b, _link_ab, c, _link_abc) = eg.event(|pc| {
        let a = lunk::Prim::new(pc, 0i32);
        let b = lunk::Prim::new(pc, 0i32);
        let c = lunk::Prim::new(pc, 0i32);
        let link_ab = lunk::link!((pc = pc), (a = a.clone()), (b = b.clone()), () {
            b.set(pc, a.get() + 5);
        });
        let link_abc = lunk::link!((pc = pc), (a = a.clone(), b = b.clone()), (c = c.clone()), () {
            c.set(pc, a.get() - b.get() + 6);
        });
        return (a, b, link_ab, c, link_abc);
    });
    eg.event(|pc| {
        a.set(pc, 17);
    });
    assert_eq!(c.get(), 1);
}

#[test]
fn second_2prim_link_prim() {
    let eg = lunk::EventGraph::new();
    let (_a, b, c, _link_abc) = eg.event(|pc| {
        let a = lunk::Prim::new(pc, 0i32);
        let b = lunk::Prim::new(pc, 0i32);
        let c = lunk::Prim::new(pc, 0i32);
        let link = lunk::link!((pc = pc), (a = a.clone(), b = b.clone()), (c = c.clone()), () {
            c.set(pc, a.get() + b.get() * 2 + 3);
        });
        return (a, b, c, link);
    });
    eg.event(|pc| {
        b.set(pc, 17);
    });
    assert_eq!(c.get(), 37);
}

#[test]
fn second_2prim_2link_2prim_link_prim() {
    let eg = lunk::EventGraph::new();
    let (a, _b, _c, _d, _link_ac, _link_bd, e, _link_cde) = eg.event(|pc| {
        let a = lunk::Prim::new(pc, 0i32);
        let b = lunk::Prim::new(pc, 0i32);
        let c = lunk::Prim::new(pc, 0i32);
        let d = lunk::Prim::new(pc, 0i32);
        let e = lunk::Prim::new(pc, 0i32);
        let link_ac = lunk::link!((pc = pc), (a = a.clone()), (c = c.clone()), () {
            c.set(pc, a.get() + 5);
        });
        let link_bd = lunk::link!((pc = pc), (b = b.clone()), (d = d.clone()), () {
            d.set(pc, b.get() + 6);
        });
        let link_cde = lunk::link!((pc = pc), (c = c.clone(), d = d.clone()), (e = e.clone()), () {
            e.set(pc, c.get() + d.get() * 2 + 10);
        });
        return (a, b, c, d, link_ac, link_bd, e, link_cde);
    });
    eg.event(|pc| {
        a.set(pc, 17);
    });
    assert_eq!(e.get(), 44);
}

#[test]
fn second_2prim_2link_2prim_link_prim_trigger_both() {
    let eg = lunk::EventGraph::new();
    let (a, b, _c, _d, _link_ac, _link_bd, e, _link_cde) = eg.event(|pc| {
        let a = lunk::Prim::new(pc, 0i32);
        let b = lunk::Prim::new(pc, 0i32);
        let c = lunk::Prim::new(pc, 0i32);
        let d = lunk::Prim::new(pc, 0i32);
        let e = lunk::Prim::new(pc, 0i32);
        let link_ac = lunk::link!((pc = pc), (a = a.clone()), (c = c.clone()), () {
            c.set(pc, a.get() + 5);
        });
        let link_bd = lunk::link!((pc = pc), (b = b.clone()), (d = d.clone()), () {
            d.set(pc, b.get() + 6);
        });
        let link_cde = lunk::link!((pc = pc), (c = c.clone(), d = d.clone()), (e = e.clone()), () {
            e.set(pc, c.get() + d.get() * 2 + 10);
        });
        return (a, b, c, d, link_ac, link_bd, e, link_cde);
    });
    eg.event(|pc| {
        a.set(pc, 17);
        b.set(pc, 1);
    });
    assert_eq!(e.get(), 46);
}

#[test]
fn second_prim_link_prim_newlink_newprim() {
    let eg = lunk::EventGraph::new();
    let (_a, b, _link_ab) = eg.event(|pc| {
        let a = lunk::Prim::new(pc, 0i32);
        let b = lunk::Prim::new(pc, 0i32);
        let link = lunk::link!((pc = pc), (a = a.clone()), (b = b.clone()), () {
            b.set(pc, a.get() + 5);
        });
        return (a, b, link);
    });
    let (c, _link_bc) = eg.event(|pc| {
        let c = lunk::Prim::new(pc, 0i32);
        let link = lunk::link!((pc = pc), (b = b.clone()), (c = c.clone()), () {
            c.set(pc, b.get() + 11);
        });
        return (c, link);
    });
    assert_eq!(c.get(), 16);
}

#[test]
fn second_prim_set_prim_newlink_newprim() {
    let eg = lunk::EventGraph::new();
    let (a, b, _link_ab) = eg.event(|pc| {
        let a = lunk::Prim::new(pc, 0i32);
        let b = lunk::Prim::new(pc, 0i32);
        let _link = lunk::link!((pc = pc), (a = a.clone()), (b = b.clone()), () {
            b.set(pc, a.get() + 5);
        });
        return (a, b, _link);
    });
    let (c, _link_bc) = eg.event(|pc| {
        let c = lunk::Prim::new(pc, 0i32);
        let link = lunk::link!((pc = pc), (b = b.clone()), (c = c.clone()), () {
            c.set(pc, b.get() + 11);
        });
        a.set(pc, 7);
        return (c, link);
    });
    assert_eq!(c.get(), 23);
}

#[test]
fn second_prim_link_prim_processing_newlink_newprim() {
    let eg = lunk::EventGraph::new();
    let c_store = Rc::new(RefCell::new(None));
    let (_a, _b, _link_ab) = eg.event(|pc| {
        let a = lunk::Prim::new(pc, 0i32);
        let b = lunk::Prim::new(pc, 0i32);
        let link = lunk::link!((pc = pc), (a = a.clone()), (b = b.clone()), (c_store = c_store.clone()) {
            b.set(pc, a.get() + 5);
            let c = lunk::Prim::new(pc, 0i32);
            *c_store.borrow_mut() = Some((c.clone(), link!((pc = pc), (b = b.clone()), (c = c.clone()), () {
                c.set(pc, b.get() + 12);
            })));
        });
        return (a, b, link);
    });
    assert_eq!(c_store.borrow().as_ref().unwrap().0.get(), 17);
}

#[test]
fn basic_list_init() {
    let eg = lunk::EventGraph::new();
    let (a, _link) = eg.event(|pc: &mut lunk::ProcessingContext<'_>| {
        let z = lunk::Prim::new(pc, 0);
        let a = lunk::List::new(pc, Vec::<i32>::new());
        let _link = lunk::link!((pc = pc), (z = z.clone()), (a = a.clone()), () {
            _ = z;
            while a.borrow_values().len() < 3 {
                a.push(pc, 14);
            }
        });
        return (a, _link);
    });
    assert_eq!(a.borrow_values().clone(), vec![14, 14, 14]);
}

#[test]
fn basic_list() {
    let eg = lunk::EventGraph::new();
    let (_a, b, _link) = eg.event(|pc: &mut lunk::ProcessingContext<'_>| {
        let a = lunk::List::new(pc, vec![]);
        let b = lunk::List::new(pc, vec![]);
        let _link = lunk::link!((pc = pc), (a = a.clone()), (b = b.clone()), () {
            for change in a.borrow_changes().iter() {
                b.splice(pc, change.offset, change.remove, change.add.iter().map(|x| x + 5).collect());
            }
        });
        a.splice(pc, 0, 0, vec![46]);
        return (a, b, _link);
    });
    assert_eq!(b.borrow_values()[0], 51);
}

#[test]
fn basic_list2x() {
    let ec = lunk::EventGraph::new();
    let (a, b, _link) = ec.event(|pc| {
        let a = lunk::List::new(pc, vec![]);
        let b = lunk::List::new(pc, vec![]);
        let _link = lunk::link!((pc = pc), (a = a.clone()), (b = b.clone()), () {
            for change in a.borrow_changes().iter() {
                b.splice(pc, change.offset, change.remove, change.add.iter().map(|x| x + 5).collect());
            }
        });
        a.splice(pc, 0, 0, vec![46]);
        return (a, b, _link);
    });
    assert_eq!(b.borrow_values()[0], 51);
    ec.event(|pc| {
        a.splice(pc, 0, 1, vec![12]);
    });
    assert_eq!(b.borrow_values()[0], 17);
}
