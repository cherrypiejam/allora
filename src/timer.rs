use core::arch::asm;
use crate::gic::GIC;
use crate::TASKS;

pub const EL1_PHYSICAL_TIMER: u32 = 30;
const SYS_FREQ: u32 = 62_500_000; // 62.5 MHz

const TIMER_FREQ: u32 = 1;
const TIMER_TVAL: u32 = SYS_FREQ / TIMER_FREQ;

pub fn init_timer(irq: GIC) {
    unsafe {
        asm!("mov x0, {:x}",            // Set system clock frequency. The cortex-a53 board
             "msr CNTFRQ_EL0, x0",      // uses a fixed val of 62.5 MHz in Qemu

             "msr CNTP_TVAL_EL0, {:x}", // Set timer frequency and enable it
             "isb",
             "mov x0, 1",
             "msr CNTP_CTL_EL0, x0",

             in(reg) SYS_FREQ,
             in(reg) TIMER_TVAL);
    }
    irq.enable();
}

pub fn tick() {
    TASKS.map(|tasks| {
        tasks.sort();
    });
    reset_tval()
}

fn reset_tval() {
    unsafe {
        asm!("msr CNTP_TVAL_EL0, {:x}",
             in(reg) TIMER_TVAL);
    }
}

// pub fn read_regs() {
    // let mut i: u32;
    // unsafe {
        // asm!(
            // "msr CNTP_TVAL_EL0, {:x}",
            // // "isb",
            // "mrs {:x}, CNTFRQ_EL0",
            // in(reg) 62500000,
            // out(reg) i);
    // }
    // use core::fmt::Write;
    // crate::UART.map(|u| writeln!(u, "{:?}", i));
// }
