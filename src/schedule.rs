use crate::kobject::{Thread, ThreadRef, KObjectRef};
use crate::switch::switch;
use crate::thread::current_thread;
use crate::mm::pgid;
use crate::READY_LIST;

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
            unsafe {
                switch(curr, next_ref.0.as_ptr())
            }
        }

    }
    // Skip when threads are not initialized

}


