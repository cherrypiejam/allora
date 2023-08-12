use crate::kobject::{Thread, ThreadRef, KObjectRef};
use crate::switch::switch;
use crate::thread::current_thread;
use crate::mm::pgid;
use crate::READY_LIST;
use crate::RESBLOCKS;


pub fn schedule_rbs() {

    use alloc::format;
    // crate::debug("begin schedule");

    if let Some(curr) = current_thread().map(|t| t as *mut Thread) {
        // crate::debug(&format!("begin 1 {:b}", crate::exception::interrupt_mask_get() >> 6));

        let ts = RESBLOCKS
            .lock()
            .as_mut()
            .and_then(|(rbs, hand)| {

                if rbs.is_empty() {
                    None
                } else {

                    // crate::debug(&format!("begin 3"));

                    assert!(rbs.len() > *hand);

                    let rb = &mut rbs[*hand];
                    let ts = rb.time_slices[rb.hand].clone();

                    rb.hand = (rb.hand + 1) % rb.time_slices.len();
                    // crate::debug(&format!("next rb hand {}", rb.hand));
                    *hand = (*hand + 1) % rbs.len();

                    // crate::debug(&format!("next hand {}", *hand));

                    ts
                }
            });

        if let Some(tref) = ts {
            // crate::debug("begin 2");
            unsafe {
                switch(curr, tref.0.as_ptr())
            }
        }

    }
    // crate::debug("end schedule");

}


pub fn schedule() {

    if let Some(curr) = current_thread().map(|t| t as *mut Thread) {

        let next_ref = READY_LIST.lock().as_mut().and_then(|l| {
            let nref = l.pop_front();
            if nref.is_some() {
                l.push_back(ThreadRef(unsafe {
                    KObjectRef::new(pgid!(curr as *const _ as usize))
                }));
            }
            nref
        });

        if let Some(next_ref) = next_ref {
            // crate::UART.map(|u| { use core::fmt::Write; write!(u, "yield to {:#p}\n", next_ref.0.as_ptr()) });
            unsafe {
                switch(curr, next_ref.0.as_ptr())
            }
        }

    }
    // Skip when threads are not initialized

}


