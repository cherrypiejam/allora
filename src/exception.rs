use crate::{UART, timer, system_off, gic};
use crate::gic::GIC;
use core::fmt::Write;
use core::arch::asm;

#[derive(Debug)]
#[repr(u16)]
pub enum Description {
    CurrentElSP0,
    CurrentElSPx,
    LowerElAArch64,
    LowerElAArch32,
}

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
    pstate: u64,        // Spsr_el1
    address: u64,       // Elr_el1 return address
    x: [u64; 29],       // General purpose registers
    frame_pointer: u64, // x29
    link_register: u64, // x30
}

#[derive(Debug)]
#[repr(u32)]
enum InterruptIndex {
    Timer = timer::EL1_PHYSICAL_TIMER,
}

const INTERRUPTS: &[(u32, &dyn Fn(u32, &Frame))] = &[
    (InterruptIndex::Timer as u32, &timer_interrupt_handler),
];

#[no_mangle]
pub extern "C" fn exception_handler(info: Info, frame: &Frame) {
    match info.desc {
        Description::CurrentElSPx => match info.kind {
            Kind::IRQ => {
                for &(irq, handler) in INTERRUPTS.iter() {
                    if gic::is_pending(irq) {
                        handler(irq, frame);
                    }
                }
            }
            _ => unimplemented!("{:?}", info)
        }
        _ => unimplemented!("{:?}", info)
    }
}

fn timer_interrupt_handler(irq: u32, _frame: &Frame) {
    UART.map(|u| write!(u, "."));
    timer::tick();
    gic::clear(irq);
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
