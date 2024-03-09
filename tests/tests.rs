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
    let mut store_input = None;
    let mut store_output = None;
    let mut store_link = None;
    eg.event(|pc| {
        let input = lunk::Prim::new(pc, 0i32);
        let output = lunk::Prim::new(pc, 0f32);
        let _link = lunk::link!((pc = pc), (input = input.clone()), (output = output.clone()), () {
            output.set(pc, *input.borrow() as f32 + 5.);
        });
        store_input = Some(input);
        store_output = Some(output);
        store_link = Some(_link);
    });
    assert_eq!(*store_output.unwrap().borrow(), 5.);
}

#[test]
fn second_prim_link_prim() {
    let eg = lunk::EventGraph::new();
    let mut store_input = None;
    let mut store_output = None;
    let mut store_link = None;
    eg.event(|pc| {
        let input = lunk::Prim::new(pc, 0i32);
        let output = lunk::Prim::new(pc, 0f32);
        let link = lunk::link!((pc = pc), (input = input.clone()), (output = output.clone()), () {
            output.set(pc, *input.borrow() as f32 + 5.);
        });
        store_input = Some(input);
        store_output = Some(output);
        store_link = Some(link);
    });
    eg.event(|pc| {
        store_input.unwrap().set(pc, 17);
    });
    assert_eq!(*store_output.unwrap().borrow(), 22.);
}

#[test]
fn second_prim_link_prim_set_twice() {
    let eg = lunk::EventGraph::new();
    let mut store_input = None;
    let mut store_output = None;
    let mut store_link = None;
    eg.event(|pc| {
        let input = lunk::Prim::new(pc, 0i32);
        let output = lunk::Prim::new(pc, 0f32);
        let link = lunk::link!((pc = pc), (input = input.clone()), (output = output.clone()), () {
            output.set(pc, *input.borrow() as f32 + 5.);
        });
        store_input = Some(input);
        store_output = Some(output);
        store_link = Some(link);
    });
    eg.event(|pc| {
        store_input.as_ref().unwrap().set(pc, 17);
    });
    assert_eq!(*store_output.as_ref().unwrap().borrow(), 22.);
    eg.event(|pc| {
        store_input.unwrap().set(pc, 3);
    });
    assert_eq!(*store_output.unwrap().borrow(), 8.);
}

#[test]
fn second_prim_link_prim_link_prim() {
    let eg = lunk::EventGraph::new();
    let mut store_a = None;
    let mut store_c = None;
    let mut store_other = None;
    eg.event(|pc| {
        let a = lunk::Prim::new(pc, 0i32);
        let b = lunk::Prim::new(pc, 0i32);
        let c = lunk::Prim::new(pc, 0i32);
        let link_ab = lunk::link!((pc = pc), (a = a.clone()), (b = b.clone()), () {
            b.set(pc, *a.borrow() + 5);
        });
        let link_bc = lunk::link!((pc = pc), (b = b.clone()), (c = c.clone()), () {
            c.set(pc, *b.borrow() + 6);
        });
        store_a = Some(a);
        store_c = Some(c);
        store_other = Some((b, link_ab, link_bc));
    });
    eg.event(|pc| {
        store_a.unwrap().set(pc, 17);
    });
    assert_eq!(*store_c.unwrap().borrow(), 28);
}

#[test]
fn second_prim_link_prim_link_prim_skiplevel() {
    let eg = lunk::EventGraph::new();
    let mut store_a = None;
    let mut store_c = None;
    let mut store_other = None;
    eg.event(|pc| {
        let a = lunk::Prim::new(pc, 0i32);
        let b = lunk::Prim::new(pc, 0i32);
        let c = lunk::Prim::new(pc, 0i32);
        let link_ab = lunk::link!((pc = pc), (a = a.clone()), (b = b.clone()), () {
            b.set(pc, *a.borrow() + 5);
        });
        let link_abc = lunk::link!((pc = pc), (a = a.clone(), b = b.clone()), (c = c.clone()), () {
            c.set(pc, *a.borrow() - *b.borrow() + 6);
        });
        store_a = Some(a);
        store_c = Some(c);
        store_other = Some((b, link_ab, link_abc));
    });
    eg.event(|pc| {
        store_a.unwrap().set(pc, 17);
    });
    assert_eq!(*store_c.unwrap().borrow(), 1);
}

#[test]
fn second_2prim_link_prim() {
    let eg = lunk::EventGraph::new();
    let mut store_b = None;
    let mut store_c = None;
    let mut store_other = None;
    eg.event(|pc| {
        let a = lunk::Prim::new(pc, 0i32);
        let b = lunk::Prim::new(pc, 0i32);
        let c = lunk::Prim::new(pc, 0i32);
        let link = lunk::link!((pc = pc), (a = a.clone(), b = b.clone()), (c = c.clone()), () {
            c.set(pc, *a.borrow() + *b.borrow() * 2 + 3);
        });
        store_b = Some(b);
        store_c = Some(c);
        store_other = Some((a, link));
    });
    eg.event(|pc| {
        store_b.unwrap().set(pc, 17);
    });
    assert_eq!(*store_c.unwrap().borrow(), 37);
}

#[test]
fn second_2prim_2link_2prim_link_prim() {
    let eg = lunk::EventGraph::new();
    let mut store_a = None;
    let mut store_e = None;
    let mut store_other = None;
    eg.event(|pc| {
        let a = lunk::Prim::new(pc, 0i32);
        let b = lunk::Prim::new(pc, 0i32);
        let c = lunk::Prim::new(pc, 0i32);
        let d = lunk::Prim::new(pc, 0i32);
        let e = lunk::Prim::new(pc, 0i32);
        let link_ac = lunk::link!((pc = pc), (a = a.clone()), (c = c.clone()), () {
            c.set(pc, *a.borrow() + 5);
        });
        let link_bd = lunk::link!((pc = pc), (b = b.clone()), (d = d.clone()), () {
            d.set(pc, *b.borrow() + 6);
        });
        let link_cde = lunk::link!((pc = pc), (c = c.clone(), d = d.clone()), (e = e.clone()), () {
            e.set(pc, *c.borrow() + *d.borrow() * 2 + 10);
        });
        store_a = Some(a);
        store_e = Some(e);
        store_other = Some((b, c, d, link_ac, link_bd, link_cde));
    });
    eg.event(|pc| {
        store_a.unwrap().set(pc, 17);
    });
    assert_eq!(*store_e.unwrap().borrow(), 44);
}

#[test]
fn second_2prim_2link_2prim_link_prim_trigger_both() {
    let eg = lunk::EventGraph::new();
    let mut store_a = None;
    let mut store_b = None;
    let mut store_e = None;
    let mut store_other = None;
    eg.event(|pc| {
        let a = lunk::Prim::new(pc, 0i32);
        let b = lunk::Prim::new(pc, 0i32);
        let c = lunk::Prim::new(pc, 0i32);
        let d = lunk::Prim::new(pc, 0i32);
        let e = lunk::Prim::new(pc, 0i32);
        let link_ac = lunk::link!((pc = pc), (a = a.clone()), (c = c.clone()), () {
            c.set(pc, *a.borrow() + 5);
        });
        let link_bd = lunk::link!((pc = pc), (b = b.clone()), (d = d.clone()), () {
            d.set(pc, *b.borrow() + 6);
        });
        let link_cde = lunk::link!((pc = pc), (c = c.clone(), d = d.clone()), (e = e.clone()), () {
            e.set(pc, *c.borrow() + *d.borrow() * 2 + 10);
        });
        store_a = Some(a);
        store_b = Some(b);
        store_e = Some(e);
        store_other = Some((c, d, link_ac, link_bd, link_cde));
    });
    eg.event(|pc| {
        store_a.unwrap().set(pc, 17);
        store_b.unwrap().set(pc, 1);
    });
    assert_eq!(*store_e.unwrap().borrow(), 46);
}

#[test]
fn second_prim_link_prim_newlink_newprim() {
    let eg = lunk::EventGraph::new();
    let mut store_b = None;
    let mut store_other = None;
    eg.event(|pc| {
        let a = lunk::Prim::new(pc, 0i32);
        let b = lunk::Prim::new(pc, 0i32);
        let link = lunk::link!((pc = pc), (a = a.clone()), (b = b.clone()), () {
            b.set(pc, *a.borrow() + 5);
        });
        store_b = Some(b);
        store_other = Some((a, link));
    });
    let mut store_c = None;
    let mut store_other2 = None;
    eg.event(|pc| {
        let c = lunk::Prim::new(pc, 0i32);
        let link = lunk::link!((pc = pc), (b = store_b.unwrap().clone()), (c = c.clone()), () {
            c.set(pc, *b.borrow() + 11);
        });
        store_c = Some(c);
        store_other2 = Some(link);
    });
    assert_eq!(*store_c.unwrap().borrow(), 16);
}

#[test]
fn second_prim_set_prim_newlink_newprim() {
    let eg = lunk::EventGraph::new();
    let mut store_a = None;
    let mut store_b = None;
    let mut store_other = None;
    eg.event(|pc| {
        let a = lunk::Prim::new(pc, 0i32);
        let b = lunk::Prim::new(pc, 0i32);
        let _link = lunk::link!((pc = pc), (a = a.clone()), (b = b.clone()), () {
            b.set(pc, *a.borrow() + 5);
        });
        store_a = Some(a);
        store_b = Some(b);
        store_other = Some(_link);
    });
    let mut store_c = None;
    let mut store_other2 = None;
    eg.event(|pc| {
        let c = lunk::Prim::new(pc, 0i32);
        let link = lunk::link!((pc = pc), (b = store_b.unwrap().clone()), (c = c.clone()), () {
            c.set(pc, *b.borrow() + 11);
        });
        store_a.unwrap().set(pc, 7);
        store_c = Some(c);
        store_other2 = Some(link);
    });
    assert_eq!(*store_c.unwrap().borrow(), 23);
}

#[test]
fn second_prim_link_prim_processing_newlink_newprim() {
    let eg = lunk::EventGraph::new();
    let store_c = Rc::new(RefCell::new(None));
    let mut store_other = None;
    eg.event(|pc| {
        let a = lunk::Prim::new(pc, 0i32);
        let b = lunk::Prim::new(pc, 0i32);
        let link = lunk::link!((pc = pc), (a = a.clone()), (b = b.clone()), (c_store = store_c.clone()) {
            b.set(pc, *a.borrow() + 5);
            let c = lunk::Prim::new(pc, 0i32);
            *c_store.borrow_mut() = Some((c.clone(), link!((pc = pc), (b = b.clone()), (c = c.clone()), () {
                c.set(pc, *b.borrow() + 12);
            })));
        });
        store_other = Some((a, b, link));
    });
    assert_eq!(*store_c.borrow().as_ref().unwrap().0.borrow(), 17);
}

#[test]
fn basic_list_init() {
    let eg = lunk::EventGraph::new();
    let mut store_a = None;
    let mut store_other = None;
    eg.event(|pc: &mut lunk::ProcessingContext<'_>| {
        let z = lunk::Prim::new(pc, 0);
        let a = lunk::List::new(pc, Vec::<i32>::new());
        let _link = lunk::link!((pc = pc), (z = z.clone()), (a = a.clone()), () {
            _ = z;
            while a.borrow_values().len() < 3 {
                a.push(pc, 14);
            }
        });
        store_a = Some(a);
        store_other = Some((z, _link));
    });
    assert_eq!(store_a.unwrap().borrow_values().clone(), vec![14, 14, 14]);
}

#[test]
fn basic_list() {
    let eg = lunk::EventGraph::new();
    let mut store_b = None;
    let mut store_other = None;
    eg.event(|pc: &mut lunk::ProcessingContext<'_>| {
        let a = lunk::List::new(pc, vec![]);
        let b = lunk::List::new(pc, vec![]);
        let _link = lunk::link!((pc = pc), (a = a.clone()), (b = b.clone()), () {
            for change in a.borrow_changes().iter() {
                b.splice(pc, change.offset, change.remove, change.add.iter().map(|x| x + 5).collect());
            }
        });
        a.splice(pc, 0, 0, vec![46]);
        store_b = Some(b);
        store_other = Some((a, _link));
    });
    assert_eq!(store_b.unwrap().borrow_values()[0], 51);
}

#[test]
fn basic_list2x() {
    let eg = lunk::EventGraph::new();
    let mut store_a = None;
    let mut store_b = None;
    let mut store_other = None;
    eg.event(|pc| {
        let a = lunk::List::new(pc, vec![]);
        let b = lunk::List::new(pc, vec![]);
        let _link = lunk::link!((pc = pc), (a = a.clone()), (b = b.clone()), () {
            for change in a.borrow_changes().iter() {
                b.splice(pc, change.offset, change.remove, change.add.iter().map(|x| x + 5).collect());
            }
        });
        a.splice(pc, 0, 0, vec![46]);
        store_a = Some(a);
        store_b = Some(b);
        store_other = Some(_link);
    });
    assert_eq!(store_b.as_ref().unwrap().borrow_values()[0], 51);
    eg.event(|pc| {
        store_a.unwrap().splice(pc, 0, 1, vec![12]);
    });
    assert_eq!(store_b.unwrap().borrow_values()[0], 17);
}
