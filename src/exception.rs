use crate::{UART, timer, system_off, gic};
use core::fmt::Write;
use core::arch::asm;

#[repr(u16)]
#[derive(Debug)]
pub enum Description {
    CurrentElSP0,
    CurrentElSPx,
    LowerElAArch64,
    LowerElAArch32,
}

#[repr(u16)]
#[derive(Debug)]
pub enum Kind {
    Synchronous,
    IRQ,
    FIQ,
    SError,
}

#[repr(C)]
#[derive(Debug)]
pub struct Info {
    kind: Kind,
    desc: Description,
}

#[repr(C)]
#[derive(Debug)]
pub struct Frame {
    pstate: u64,        // spsr_el1
    address: u64,        // elr_el1 return address
    x: [u64; 29],       // general purpose registers
    frame_pointer: u64, // x29
    link_register: u64, // x30
}

#[no_mangle]
pub extern "C" fn exception_handler(info: Info, frame: &Frame) {

    match info.kind {
        Kind::Synchronous => todo!(),
        Kind::IRQ => interrupt_handler(frame),
        Kind::FIQ => todo!(),
        Kind::SError => todo!(),
    }
    // UART.map(|u| write!(u, "{:?}\n{:#?}\n", info, frame));
    // unsafe {
        // system_off();
    // }
}

use alloc::vec::Vec;
fn interrupt_handler(frame: &Frame) {
    let a = (0..100).filter(|&irq| gic::is_pending(irq)).collect::<Vec<u32>>();
    for &irq in a.iter() {
        if gic::is_pending(irq) {
            UART.map(|u| write!(u, "{:?}\n", a));
            UART.map(|u| write!(u, "{:?}\n", irq));
            gic::clear(30);
            timer::reset_tval();
        }
    }
}

pub fn load_table() {
    unsafe {
        asm!("ldr x0, =exception_vector_table",
             "msr VBAR_EL1, x0",
             "msr DAIFClr, 0x7");
    }
}
