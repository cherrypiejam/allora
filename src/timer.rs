use core::arch::asm;
use crate::gic::GIC;

pub const EL1_PHYSICAL_TIMER: u32 = 0x1e;

const SYS_FREQ: u32 = 62_500_000;
// const TIMER_FREQ: u32 = 1_000;
const TIMER_FREQ: u32 = 1_00;
pub const TIMER_TVAL: u32 = SYS_FREQ / TIMER_FREQ;

pub struct Timer(GIC);

impl Timer {
    pub fn init(irq: GIC) {

        let i: u32 = 62_500_000; // FIXME remove after debug
        // let i: u32 = TIMER_TVAL;
        unsafe {
            asm!("msr CNTP_TVAL_EL0, {:x}",
                 "isb",
                 "mov x0, 1",
                 "msr CNTP_CTL_EL0, x0",
                 in(reg) i);
        }

        irq.enable();
    }
}

pub fn reset_tval() {
    unsafe {
        asm!("msr CNTP_TVAL_EL0, {:x}",
            in(reg) TIMER_TVAL);
    }
}

