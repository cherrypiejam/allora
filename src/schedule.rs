use crate::kobject::Thread;
use crate::switch::switch;
use crate::thread::current_thread;
use crate::mm::{pa, koref};
use crate::READY_LIST;

pub fn schedule() {

    let curr = current_thread() as *mut Thread;

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


