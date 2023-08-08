use crate::kobject::Thread;
use crate::switch::switch;
use crate::thread::current_thread;
use crate::mm::{pa, koref};
use crate::READY_LIST;

pub fn schedule() {

    if let Some(curr) = current_thread().map(|t| t as *mut Thread) {

        let next_ref = READY_LIST.lock().as_mut().and_then(|l| {
            let nref = l.pop();
            if nref.is_some() {
                l.push(koref!(curr as *const _ as usize));
            }
            nref
        });

        if let Some(next_ref) = next_ref {
            unsafe {
                switch(curr, pa!(next_ref) as *mut Thread)
            }
        }

    }
    // Skip when threads are not initialized

}


