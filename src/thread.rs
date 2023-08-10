use core::arch::asm;

use crate::kobject::{Container, Thread, ThreadRef};
use crate::schedule::schedule;
use crate::exception::with_intr_disabled;
use crate::kobject::KObjectRef;

pub const TIME_SLICE: u64 = 4;


pub fn spawn<F: FnMut() + 'static>(ct_ref: KObjectRef<Container>, mut f: F) {
    let th_slot = ct_ref.as_mut().get_slot().unwrap();
    let th_page_id = ct_ref.map_meta(|m| m.free_pages.get()).unwrap().unwrap();

    let th_ref = unsafe {
        Thread::create(th_page_id, move || { f(); loop {} })
    };

    ct_ref.as_mut().set_slot(th_slot, th_ref);

    crate::READY_LIST.map(|l| l.push_back(ThreadRef(th_ref)));
}


pub fn yield_to_next() {
    schedule();
}

pub fn yield_with_insr_disabled() {
    with_intr_disabled(|| {
        schedule();
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
