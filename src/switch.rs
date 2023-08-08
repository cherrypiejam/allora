use core::arch::asm;
use alloc::boxed::Box;

use crate::kobject::Thread;

extern "C" {
    fn context_stack();
    fn context_restore();
}

macro_rules! context_stack {
    () => {
        asm!("stp lr, xzr, [sp, -16]!");
        context_stack()
    };
}

macro_rules! context_restore {
    () => {
        context_restore();
        asm!("ldp lr, xzr, [sp], 16")
    };
}

#[no_mangle]
pub unsafe extern "C" fn switch(_curr: *mut Thread, _next: *mut Thread) {
    context_stack!();

    let curr: *mut Thread;
    let next: *mut Thread;
    asm!("mov {}, x0", out(reg) curr);
    asm!("mov {}, x1", out(reg) next);

    let thread = &mut *curr;
    let next_thread = &mut *next;

    let mut sp: usize;
    asm!("mov {}, sp", out(reg) sp);
    thread.saved.sp = sp;

    if next_thread.saved.sp == 0 {
        sp = &*(next_thread.stack) as *const _ as usize + 256 * 8;
        let next_thread_addr = next_thread as *const _ as usize;
        let entry = *(next_thread as *const _ as *const usize);

        asm!("mov x0, {}",
             "mov x6, {}",
             "mov sp, {}",
             "br x6",
             in(reg) next_thread_addr,
             in(reg) entry,
             in(reg) sp);

    } else {
        sp = next_thread.saved.sp;
        asm!("mov sp, {}", in(reg) sp);
        context_restore!();
    }

}
