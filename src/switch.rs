use core::arch::asm;

use crate::kobject::{Thread, STACK_SIZE};

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
    asm!("mov {}, x0",
         "mov {}, x1",
         out(reg) curr,
         out(reg) next);

    let thread = &mut *curr;
    let next_thread = &mut *next;

    let mut sp: usize;
    asm!("mov {}, sp", out(reg) sp);
    thread.saved.sp = sp;

    asm!("msr TPIDR_EL2, {}", in(reg) next);

    if next_thread.saved.sp == 0 {
        sp = &*(next_thread.stack) as *const _ as usize + STACK_SIZE;
        let entry = *(next_thread as *const _ as *const usize);

        asm!("msr TPIDR_EL2, {0}",
             "mov x0, {0}",
             "mov x6, {1}",
             "mov sp, {2}",
             "msr DAIFClr, 7", // Enable interrupts
             "br x6",
             in(reg) next,
             in(reg) entry,
             in(reg) sp);

    } else {
        sp = next_thread.saved.sp;

        asm!("msr TPIDR_EL2, {}",
             "mov sp, {}",
             in(reg) next,
             in(reg) sp);
        context_restore!();

    }
}
