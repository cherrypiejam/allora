use core::arch::asm;

use crate::kobject::{Container, Thread, Label, ThreadRef, THREAD_NPAGES};
use crate::schedule::{schedule, schedule_rbs};
use crate::exception::with_intr_disabled;
use crate::kobject::KObjectRef;
use crate::cpu_idle;

pub const TIME_SLICE: u64 = 4;

const PUBLIC: &str = "T,T";
const BOTTOM: &str = "T,F";
const TOP:    &str = "F,T";

pub fn spawn<F: FnMut() + 'static>(ct_ref: KObjectRef<Container>, f: F) {
    crate::READY_LIST.map(|l| l.push_back(
        spawn_thref(ct_ref, BOTTOM, f)
    ));
}


pub fn spawn_thref<F: FnMut() + 'static>(ct_ref: KObjectRef<Container>, label: &str, mut f: F) -> ThreadRef {
    let lb_slot = ct_ref.as_mut().get_slot().unwrap();
    let lb_page_id = ct_ref.map_meta(|m| m.free_pages.get()).unwrap().unwrap();

    let lb_ref = unsafe {
        Label::create(lb_page_id, label)
    };
    lb_ref.map_meta(|lb| lb.parent = Some(ct_ref));
    ct_ref.as_mut().set_slot(lb_slot, lb_ref);

    // label checking
    let curr = current_thread().expect("no current thread");
    let curr_ref = unsafe { KObjectRef::<Thread>::new(crate::mm::pgid!(curr as *const _ as usize)) };
    if !curr_ref
        .label()
        .unwrap()
        .can_flow_to(&ct_ref.label().unwrap())
    {
        panic!("fail to create the thread with label <{:?}>", label);
    }

    let th_slot = ct_ref.as_mut().get_slot().unwrap();
    let th_page_id = ct_ref.map_meta(|m| m.free_pages.get_multiple(THREAD_NPAGES)).unwrap().unwrap();
    let th_ref = unsafe {
        Thread::create(th_page_id, move || { f(); cpu_idle(); })
    };

    th_ref.map_meta(|th| {
        th.parent = Some(ct_ref);
        th.label = Some(lb_ref);
    });

    ct_ref.as_mut().set_slot(th_slot, th_ref);

    ThreadRef(th_ref)
}


pub fn yield_to_next() {
    with_intr_disabled(|| {
        // schedule();
        schedule_rbs();
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
