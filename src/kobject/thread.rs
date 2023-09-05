use alloc::boxed::Box;
use core::mem::size_of;

use super::{KObjectRef, KObjectArena};
use super::kobject_create;

use crate::mm::PAGE_SIZE;

// pub const STACK_SIZE: usize = 4096;
pub const STACK_SIZE: usize = 1 << 14; // XXX: hardcoded this in switch.S
const STACK_LEN: usize = STACK_SIZE / size_of::<usize>();

// pub const THREAD_NPAGES: usize = 2; // FIXME: init threads with 2 pages for a bigger stack
pub const THREAD_NPAGES: usize = STACK_SIZE / PAGE_SIZE + 12;

#[repr(C)]
pub struct Thread {
    pub main: extern "C" fn(Box<Self, KObjectArena>),
    pub stack: Box<[usize; STACK_LEN], KObjectArena>,
    pub saved_sp: usize,
    pub userdata: Box<dyn FnOnce(), KObjectArena>,
}


impl Thread {
    pub unsafe fn create<F: FnOnce() + 'static>(pg: usize, f: F) -> KObjectRef<Thread> {
        let th_ref = kobject_create!(Thread, pg);
        th_ref
            .as_ptr()
            .write(Thread {
                main: thread_start,
                stack: Box::new_in([0; STACK_LEN], th_ref.meta().alloc.clone()),
                saved_sp: 0,
                userdata: Box::new_in(move || f(), th_ref.meta().alloc.clone()),
            });

        th_ref
    }

}

extern "C" fn thread_start(conf: Box<Thread, KObjectArena>) {
    (conf.userdata)()
}
