use core::arch::asm;
use core::sync::atomic::{AtomicU64, Ordering};
use core::time;
use crate::gic::GIC;
use crate::TASK_LIST;
use crate::thread::cpu_off;

pub const EL1_PHYSICAL_TIMER: u32 = 30;
const SYS_FREQ: u32 = 62_500_000; // 62.5 MHz

const TIMER_FREQ: u32 = 10;
const TIMER_TVAL: u32 = SYS_FREQ / TIMER_FREQ;

static TICK_COUNT: AtomicU64 = AtomicU64::new(0);

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

pub fn current_ticks() -> u64 {
    TICK_COUNT.load(Ordering::SeqCst)
}

pub fn convert_to_ticks(duration: time::Duration) -> u64 {
    duration.as_millis() as u64 / 1000 * TIMER_FREQ as u64
}

pub fn spin_wait(duration: time::Duration) {
    let ticks = current_ticks() + convert_to_ticks(duration);
    while TICK_COUNT
        .compare_exchange(ticks, ticks, Ordering::SeqCst, Ordering::SeqCst)
        != Ok(ticks)
    {}
}

use core::fmt::Write;
pub fn tick() {
    let count = TICK_COUNT.fetch_add(1, Ordering::SeqCst);
    if count % 4 == 0 {
        TASK_LIST.map(|t| {
            while let Some(task) = t.pop() {
                if task.alive_until <= count {
                    task.hold.store(false, Ordering::Relaxed);
                } else {
                    t.push(task);
                    break;
                }
            }
        });
    }
    tick_tail()
}

fn tick_tail() {
    unsafe {
        asm!("msr CNTP_TVAL_EL0, {:x}",
             in(reg) TIMER_TVAL);
    }
}
