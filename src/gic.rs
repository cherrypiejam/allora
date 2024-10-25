//
// gic.rs - generic interrupt controller driver
// Copyright (C) 2020 Ilja Kartašov <ik@lowenware.com>
// Distributed under terms of the MIT license.
//

use core::ptr;

// Distributor
const GICD_BASE:       u64      = 0x8000000; // TODO: board-dependent value
const GICD_CTLR:       *mut u32 = GICD_BASE as *mut u32;
const GICD_ISENABLER:  *mut u32 = (GICD_BASE + 0x0100) as *mut u32;
const GICD_ICENABLER:  *mut u32 = (GICD_BASE + 0x0180) as *mut u32;
//const GICD_ISPENDR:  *mut u32 = (GICD_BASE + 0x0200) as *mut u32;
const GICD_ICPENDR:    *mut u32 = (GICD_BASE + 0x0280) as *mut u32;
const GICD_ITARGETSR:  *mut u32 = (GICD_BASE + 0x0800) as *mut u32;
const GICD_IPRIORITYR: *mut u32 = (GICD_BASE + 0x0400) as *mut u32;
const GICD_ICFGR:      *mut u32 = (GICD_BASE + 0x0c00) as *mut u32;
const GICD_SGIR:       *mut u32 = (GICD_BASE + 0x0f00) as *mut u32;
const GICD_CPENDSGIR:  *mut u32 = (GICD_BASE + 0x0f10) as *mut u32;

const GICD_CTLR_ENABLE:      u32 = 1;
//const GICD_CTLR_DISABLE:   u32 = 0;
//const GICD_ICENABLER_SIZE: u32 = 32;
const GICD_ISENABLER_SIZE:   u32 = 32;
const GICD_ICPENDR_SIZE:     u32 = 32;
const GICD_ITARGETSR_SIZE:   u32 = 4; // number of interrupts controlled by the register
const GICD_ITARGETSR_BITS:   u32 = 8; // number of bits per interrupt
const GICD_CPENDSGIR_SIZE:   u32 = 4;
const GICD_SGIR_CPU_SHIFT:   u32 = 16;

const GICD_IPRIORITY_SIZE: u32 = 4;
const GICD_IPRIORITY_BITS: u32 = 8;
const GICD_ICFGR_SIZE:     u32 = 16;
const GICD_ICFGR_BITS:     u32 = 2;

// CPU
const GICC_BASE: u64      = 0x8010000; // TODO: board-dependent value
const GICC_CTLR: *mut u32 = GICC_BASE as *mut u32;
const GICC_PMR:  *mut u32 = (GICC_BASE + 0x0004) as *mut u32;
const GICC_BPR:  *mut u32 = (GICC_BASE + 0x0008) as *mut u32;

const GICC_CTLR_ENABLE:     u32 = 1;
//const GICC_CTLR_DISABLE:  u32 = 0;
const GICC_PMR_PRIO_LOW:    u32 = 0xff;
//const GICC_PMR_PRIO_HIGH: u32 = 0x00;
const GICC_BPR_NO_GROUP:    u32 = 0x00;

pub const ICFGR_EDGE: u32 = 2;

pub fn init() {
    unsafe {
        ptr::write_volatile(GICD_CTLR, GICD_CTLR_ENABLE);
        ptr::write_volatile(GICC_CTLR, GICC_CTLR_ENABLE);
        ptr::write_volatile(GICC_PMR, GICC_PMR_PRIO_LOW);
        ptr::write_volatile(GICC_BPR, GICC_BPR_NO_GROUP);
    }
}

pub fn enable(interrupt: u32) {
    unsafe {
        ptr::write_volatile(
            GICD_ISENABLER.add((interrupt / GICD_ISENABLER_SIZE) as usize),
            1 << (interrupt % GICD_ISENABLER_SIZE),
        );
    }
}

pub fn disable(interrupt: u32) {
    unsafe {
        ptr::write_volatile(
            GICD_ICENABLER.add((interrupt / GICD_ISENABLER_SIZE) as usize),
            1 << (interrupt % GICD_ISENABLER_SIZE),
        );
    }
}

pub fn is_pending(interrupt: u32) -> bool {
    unsafe {
        ptr::read_volatile(
            GICD_ICPENDR.add((interrupt / GICD_ICPENDR_SIZE) as usize)
        ) & (1 << (interrupt % GICD_ICPENDR_SIZE)) != 0
    }
}

pub fn clear(interrupt: u32) {
    unsafe {
        ptr::write_volatile(
            GICD_ICPENDR.add((interrupt / GICD_ICPENDR_SIZE) as usize),
            1 << (interrupt % GICD_ICPENDR_SIZE),
        );
    }
}

pub fn clear_soft(interrupt: u32) {
    unsafe {
        ptr::write_volatile(
            GICD_CPENDSGIR.add(((interrupt / GICD_CPENDSGIR_SIZE) << 2) as usize),
            0xff << ((interrupt % GICD_CPENDSGIR_SIZE) << 2), // ad hoc
        );
    }
}

pub fn get_core(interrupt: u32) -> u32 {
    let shift: u32 = (interrupt % GICD_ITARGETSR_SIZE) * GICD_ITARGETSR_BITS;
    unsafe {
        let addr: *mut u32 = GICD_ITARGETSR.add((interrupt / GICD_ITARGETSR_SIZE) as usize);
        let value: u32 = ptr::read_volatile(addr);
        let cores = (value >> shift) & 0xff;
        (cores & (1 << cores.trailing_zeros())) - 1
    }
}

pub fn set_core(interrupt: u32, core: u32) {
    let shift: u32 = (interrupt % GICD_ITARGETSR_SIZE) * GICD_ITARGETSR_BITS;
    unsafe {
        let addr: *mut u32 = GICD_ITARGETSR.add((interrupt / GICD_ITARGETSR_SIZE) as usize);
        let mut value: u32 = ptr::read_volatile(addr);
        value &= !(0xff << shift);
        value |= (1 << core) << shift;
        ptr::write_volatile(addr, value);
    }
}

pub fn set_priority(interrupt: u32, priority: u32) {
    let shift = (interrupt % GICD_IPRIORITY_SIZE) * GICD_IPRIORITY_BITS;
    unsafe {
        let addr: *mut u32 = GICD_IPRIORITYR.add((interrupt / GICD_IPRIORITY_SIZE) as usize);
        let mut value: u32 = ptr::read_volatile(addr);
        value &= !(0xff << shift);
        value |= priority << shift;
        ptr::write_volatile(addr, value);
    }
}

pub fn set_config(interrupt: u32, config: u32) {
    let shift = (interrupt % GICD_ICFGR_SIZE) * GICD_ICFGR_BITS;
    unsafe {
        let addr: *mut u32 = GICD_ICFGR.add((interrupt / GICD_ICFGR_SIZE) as usize);
        let mut value: u32 = ptr::read_volatile(addr);
        value &= !(0x03 << shift);
        value |= config << shift;
        ptr::write_volatile(addr, value);
    }
}

pub fn signal_soft(interrupt: u32, core: u32) -> u32 {
    let mut value: u32 = (1 << core) << GICD_SGIR_CPU_SHIFT;
    value &= 0xff << GICD_SGIR_CPU_SHIFT;
    value |= interrupt & 0x0f;
    unsafe {
        ptr::write_volatile(GICD_SGIR, value);
    }
    value
}

pub struct GIC(u32);

impl GIC {
    pub const unsafe fn new(exception_num: u32) -> GIC {
        GIC(exception_num)
    }

    pub fn enable(&self) {
        set_core(self.0, crate::utils::current_core() as u32);
        enable(self.0)
    }

    pub fn disable(&self) {
        disable(self.0)
    }

    pub fn clear(&self) {
        clear(self.0)
    }
}
