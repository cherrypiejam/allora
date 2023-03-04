use crate::{timer, gic};
use crate::thread::cpu_off_graceful;

use core::arch::asm;
use core::fmt::Write;

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
    pstate:        u64,        // SPSR_EL1
    address:       u64,        // ELR_EL1 return address
    thread_id:     u64,        // TPIDR_EL1
    va_table_base: u64,        // TTBR0_EL1
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
    match info.desc {
        Description::CurrentElSPx => match info.kind {
            Kind::IRQ => {
                // FIXME: potential issue, an interrupt is triggered
                // but the info is corrupted.
                // e.g. <><><><><> 0A { ate: , address: , x: panicked at 'not implemented: Info { kind: qemu-system-aarch64: terminating on signal 2 from pid 76610 (<unknown process>)
                // heap allocation may use this memory for exception handler?
                // check the argument passed in the exception handler in exception.S
                // the argument is corrupted
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
            }
            _ => unimplemented!("u1 {:?}", info)
        }
        _ => unimplemented!("u2 {:?}", info)
    }
}

unsafe fn dump_memory(ptr: *const u8, size: usize) {
    (0..size).for_each(|_| {
        crate::UART.map(|u| {
            let _ = u.write_fmt(format_args!("{:p}: {:x}\n", ptr, core::ptr::read(ptr)));
        });
    })
}

fn timer_interrupt_handler(_irq: u32, _frame: &Frame) {
    // UART.map(|u| write!(u, "."));
    timer::tick();
}


fn cpu_power_down_handler(irq: u32, _: &Frame) {
    gic::clear_soft(irq); // Must clear before power off
    cpu_off_graceful();
}


pub fn load_table() {
    unsafe {
        asm!("ldr x0, =exception_vector_table",
             "msr VBAR_EL1, x0");
    }
    interrupt_enable();
}

pub fn interrupt_enable() {
    unsafe {
        asm!("msr DAIFClr, 7");
    }
}

pub fn interrupt_disable() {
    unsafe {
        asm!("msr DAIFSet, 7");
    }
}

pub struct InterruptDisabled;

impl InterruptDisabled {
    pub fn with<F: Fn()>(f: F) {
        interrupt_disable();
        f();
        interrupt_enable();
    }
}



