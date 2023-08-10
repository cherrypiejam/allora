use alloc::boxed::Box;
use core::mem::size_of;

use super::{KObjectRef, KObjectArena};
use super::kobject_create;

#[repr(C)]
#[derive(Default)]
pub struct SavedFrame {
    pub sp: usize,
}

pub const STACK_SIZE: usize = 2048;
const STACK_LEN: usize = STACK_SIZE / size_of::<usize>();

#[repr(C)]
pub struct Thread {
    pub main: extern "C" fn(Box<Self, KObjectArena>),
    pub stack: Box<[usize; STACK_LEN], KObjectArena>,
    pub saved: SavedFrame,
    pub userdata: Box<dyn FnMut(), KObjectArena>,
}


impl Thread {
    pub unsafe fn create<F: FnMut() + 'static>(pg: usize, mut f: F) -> KObjectRef<'static, Thread> {
        let th_ref = kobject_create!(Thread, pg);
        let th_ptr = th_ref.as_ptr();

        th_ref.map_meta(|th_meta| {
            th_ptr.write(Thread {
                main: thread_start,
                stack: Box::new_in([0; 256], th_meta.alloc.clone()),
                saved: Default::default(),
                userdata: Box::new_in(move || f(), th_meta.alloc.clone()),
            });
        });

        th_ref
    }

}

extern "C" fn thread_start(mut conf: Box<Thread, KObjectArena>) {
    (conf.userdata)()
}
