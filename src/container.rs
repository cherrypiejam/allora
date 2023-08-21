use crate::kobject::{KObjectRef, Container, Label};
use crate::thread;

pub fn create(ct_ref: KObjectRef<Container>, label: &str) -> KObjectRef<Container> {
    // label checks
    if !thread::current_label()
        .unwrap()
        .can_flow_to(&ct_ref.label().unwrap())
    {
        panic!("fail to create a container with label <{:?}>", label);
    }

    let lb_slot = ct_ref.as_mut().get_slot().unwrap();
    let lb_page = ct_ref.map_meta(|ct| ct.free_pages.get().unwrap()).unwrap();
    let lb_ref = unsafe { Label::create(lb_page, label) };
    ct_ref.as_mut().set_slot(lb_slot, lb_ref);

    let new_ct_slot = ct_ref.as_mut().get_slot().unwrap();
    let new_ct_page = ct_ref.map_meta(|ct| ct.free_pages.get().unwrap()).unwrap();
    let new_ct_ref = unsafe { Container::create(new_ct_page) };
    ct_ref.as_mut().set_slot(new_ct_slot, new_ct_ref);
    new_ct_ref.map_meta(|ct| {
        ct.label = Some(lb_ref);
    });

    new_ct_ref
}


pub fn move_npages(ct_ref_1: KObjectRef<Container>, ct_ref_2: KObjectRef<Container>, npages: usize) {
    // label checks (strict)
    // TODO: make it larps instead
    // let th_lb = thread::current_label().unwrap();
    // let lb_1 = ct_ref_1.label().unwrap();
    // let lb_2 = ct_ref_2.label().unwrap();
    // if !th_lb.can_flow_to(&lb_1)
        // || !lb_1.can_flow_to(&th_lb)
        // || !th_lb.can_flow_to(&lb_2)
        // || !lb_2.can_flow_to(&th_lb)
    // {
        // panic!("fail to move {} pages", npages);
    // }

    let page = ct_ref_1.map_meta(|ct| ct.free_pages.get_multiple(npages).unwrap()).unwrap();
    ct_ref_2.map_meta(|ct| {
        (page..(page+npages))
            .for_each(|p| unsafe { ct.free_pages.insert(p) });
    });
}
