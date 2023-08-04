use core::mem::size_of;
use alloc::boxed::Box;

use crate::kobject::{KObjectKind, Container, Thread};
use crate::{KOBJECTS, PAGE_SIZE};
use crate::mm::{pa, koarena::KObjectArena};

use crate::switch::switch;

extern "C" {
    fn cpu_on(core: usize, main: *mut core::ffi::c_void) -> isize;
    fn cpu_off();
}

extern "C" fn thread_start(mut conf: Box<Thread<Box<dyn FnMut(), KObjectArena>>, KObjectArena>) {
    (conf.userdata)()
}


pub fn spawn<F: FnMut() + 'static>(ct: &mut Container, mut f: F) {

    let _ = KOBJECTS.map(|(ks, ofs)| {

        let ct_ref = ct as *const _ as usize / PAGE_SIZE;
        let ct_id = ct_ref - *ofs;
        let (npages, npages_alloc) = (5, 2);
        let pg = ks[ct_id].free_pages.get_multiple(npages).unwrap();

        let th_ref = pg;
        let th_id = pg - *ofs;
        let th_meta = &mut ks[th_id];
        th_meta.kind = KObjectKind::Thread;
        unsafe {
            th_meta.alloc.as_mut().lock().append(
                pa!(pg) + size_of::<Thread<Box<dyn FnMut(), KObjectArena>>>(),
                npages_alloc * PAGE_SIZE - size_of::<Thread<Box<dyn FnMut(), KObjectArena>>>(),
            );

            let th_ptr = pa!(th_ref) as *mut Thread<Box<dyn FnMut(), KObjectArena>>;
            th_ptr.write(Thread {
                main: thread_start,
                stack: Box::new_in([0; 256], th_meta.alloc.clone()),
                saved: Default::default(),
                userdata: Box::new_in(move || {
                    f();

                    switch(th_ptr, th_ptr);

                    f();

                    cpu_off();
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
            cpu_on(1, th_ptr as *mut _);
        }

    });


}

