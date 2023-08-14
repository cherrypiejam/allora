#![no_main]
#![no_std]
#![feature(alloc_error_handler)]
#![feature(allocator_api, nonnull_slice_from_raw_parts)]
#![feature(new_uninit)]

#![feature(custom_test_frameworks)]
#![test_runner(test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::collections::VecDeque;

pub mod device_tree;
pub mod gic;
pub mod mutex;
pub mod thread;
pub mod uart;
pub mod utils;
pub mod virtio;

mod apps;
mod collections;
mod mm;
mod exception;
mod timer;
mod kobject;
mod schedule;
mod switch;
mod lfchannel;

use virtio::VirtIORegs;

#[cfg(target_arch = "aarch64")]
global_asm!(
    include_str!("boot.S"),
    include_str!("exception.S"),
    include_str!("switch.S"),
);

use core::fmt::Write;
use core::panic::PanicInfo;
use core::arch::{asm, global_asm};
// use core::time::Duration;

// use mm::arena;
// use mm::page_tree;
use mm::PAGE_SIZE;


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

fn get_interrupt(irq_type: usize, irq: usize) -> u32 {
    if irq_type == 0 {
        // SPI
        32 + (irq as u32)
    } else {
        // irq_type == 1, PPI
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

fn interrupts_for_node(node: &device_tree::Node) -> Option<Vec<u32>> {
    node.prop_by_name("interrupts").map(|interrupt| {
        let mut interrupts: Vec<u32> = Vec::new();
        let (mut irq_type, rest) = regs_to_usize(interrupt.value, 1);
        let (mut irq, mut rest) = regs_to_usize(rest, 1);
        interrupts.push(get_interrupt(irq_type, irq));
        loop {
            if rest.len() > 4 { // Stop at 0, 0, 15, 4
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

#[global_allocator]
static ALLOCATOR: linked_list_allocator::LockedHeap = linked_list_allocator::LockedHeap::empty();

// LEAK: must be wait-free
static KOBJECTS: mutex::Mutex<Option<(Vec<kobject::KObjectMeta>, usize)>> = mutex::Mutex::new(None);
static READY_LIST: mutex::Mutex<Option<VecDeque<kobject::ThreadRef>>> = mutex::Mutex::new(None);


struct ResourceBlock {
    time_slices: [Option<kobject::ThreadRef>; 2],
    hand: usize,
}
static RESBLOCKS: mutex::Mutex<Option<(Vec<ResourceBlock>, usize)>> = mutex::Mutex::new(None);


static UART: mutex::Mutex<Option<uart::UART>> = mutex::Mutex::new(None);

#[no_mangle]
pub extern "C" fn kernel_main(dtb: &device_tree::DeviceTree, _start_addr: u64, _ttbr0_el1: u64, _x3: u64) {
    gic::init();

    // static UART: mutex::Mutex<Option<uart::UART>> = mutex::Mutex::new(None);

    static BLK: mutex::Mutex<Option<virtio::VirtIOBlk>> = mutex::Mutex::new(None);
    static ENTROPY: mutex::Mutex<Option<virtio::VirtIOEntropy>> = mutex::Mutex::new(None);
    static NET: mutex::Mutex<Option<virtio::VirtIONet>> = mutex::Mutex::new(None);

    let mut hstart = 0;
    let mut hsize = 0;

    let mut mem_start = 0;
    let mut mem_size = 0;

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

                        hstart = heap_start;
                        hsize = size;

                        ALLOCATOR.lock().init(heap_start, size / 2);
                        mem_start = mm::page_align_up(heap_start + size / 2);
                        mem_size = mm::page_align_down(size / 2 / 2); // FIXME: size isn't the
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

    READY_LIST.lock().replace(VecDeque::new());
    RESBLOCKS.lock().replace((Vec::new(), 0));

    #[cfg(test)]
    test_main();

    use alloc::format;
    debug("Booting allora...");
    debug(&format!("starting address: {:#x}", _start_addr));

    // Initialize kernel objects
    use kobject::{KObjectMeta, KObjectRef, Container, Thread, Label, THREAD_NPAGES};
    use mm::page_tree::PageTree;
    use mm::{page_align_up, page_align_down};

    let npages = page_align_down(mem_size) / PAGE_SIZE;
    KOBJECTS.lock().replace((
        Vec::with_capacity(npages),
        page_align_up(mem_start) / PAGE_SIZE,
    ));
    (0..npages)
        .for_each(|_| {
            KOBJECTS.map(|(ks, _)| ks.push(KObjectMeta::empty()));
        });

    debug(&format!("heap_start: {:#x}, heap_size: {:#x}, mem_start: {:#x}, mem_size: {:#x}", hstart, hsize, mem_start, mem_size));

    // let mut page_tree = unsafe { PageTree::new(mem_start, PAGE_SIZE * 512) };
    let mut page_tree = unsafe { PageTree::new(mem_start, PAGE_SIZE * 20) };

    // create the root container
    let lb_page = page_tree.get().unwrap();
    let lb_ref = unsafe {
        Label::create(lb_page, "T,F")
    };
    let ct_page = page_tree.get().unwrap();
    let root_ct_ref = unsafe {
        let ct_ref = Container::create(ct_page, KObjectRef::new(0));
        ct_ref.map_meta(move |ct| {
            ct.free_pages = page_tree;
            ct.label = Some(lb_ref);
        });
        ct_ref
    };
    lb_ref.map_meta(|lb| lb.parent = Some(root_ct_ref));


    // init the main thread
    let lb_slot = root_ct_ref.as_mut().get_slot().unwrap();
    let lb_page_id = root_ct_ref.map_meta(|m| m.free_pages.get()).unwrap().unwrap();
    let lb_ref = unsafe {
        Label::create(lb_page_id, "T,F")
    };
    root_ct_ref.as_mut().set_slot(lb_slot, lb_ref);

    let th_slot = root_ct_ref.as_mut().get_slot().unwrap();
    let th_page_id = root_ct_ref.map_meta(|m| m.free_pages.get_multiple(THREAD_NPAGES)).unwrap().unwrap();

    let main_th_ref = unsafe {
        let th_ref = Thread::create(th_page_id, || {});
        th_ref.map_meta(|th| th.label = Some(lb_ref));
        thread::init_thread(th_ref.as_ptr());
        th_ref
    };

    root_ct_ref.as_mut().set_slot(th_slot, main_th_ref);


    exception::with_intr_disabled(move || {

        let mut rb = ResourceBlock {
            time_slices: [None, None],
            hand: 0,
        };
        rb.time_slices[0] = Some(thread::spawn_thref(
            root_ct_ref,
            "T,F",
            || cpu_idle_debug("idling"),
        ));
        rb.time_slices[1] = Some(thread::spawn_thref(
            root_ct_ref,
            "T,F",
            || cpu_idle_debug("idling"),
        ));

        RESBLOCKS.map(|(rbs, _)| rbs.push(rb));


        // let mut rb = ResourceBlock {
            // time_slices: [None, None],
            // hand: 0,
        // };
        // rb.time_slices[0] = Some(thread::spawn_thref(
            // root_ct_ref,
            // || cpu_idle_debug("idling"),
        // ));
        // rb.time_slices[1] = Some(thread::spawn_thref(
            // root_ct_ref,
            // || cpu_idle_debug("idling"),
        // ));

        // RESBLOCKS.map(|(rbs, _)| rbs.push(rb));
    });

    // READY_LIST.map(|l| { (0..2).for_each(|i| l.push_back(rb.time_slices[i].as_ref().unwrap().clone())) });

    cpu_idle();

}


pub fn cpu_idle() -> ! {
    loop {
        // ensure CPU waking up first
        exception::with_intr_disabled(|| unsafe {
            asm!("wfi");
        });
    }
}

pub fn cpu_idle_debug(msg: &str) -> ! {
    loop {
        debug(msg);
        exception::with_intr_disabled(|| unsafe {
            asm!("wfi");
        });
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

#[cfg(test)]
fn test_runner(tests: &[&dyn Testable]) {
    exception::with_intr_disabled(|| {
        // It is single threaded anyway, let's disable interrupts.
        let mut uart = unsafe { uart::UART::new(0x0900_0000 as _, gic::GIC::new(uart::IRQ)) };
        for test in tests {
            test.run(&mut uart);
        }
        unsafe { system_off() }
    });
}

trait Testable {
    fn run(&self, uart: &mut uart::UART);
}

impl<T: Fn()> Testable for T {
    fn run(&self, uart: &mut uart::UART) {
        let _ = write!(uart, "{}...\t", core::any::type_name::<T>());
        self();
        let _ = writeln!(uart, "[ok]");
    }
}


pub fn debug(msg: &str) {
    exception::with_intr_disabled(|| {
        UART.map(|uart| {
            let _ = write!(uart, "DEBUG @ Thread {:#x}: {}\n",
                           thread::current_thread().map(|t| mm::pgid!(t as *const kobject::Thread as usize)).unwrap_or(0),
                           msg);
        });
    })
}


// pub fn debug(msg: &str) {
    // let a = exception::interrupt_mask_get();
    // exception::with_intr_disabled(|| {
        // UART.map(|uart| {
            // let _ = write!(uart, "DEBUG {}{} {:b} @ Thread {:#x}: {}\n",
                           // utils::current_core(),
                           // utils::current_el(),
                           // a >> 6,
                           // thread::current_thread().map(|t| mm::pgid!(t as *const kobject::Thread as usize)).unwrap_or(0),
                           // msg,
                           // );
        // });
    // })
// }

