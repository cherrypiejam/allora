use crate::kobject::{KObjectRef, Container, Label, KOBJ_NPAGES};
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
    let lb_page = ct_ref.meta_mut().free_pages.get_multiple(KOBJ_NPAGES).unwrap();
    let lb_ref = unsafe { Label::create(lb_page, label) };
    ct_ref.as_mut().set_slot(lb_slot, lb_ref);

    let new_ct_slot = ct_ref.as_mut().get_slot().unwrap();
    let new_ct_page = ct_ref.meta_mut().free_pages.get_multiple(KOBJ_NPAGES).unwrap();
    let new_ct_ref = unsafe { Container::create(new_ct_page) };
    ct_ref.as_mut().set_slot(new_ct_slot, new_ct_ref);
    new_ct_ref.meta_mut().label = Some(lb_ref);

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

    let page = ct_ref_1.meta_mut().free_pages.get_multiple(npages).unwrap();
    (page..(page+npages))
        .for_each(|p| unsafe {
            ct_ref_2.meta_mut().free_pages.insert(p)
        });
}

pub fn move_time_slices() {}

pub fn search(ct_ref: KObjectRef<Container>, key: usize) -> Option<KObjectRef<Container>> {
    // label checks
    // ct_ref can flow to the current label

    let mut cur = ct_ref;
    // ct_ref.as_ref().known_containers.unwrap().iter().find(|ctref|)
    // search all from root
    // find the oldest one

    None
    // 2 options after getting the target pool
    // 1. merge to the target pool
    // 2. let the target pool's scheduling thread to manage two pools
    //
    // atomic load VS redirect
}
