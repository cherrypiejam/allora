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

type ThreadBox = Thread<Box<dyn FnMut(), crate::mm::koarena::KObjectArena>>;

#[no_mangle]
pub unsafe extern "C" fn switch(curr: *mut ThreadBox, next: *mut ThreadBox) {
    context_stack!();

    let curr: *mut ThreadBox;
    let next: *mut ThreadBox;
    asm!("mov {}, x0", out(reg) curr);
    asm!("mov {}, x1", out(reg) next);

    let thread = &mut *curr;
    let next_thread = &mut *next;

    let mut sp: usize;
    asm!("mov {}, sp", out(reg) sp);
    thread.saved.sp = sp;

    sp = next_thread.saved.sp;
    asm!("mov sp, {}", in(reg) sp);

    context_restore!();
}
