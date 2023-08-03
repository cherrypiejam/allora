use core::arch::asm;
use core::sync::atomic::{AtomicU64, Ordering};
use core::time;

use crate::gic::{GIC, self};
use crate::WAIT_LIST;
use crate::exception::InterruptIndex;

pub const EL1_PHYSICAL_TIMER: u32 = 30;
// pub const EL1_PHYSICAL_TIMER: u32 = 26;
const SYS_FREQ: u32 = 62_500_000; // 62.5 MHz

const TIMER_FREQ: u32 = 100;
const TIMER_TVAL: u32 = SYS_FREQ / TIMER_FREQ;

static TICK_COUNT: AtomicU64 = AtomicU64::new(0);

pub fn init_timer(irq: GIC) {
    unsafe {
        asm!("mov x0, {:x}",            // Set system clock frequency. The cortex-a53 board
             "msr CNTFRQ_EL0, x0",      // uses a fixed value of 62.5 MHz in QEMU

             "msr CNTP_TVAL_EL0, {:x}", // Set timer frequency and enable it
             "isb",
             "mov x0, 1",
             "msr CNTP_CTL_EL0, x0",

             in(reg) SYS_FREQ,
             in(reg) TIMER_TVAL);
    }
    irq.enable();
}

pub fn current_ticks() -> u64 {
    TICK_COUNT.load(Ordering::SeqCst)
}

pub fn convert_to_ticks(duration: time::Duration) -> u64 {
    duration.as_millis() as u64 / 1000 * TIMER_FREQ as u64
}

#[allow(unused)]
pub fn spin_wait(duration: time::Duration) {
    let ticks = current_ticks() + convert_to_ticks(duration);
    while TICK_COUNT
        .compare_exchange(ticks, ticks, Ordering::SeqCst, Ordering::SeqCst)
        != Ok(ticks)
    {}
}

pub fn tick() {
    let count = TICK_COUNT.fetch_add(1, Ordering::SeqCst);
    if count % 4 == 0 {
        WAIT_LIST.map(|t| {
            while let Some(task) = t.pop() {
                if task.alive_until <= count {
                    let irq = InterruptIndex::CPUPowerDown as u32;
                    gic::signal_soft(irq, task.cpu as u32);
                } else {
                    t.push(task);
                    break;
                }
            }
        });
    }
    reset_timer()
}

fn reset_timer() {
    unsafe {
        asm!("msr CNTP_TVAL_EL0, {:x}",
             in(reg) TIMER_TVAL);
    }
}
