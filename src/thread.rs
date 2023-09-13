use core::arch::asm;

use crate::kobject::{Container, Thread, Label, ThreadRef, THREAD_NPAGES, KOBJ_NPAGES};
use crate::schedule::{schedule_by_resource_blocks, schedule_thread};
use crate::exception::with_intr_disabled;
use crate::kobject::KObjectRef;
use crate::cpu_idle;

pub const TIME_SLICE: u64 = 4;

const PUBLIC: &str = "T,T";
const BOTTOM: &str = "T,F";
const TOP:    &str = "F,T";

pub fn spawn<F: FnOnce() + 'static>(ct_ref: KObjectRef<Container>, f: F) {
    crate::READY_LIST.map(|l| l.push_back(
        spawn_raw(ct_ref, BOTTOM, f)
    ));
}


pub fn spawn_raw<F: FnOnce() + 'static>(ct_ref: KObjectRef<Container>, label: &str, f: F) -> ThreadRef {
    // label checks
    let curr_ref = current_thread_koref().expect("no current thread");
    if !curr_ref
        .label()
        .unwrap()
        .can_flow_to(&ct_ref.label().unwrap())
    {
        panic!("fail to create the thread with label <{:?}>", label);
    }

    let lb_slot = ct_ref.as_mut().get_slot().unwrap();
    let lb_page_id = ct_ref.meta_mut().free_pages.get_multiple(KOBJ_NPAGES).unwrap();
    let lb_ref = unsafe {
        Label::create(lb_page_id, label)
    };
    lb_ref.meta_mut().parent = Some(ct_ref);
    ct_ref.as_mut().set_slot(lb_slot, lb_ref);

    let th_slot = ct_ref.as_mut().get_slot().unwrap();
    let th_page_id = ct_ref.meta_mut().free_pages.get_multiple(THREAD_NPAGES).unwrap();
    let th_ref = unsafe {
        Thread::create(th_page_id, move || { f(); cpu_idle!(); })
    };
    th_ref.meta_mut().parent = Some(ct_ref);
    th_ref.meta_mut().label = Some(lb_ref);
    ct_ref.as_mut().set_slot(th_slot, th_ref);

    ThreadRef(th_ref)
}


pub fn yield_to_next() {
    with_intr_disabled(|| {
        // schedule();
        schedule_by_resource_blocks();
    })
}

pub fn yield_to(next: ThreadRef) {
    with_intr_disabled(|| {
        schedule_thread(next)
    })
}


pub unsafe fn init_thread(th_ptr: *const Thread) {
    asm!("msr TPIDR_EL2, {}", in(reg) th_ptr as u64);
}

pub fn current_thread<'a>() -> Option<&'a mut Thread> {
    let th_ptr: u64;
    unsafe {
        asm!("mrs {}, TPIDR_EL2", out(reg) th_ptr);
    }
    if th_ptr == 0 {
        None
    } else {
        Some(unsafe {
            &mut *(th_ptr as *mut _)
        })
    }
}

pub fn current_thread_koref() -> Option<KObjectRef<Thread>> {
    current_thread()
        .map(|th| {
            unsafe {
                KObjectRef::<Thread>::new(
                    crate::mm::pgid!(th as *const _ as usize)
                    - 1 // XXX: the beginning of kobjref points to its meta data
                )
            }
        })
}

pub fn current_label() -> Option<KObjectRef<Label>> {
    current_thread_koref()
        .and_then(|th_ref| th_ref.label())
}
