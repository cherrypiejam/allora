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
mod label;
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
use core::time::Duration;

use mm::arena;
use mm::page_tree;
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

// Top-level memory allocator
static MEM_POOL: mutex::Mutex<Option<mm::page::PageMap>> = mutex::Mutex::new(None);

// Label-specific memory allocator
static LOCAL_MEM_POOL: mutex::Mutex<Option<Vec<mm::page::LabeledPageSet>>> = mutex::Mutex::new(None);

// LEAK: must be wait-free
static KOBJECTS: mutex::Mutex<Option<(Vec<kobject::KObjectMeta>, usize)>> = mutex::Mutex::new(None);
static READY_LIST: mutex::Mutex<Option<Vec<kobject::KObjectRef>>> = mutex::Mutex::new(None);

static UART: mutex::Mutex<Option<uart::UART>> = mutex::Mutex::new(None);
const APP_ENABLE: bool = false;

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

                        // let pool_start = memory::page_align_up(heap_start + size / 2);
                        // let pool_end = memory::page_align_down(heap_start + size);
                        // let pool_size = pool_end - pool_start;
                        // let heap_size = pool_start - heap_start;

                        ALLOCATOR.lock().init(heap_start, size / 2);

                        // MEM_POOL.lock().replace(memory::page::PageMap::new(pool_start, pool_size));

                        mem_start = mm::page_align_up(heap_start + size / 2);
                        mem_size = mm::page_align_down(size / 2 / 2); // FIXME: size isn't the
                                                                          // end of the heap

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

    READY_LIST.lock().replace(Vec::new());

    #[cfg(test)]
    test_main();

    // UART.map(|u| writeln!(u, "EL: {}, CORE: {}, _start_addr: {:#x}", utils::current_el(), utils::current_core(), _start_addr));

    // Initialize kernel objects
    use kobject::{KObjectKind, KObjectMeta, Container, Thread};
    use mm::koarena::KObjectArena;
    use mm::page_tree::PageTree;
    use mm::{page_align_up, page_align_down, pa};

    KOBJECTS.lock().replace((
        Vec::with_capacity(page_align_down(mem_size) / PAGE_SIZE),
        page_align_up(mem_start) / PAGE_SIZE,
    ));
    (0..(mem_size/PAGE_SIZE))
        .for_each(|i| {
            KOBJECTS.map(|(ks, ofs)| ks.push(KObjectMeta {
                id: *ofs + i,
                parent: None,
                label: None,
                alloc: KObjectArena::empty(),
                kind: KObjectKind::None,
                free_pages: PageTree::empty(),
            }));
        });


    // UART.map(|u| writeln!(u, "heap_start: {}, heap_size: {}, mem_start: {}, mem_size: {}", hstart, hsize, mem_start, mem_size));

    let mut page_tree = unsafe { PageTree::new(mem_start, PAGE_SIZE * 512) };
    let page = page_tree.get().unwrap();

    use kobject::IsKObjectRef;

    let root_ct_ref = unsafe {
        let ct_ref = Container::create(page, 0);
        ct_ref.map_meta(move |ct_meta| {
            ct_meta.free_pages = page_tree;
        });
        ct_ref
    };


    let root_ct = unsafe { (pa!(root_ct_ref) as *mut Container).as_mut().unwrap() };
    // UART.map(|u| writeln!(u, "root container slots: {:?}", root_ct.slots));


    let th_slot = root_ct.get_slot().unwrap();
    let th_page = root_ct_ref.map_meta(|m| m.free_pages.get()).unwrap().unwrap();
    root_ct.set_slot(th_slot, th_page);

    let main_th_ref = unsafe {
        let th_ref = Thread::create(th_page, || {});
        thread::init_thread(pa!(th_ref) as *mut Thread);
        th_ref
    };


    thread::spawn(root_ct, || {
        loop {
            crate::exception::interrupt_disable();
            UART.map(|uart| {
                let _ = write!(uart, "Running from core {}, thread 2\n", utils::current_core());
            });
            crate::exception::interrupt_enable();
        }
    });

    cpu_idle();

}

pub fn cpu_idle() -> ! {
    loop {
        exception::with_intr_disabled(|| unsafe {
            // ensure CPU waking up first
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
