use core::mem::size_of;
use core::arch::asm;
use alloc::boxed::Box;

use crate::kobject::{KObjectKind, Container, Thread};
use crate::schedule::schedule;
use crate::{KOBJECTS, PAGE_SIZE};
use crate::mm::{pa, koarena::KObjectArena};

pub const TIME_SLICE: u64 = 4;

extern "C" {
    fn cpu_on(core: usize, main: *mut core::ffi::c_void) -> isize;
    fn cpu_off();
}

extern "C" fn thread_start(mut conf: Box<Thread, KObjectArena>) {
    (conf.userdata)()
}


pub fn spawn<F: FnMut() + 'static>(ct: &mut Container, mut f: F) {

    let _ = KOBJECTS.map(|(ks, ofs)| {

        let ct_ref = ct as *const _ as usize / PAGE_SIZE;
        let ct_id = ct_ref - *ofs;
        let (npages, npages_alloc) = (1, 1);
        let pg = ks[ct_id].free_pages.get_multiple(npages).unwrap();

        let th_ref = pg;
        let th_id = pg - *ofs;
        let th_meta = &mut ks[th_id];
        th_meta.kind = KObjectKind::Thread;
        unsafe {
            th_meta.alloc.as_mut().lock().append(
                pa!(pg) + size_of::<Thread>(),
                npages_alloc * PAGE_SIZE - size_of::<Thread>(),
            );

            let th_ptr = pa!(th_ref) as *mut Thread;
            th_ptr.write(Thread {
                main: thread_start,
                stack: Box::new_in([0; 256], th_meta.alloc.clone()),
                saved: Default::default(),
                userdata: Box::new_in(move || {
                    init_thread(th_ptr);
                    f();
                    loop {}

                    // switch(th_ptr, th_ptr);

                    // f();

                    // cpu_off();
                }, th_meta.alloc.clone()),
            });

            // Update meta info if free pages assigned to the thread
            (npages_alloc..npages)
                .for_each(|i| {
                    th_meta.free_pages.insert(pg + i);
                });

            // Update the container slot
            ct.slots.push(th_ref);

            // Run
            // cpu_on(1, th_ptr as *mut _);

            // Add to the ready queue
            crate::READY_LIST.map(|l| l.push(th_ref))
        }

    });

}

pub fn yield_to_next() {
    // use core::fmt::Write;
    // crate::UART.map(|u| writeln!(u, "yield"));
    schedule();
}


pub unsafe fn init_thread(th_ptr: *const Thread) {
    asm!("msr TPIDR_EL2, {}", in(reg) th_ptr as u64);
}

pub fn current_thread<'a>() -> &'a mut Thread {
    let th_ptr: u64;
    unsafe {
        asm!("mrs {}, TPIDR_EL2", out(reg) th_ptr);
    }
    if th_ptr == 0 {
        panic!("currently not a thread")
    } else {
        unsafe { &mut *(th_ptr as *mut _) }
    }
}
