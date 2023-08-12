use core::arch::asm;
use core::sync::atomic::{AtomicBool, Ordering};
use core::fmt::Write;

use crate::{timer, gic, thread};

static YIELD_BEFORE_RETURN: AtomicBool = AtomicBool::new(false);

#[allow(unused)]
#[derive(Debug)]
#[repr(u16)]
pub enum Description {
    CurrentElSP0,
    CurrentElSPx,
    LowerElAArch64,
    LowerElAArch32,
}

#[allow(unused)]
#[derive(Debug)]
#[repr(u16)]
pub enum Kind {
    Synchronous,
    IRQ,
    FIQ,
    SError,
}

#[derive(Debug)]
#[repr(C)]
pub struct Info {
    kind: Kind,
    desc: Description,
}

#[derive(Debug)]
#[repr(C)]
pub struct Frame {
    esr:           u64,        // ESR_EL2
    pstate:        u64,        // SPSR_EL2
    return_addr:   u64,        // ELR_EL2 return address
    thread_addr:   u64,        // TPIDR_EL2
    va_table_base: u64,        // TTBR0_EL2
    v:             [u128; 32], // SIMD registers
    x:             [u64;  29], // General purpose registers
    frame_pointer: u64,        // x29
    link_register: u64,        // x30
}

#[derive(Debug)]
#[repr(u32)]
pub enum InterruptIndex {
    CPUPowerDown = 0, // SGI
    Timer = timer::EL1_PHYSICAL_TIMER,
}

impl InterruptIndex {
    fn is_soft(interrupt: u32) -> bool {
        interrupt & !0x0f == 0
    }
}

const INTERRUPTS: &[(u32, &dyn Fn(u32, &Frame))] = &[
    (InterruptIndex::CPUPowerDown as u32, &cpu_power_down_handler),
    (InterruptIndex::Timer as u32, &timer_interrupt_handler),
];

#[no_mangle]
pub extern "C" fn exception_handler(info: Info, frame: &Frame) {
    // crate::UART.map(|u| { use core::fmt::Write; write!(u, "exception taken 1\n") });
    // crate::debug("exception taken");


        // crate::UART.map(|uart| {
            // let _ = write!(uart, "DEBUG @ Thread {:#x}:\n",
                           // // utils::current_core(),
                           // // utils::current_el(),
                           // // 0,
                           // // 0,
                           // // a >> 6,
                           // // thread::current_thread().map(|t| mm::pgid!(t as *const kobject::Thread as usize)).unwrap_or(0),
                           // // 0,
                           // 0,
                           // );
        // });

    match info.desc {
        Description::CurrentElSPx => match info.kind {
            Kind::IRQ => {
                for &(irq, handler) in INTERRUPTS.iter() {
                    if gic::is_pending(irq) {
                        handler(irq, frame);
                        if InterruptIndex::is_soft(irq) {
                            gic::clear_soft(irq)
                        } else {
                            gic::clear(irq);
                        }
                    }
                }

                if YIELD_BEFORE_RETURN
                    .compare_exchange(true, false, Ordering::SeqCst, Ordering::Relaxed)
                    == Ok(true)
                {
                    thread::yield_to_next();
                }
            }
            Kind::Synchronous => {
                // Ref https://developer.arm.com/documentation/ddi0595/2021-12/AArch64-Registers/ESR-EL2--Exception-Syndrome-Register--EL2-?lang=en#fieldset_0-24_0_8
                unimplemented!("{:?}: exception class {:#b}", info, frame.esr >> 26)
            }
            _ => unimplemented!("{:?}", info)
        }
        _ => unimplemented!("{:?}", info)
    }
}

fn timer_interrupt_handler(_irq: u32, _frame: &Frame) {
    // crate::UART.map(|u| { use core::fmt::Write; write!(u, ".") });
    let tick = timer::tick();
    if tick % thread::TIME_SLICE == 0 {
        let _ =
            YIELD_BEFORE_RETURN
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
            .expect("assumption failed");
    }
}

fn cpu_power_down_handler(irq: u32, _: &Frame) {
    gic::clear_soft(irq); // Must clear before power off
    // cpu_off_graceful();
}

pub fn load_table() {
    unsafe {
        asm!("ldr x0, =exception_vector_table",
             "msr VBAR_EL2, x0");
    }
    interrupt_enable();
}


pub fn with_intr_disabled<F: Fn()>(f: F) {
    let old_mask = interrupt_disable();
    f();
    interrupt_mask_set(old_mask);
}

#[allow(dead_code)]
unsafe fn dump_memory(ptr: *const u8, size: usize) {
    (0..size).for_each(|_| {
        crate::UART.map(|u| {
            let _ = u.write_fmt(format_args!("{:p}: {:x}\n", ptr, core::ptr::read(ptr)));
        });
    })
}

pub fn interrupt_enable() {
    unsafe {
        asm!("msr DAIFClr, 7");
    }
}

pub fn interrupt_disable() -> usize {
    let old_mask = interrupt_mask_get();
    unsafe {
        asm!("msr DAIFSet, 7");
    }
    old_mask
}

pub fn interrupt_mask_get() -> usize {
    unsafe {
        let mask: usize;
        asm!("mrs {}, DAIF", out(reg) mask);
        mask
    }
}

pub fn interrupt_mask_set(mask: usize) {
    unsafe {
        asm!("msr DAIF, {}", in(reg) mask);
    }
}

