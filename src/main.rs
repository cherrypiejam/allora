#![no_main]
#![no_std]
#![feature(alloc_error_handler)]
#![feature(const_mut_refs)]

extern crate alloc;
use alloc::boxed::Box;

pub mod device_tree;
pub mod gic;
pub mod mutex;
pub mod thread;
pub mod uart;
pub mod utils;
pub mod virtio;

mod apps;
mod allocator;
mod exception;
mod timer;

use virtio::VirtIORegs;

#[cfg(target_arch = "aarch64")]
global_asm!(include_str!("boot.S"));
global_asm!(include_str!("exception.S"));

use core::fmt::Write;
use core::panic::PanicInfo;
use core::arch::{asm, global_asm};

extern "C" {
    static HEAP_START: usize;
    fn system_off() -> !;
}

fn null_terminated_str(bytes: &[u8]) -> &[u8] {
    if bytes[bytes.len() - 1] == 0 {
        &bytes[..bytes.len() - 1]
    } else {
        bytes
    }
}

fn regs_to_usize(regs: &[u8], cell_size: usize) -> (usize, &[u8]) {
    let mut result = 0;
    let (work, rest) = regs.split_at(cell_size * 4);
    for chunk in work.chunks(4) {
        let mut c = [0; 4];
        c.copy_from_slice(chunk);
        result = result << 32 | (u32::from_be_bytes(c) as usize);
    }
    (result, rest)
}

// FIXME ref DEN0024A_v8_architecture_PG.pdf:139
// SGI -> 0-15, PPI -> 16-31, SPI -> 32-1020
fn get_interrupt(irq_type: usize, irq: usize) -> u32 {
    if irq_type == 0 {
        // IRQ
        32 + (irq as u32)
    } else {
        // irq_type == 1, SPI
        16 + (irq as u32)
    }
}

fn interrupt_for_node(node: &device_tree::Node) -> Option<u32> {
    node.prop_by_name("interrupts").map(|interrupt| {
        let (irq_type, rest) = regs_to_usize(interrupt.value, 1);
        let (irq, _rest) = regs_to_usize(rest, 1);
        get_interrupt(irq_type, irq)
    })
}

use alloc::vec;
fn interrupts_for_node(node: &device_tree::Node) -> Option<vec::Vec<u32>> {
    node.prop_by_name("interrupts").map(|interrupt| {
        let mut interrupts: vec::Vec<u32> = vec![];
        let (mut irq_type, rest) = regs_to_usize(interrupt.value, 1);
        let (mut irq, mut rest) = regs_to_usize(rest, 1);
        interrupts.push(get_interrupt(irq_type, irq));
        loop {
            if rest.len() > 4 { // ignore 0, 0, 15, 4
                (irq_type, rest) = regs_to_usize(&rest[4..], 1);
                (irq, rest) = regs_to_usize(rest, 1);
                interrupts.push(get_interrupt(irq_type, irq));
            } else {
                break
            }
        }
        interrupts
    })
}

// fn interrupt_for_node_nth(node: &device_tree::Node, n: u32) -> Option<u32> {
    // node.prop_by_name("interrupts").and_then(|interrupt| {
        // let (mut irq_type, rest) = regs_to_usize(interrupt.value, 1);
        // let (mut irq, mut rest) = regs_to_usize(rest, 1);
        // for _ in 0..n {
            // if rest.len() > 4 { // ignore 0, 0, 15, 4
                // (irq_type, rest) = regs_to_usize(&rest[4..], 1);
                // (irq, rest) = regs_to_usize(rest, 1);
            // } else {
                // return None
            // }
        // }
        // if irq_type == 0 {
            // // IRQ
            // Some(32 + (irq as u32))
        // } else {
            // // irq_type == 1, SPI
            // Some(16 + (irq as u32))
        // }
    // })
// }

#[global_allocator]
static ALLOCATOR: mutex::Mutex<allocator::FixedBlockAllocator> =
    mutex::Mutex::new(allocator::FixedBlockAllocator::new());
// static ALLOCATOR: linked_list_allocator::LockedHeap = linked_list_allocator::LockedHeap::empty();

static UART: mutex::Mutex<Option<uart::UART>> = mutex::Mutex::new(None);

#[no_mangle]
pub extern "C" fn kernel_main(dtb: &device_tree::DeviceTree) {
    gic::init();

    // static UART: mutex::Mutex<Option<uart::UART>> = mutex::Mutex::new(None);

    static BLK: mutex::Mutex<Option<virtio::VirtIOBlk>> = mutex::Mutex::new(None);
    static ENTROPY: mutex::Mutex<Option<virtio::VirtIOEntropy>> = mutex::Mutex::new(None);
    static NET: mutex::Mutex<Option<virtio::VirtIONet>> = mutex::Mutex::new(None);

    if let Some(root) = dtb.root() {
        let size_cell = root
            .prop_by_name("#size-cells")
            .map(|sc| {
                let mut buf = [0; 4];
                buf.copy_from_slice(sc.value);
                u32::from_be_bytes(buf) as usize
            })
            .unwrap_or(2);
        let address_cell = root
            .prop_by_name("#address-cells")
            .map(|sc| {
                let mut buf = [0; 4];
                buf.copy_from_slice(sc.value);
                u32::from_be_bytes(buf) as usize
            })
            .unwrap_or(2);

        for memory in root.children_by_prop("device_type", |prop| prop.value == b"memory\0") {
            if let Some(reg) = memory.prop_by_name("reg") {
                let (addr, rest) = regs_to_usize(reg.value, address_cell);
                let (size, _) = regs_to_usize(rest, size_cell);
                unsafe {
                    let heap_start = &HEAP_START as *const _ as usize;
                    if heap_start >= addr {
                        ALLOCATOR.lock().init(heap_start, size);
                        break;
                    } else {
                        panic!("{:#x} {:#x}", addr, heap_start);
                    }
                }
            }
        }

        if let Some(chosen) = root.child_by_name("chosen") {
            chosen
                .prop_by_name("stdout-path")
                .map(|stdout_path| null_terminated_str(stdout_path.value))
                .filter(|stdout_path| stdout_path == b"/pl011@9000000")
                .map(|stdout_path| {
                    root.child_by_path(stdout_path).map(|stdout| {
                        let irq = interrupt_for_node(&stdout).unwrap_or(0) as u32;
                        if let Some(reg) = stdout.prop_by_name("reg") {
                            let (addr, rest) = regs_to_usize(reg.value, address_cell);
                            let (size, _) = regs_to_usize(rest, size_cell);
                            if size == 0x1000 {
                                let mut uart = UART.lock();
                                *uart =
                                    Some(unsafe { uart::UART::new(addr as _, gic::GIC::new(irq)) });
                            }
                        }
                    });
                });
        }

        exception::load_table();

        if let Some(timer) = root.child_by_name("timer") {
            if let Some(irq) = interrupts_for_node(&timer)
                .map(|irqs| {
                    irqs.into_iter().find(|&irq| irq == timer::EL1_PHYSICAL_TIMER)
                })
                .flatten() {
                timer::init_timer(unsafe { gic::GIC::new(irq) });
            }
        }

                // unsafe {
                    // let mut o: u32;
                    // asm!("mrs {:x}, CNTFRQ_EL0",
                         // out(reg) o);
                    // UART.map(|u| write!(u, "frq: {}\n", o));
                    // asm!("mrs {:x}, CNTP_TVAL_EL0",
                         // out(reg) o);
                    // UART.map(|u| write!(u, "tval: {}\n", o));
                // }
                // UART.map(|u| write!(u, "timer interrupt: {}\n", irq));

        // // print all
        // for c in root.children() {
            // use alloc::string::String;
            // let name = c.name
                // .iter()
                // .map(|&b| b as char)
                // .collect::<String>();
            // UART.map(|u| write!(u, "name: {}\n", name));
            // if let Some(t) = c.prop_by_name("device_type") {
                // let name = t.value
                    // .iter()
                    // .map(|&b| b as char)
                    // .collect::<String>();
                // UART.map(|u| write!(u, "device type: {}\n", name));
            // }
        // }



        for child in root.children_by_prop("compatible", |prop| prop.value == b"virtio,mmio\0") {
            if let Some(reg) = child.prop_by_name("reg") {
                let (addr, _rest) = regs_to_usize(reg.value, address_cell);
                let irq =
                    unsafe { crate::gic::GIC::new(interrupt_for_node(&child).unwrap_or(0) as u32) };
                if let Some(virtio) = unsafe { VirtIORegs::new(addr as *mut VirtIORegs<()>) } {
                    match virtio.device_id() {
                        virtio::DeviceId::Blk => {
                            let mut virtio_blk = BLK.lock();
                            *virtio_blk = unsafe {
                                Some(virtio::VirtIOBlk::new(
                                    &mut *(virtio as *mut _ as *mut _),
                                    Box::leak(Box::new(virtio::Queue::new())),
                                    irq,
                                ))
                            };
                        }
                        virtio::DeviceId::Entropy => {
                            let mut virtio_entropy = ENTROPY.lock();
                            *virtio_entropy = unsafe {
                                Some(virtio::VirtIOEntropy::new(
                                    &mut *(virtio as *mut _ as *mut _),
                                    Box::leak(Box::new(virtio::Queue::new())),
                                    irq,
                                ))
                            };
                        }
                        virtio::DeviceId::Net => {
                            let mut virtio_net = NET.lock();
                            *virtio_net = unsafe {
                                Some(virtio::VirtIONet::new(
                                    &mut *(virtio as *mut _ as *mut _),
                                    Box::leak(Box::new(virtio::Queue::new())),
                                    Box::leak(Box::new(virtio::Queue::new())),
                                    irq,
                                ))
                            };
                        }
                        _ => {}
                    }
                }
            }
        }
    }


    // UART.map(|uart| uart.write_bytes(b"Booting Allora...\n"));


    // let a = 0x0 as *mut usize;
    // for i in 0..1000000000 {
        // unsafe {
            // let b = a.add(i);
            // if *b > 0 {
                // UART.map(|uart| {
                    // let _ = write!(uart, "{}: {}\n", b as usize, *b);
                // });
            // }
        // }
    // }

    // thread::spawn(|| {
        // UART.map(|uart| {
            // let _ = write!(uart, "Running from core {}\n", utils::current_core());
        // });
        // let mut shell = apps::shell::Shell {
            // blk: &BLK,
            // entropy: &ENTROPY,
        // };
        // apps::shell::main(&UART, &mut shell);
    // });

    UART.map(|uart| write!(uart, "Booting Allora...{}\n", utils::current_core()));

    // // Debug
    // for i in 0..10 {
        // let a = i;
        // thread::spawn(move || {
            // UART.map(|uart| {
                // let _ = write!(uart, "{} Running from core {}\n", a, utils::current_core());
            // });
            // for j in 0..100000 {
                // let _a = j + 30;
            // }
        // });
    // }

    // thread::spawn(|| {
        // UART.map(|uart| {
            // let _ = write!(uart, "Running from core {}\n", utils::current_core());
        // });
        // NET.map(|mut net| {
            // let mut shell = apps::shell::Shell {
                // blk: &BLK,
                // entropy: &ENTROPY,
            // };
            // apps::net::Net { net: &mut net }.run(&mut shell)
        // });
    // });

    loop {
        unsafe {
            asm!("wfi");
        }
    }
}

#[panic_handler]
fn panic(panic_info: &PanicInfo<'_>) -> ! {
    let mut uart = unsafe { uart::UART::new(0x0900_0000 as _, gic::GIC::new(uart::IRQ)) };
    let _ = uart.write_fmt(format_args!("{}", panic_info));
    unsafe { system_off() }
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}
