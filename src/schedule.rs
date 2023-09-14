use crate::kobject::{Thread, ThreadRef, KObjectRef};
use crate::thread::current_thread;
use crate::mm::pgid;
use crate::READY_LIST;
use crate::RESBLOCKS;
use crate::TS;
use crate::kobject::TimeSlices;
use crate::kobject::Container;

extern "C" {
    pub fn switch(curr: *mut core::ffi::c_void, next: *mut core::ffi::c_void);
}

fn find_next_thread(ct_ref: KObjectRef<Container>) -> Option<ThreadRef> {
    let hand = TS
        .lock()
        .as_mut()
        .and_then(|cts| {
            cts.iter_mut()
                .find(|(ct_ref2, _)| *ct_ref2 == ct_ref)
                .and_then(|(ct_ref2, h)| {
                    if let Some(slices) = ct_ref2.as_ref().time_slices.as_ref() {
                        if slices.is_empty() {
                            None
                        } else {
                            let old_h = *h;
                            *h = (*h + 1) % slices.len();
                            Some(old_h)
                        }
                    } else {
                        None
                    }
                })
        });

    use crate::kobject::TimeSlice as TS;
    hand.and_then(|h| {
        if let Some(slices) = ct_ref.as_ref().time_slices.as_ref() {
            match slices[h] {
                TS::Routine => ct_ref.as_ref().scheduler.clone().map(|th| ThreadRef(th)),
                TS::Execute(th) => Some(th.clone()),
                TS::Redirect(ct_ref) => find_next_thread(ct_ref),
            }
        } else {
            None
        }
    })
}

pub fn schedule_by_resource_blocks() {
    if let Some(curr) = current_thread().map(|t| t as *mut Thread) {
        let ts = RESBLOCKS
            .lock()
            .as_mut()
            .and_then(|(rbs, hand)| {
                if rbs.is_empty() {
                    None
                } else {
                    assert!(rbs.len() > *hand);

                    let rb = {
                        let old_hand = *hand;
                        *hand = (*hand + 1) % rbs.len();
                        &mut rbs[old_hand]
                    };

                    find_next_thread(rb.holder)
                }
            });

        if let Some(tref) = ts {
            unsafe {
                switch(curr as *mut _, tref.0.as_ptr() as *mut _)
            }
        }
    }
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
            // crate::debug!("switch from {:p} to {:p}", curr, next_ref.0.as_ptr());
            // crate::UART.map(|u| { use core::fmt::Write; write!(u, "yield to {:#p}\n", next_ref.0.as_ptr()) });
            unsafe {
                switch(curr as *mut _, next_ref.0.as_ptr() as *mut _)
            }
        }

    }
    // Skip when threads are not initialized

}

pub fn schedule_list(list: &mut alloc::collections::VecDeque<ThreadRef>) {

    if let Some(curr) = current_thread().map(|t| t as *mut Thread) {

        if let Some(next_ref) = list.pop_front() {

            list.push_back(ThreadRef(unsafe {
                KObjectRef::new(pgid!(curr as *const _ as usize))
            }));

            unsafe {
                switch(curr as *mut _, next_ref.0.as_ptr() as *mut _)
            }

        }

    }

}

pub fn schedule_thread(next_ref: ThreadRef) {
    if let Some(curr) = current_thread().map(|t| t as *mut Thread) {
        unsafe {
            switch(curr as *mut _, next_ref.0.as_ptr() as *mut _)
        }
    }
}

