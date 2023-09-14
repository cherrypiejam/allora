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

pub fn search(ct_ref: KObjectRef<Container>, label: &str, avoid: KObjectRef<Container>) -> Option<KObjectRef<Container>> {
    // label checks
    // ct_ref can flow to the current label

    use alloc::vec::Vec;

    use labeled::buckle2::Buckle2;

    use crate::thread::current_thread_koref;

    let local_alloc = current_thread_koref().unwrap().meta().alloc.clone();

    let mut containers = Vec::new();
    let mut visited = Vec::new();
    let mut found = Vec::new();

    containers.push(ct_ref);

    while let Some(ct_ref) = containers.pop() {

        // label checks
        // ct_ref can flow to the current label
        // if not, continue

        if visited.iter().find(|&&v| v == ct_ref).is_some() {
            continue
        } else {
            visited.push(ct_ref)
        }

        if ct_ref != avoid
            && ct_ref.label().unwrap().as_ref().inner
            == Buckle2::parse_in(label, local_alloc.clone()).unwrap()
        {
            found.push(ct_ref)
        }

        if let Some(cts) = ct_ref.as_ref().known_containers.as_ref() {
            cts.iter().for_each(|&ct| {
                containers.push(ct)
            })
        }

    }

    found.pop()

    // after getting the target pool, there are 2 options for us
    // 1. merge the current pool to the target pool
    //      + One main pool: maximize resource utilization
    //      - Consensus & GC
    //      a. redirect
    //          + NO write down (no leak)
    //          - Keeping two pools (underused resource)
    //          - Consensus about who is the main pool is a problem,
    //              especially when the previous main pool is deleted
    //      b. atomic load
    //          + One pool with RC
    //          + No leak
    //          - Write down,
    //              need lang support to enforce the struct rw at a finer level
    // 2. +let the target pool's scheduling thread to manage two pools+
    //      + NOTE: not correct (?)
    //
    // atomic load VS redirect
    //
    //
    //
}
