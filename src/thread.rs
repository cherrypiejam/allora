use alloc::boxed::Box;
use core::fmt::Write;
use core::sync::atomic::{AtomicU16, Ordering};
use core::time::Duration;
use core::arch::asm;
use core::mem;
use core::ptr;

// use core::marker::PhantomPinned;

use crate::{gic, timer, WAIT_LIST, ALLOCATOR};
use crate::utils::current_core;
use crate::arena::{LabeledArena, RawLabeledArena};
use crate::exception::{self, InterruptDisabled};

#[repr(C)]
struct Thread<T: Sized> {
    main: extern "C" fn(Box<Self>),
    stack: Box<[usize; 1024], RawLabeledArena>,
    userdata: T,
    arena: Option<LabeledArena>,
    // arena: Option<Arena>,
    // label: Label,
}

// impl<'a, T: Sized> Drop for Thread<T> {
    // fn drop(&mut self) {
        // // TODO add arena back to the label-specific allocator
        // crate::UART.map(|u| u.write_fmt(format_args!("{:?}\n", self.arena.take().unwrap())));
        // // self.arena
            // // .take()
            // // .map(|arena| {
                // // ALLOCATOR_LIST.map(|alist| {
                    // // alist.iter()
                        // // .find(|a| a.label() == arena.label())
                        // // .map(|a| {
                            // // a.join(arena);
                        // // });
                // // });
            // // });
        // crate::UART.map(|u| u.write_fmt(format_args!("in drop end\n")));
    // }
// }

extern "C" {
    fn cpu_on(core: usize, main: *mut core::ffi::c_void) -> isize;
    fn cpu_off();
}

extern "C" fn thread_start(mut conf: Box<Thread<Box<dyn FnMut(), RawLabeledArena>>>) {
    (conf.userdata)()
}

static USED_CPUS: AtomicU16 = AtomicU16::new(!0b1110);

fn prepare<F: 'static + FnMut()>(mut f: F, arena: Option<LabeledArena>) -> (usize, *mut u8) {
    // Wait until there is a free CPU in the bit map
    let mut used_cpus = USED_CPUS.load(Ordering::Relaxed);
    let mut next_cpu;
    loop {
        loop {
            if used_cpus != !0 {
                break;
            }
            used_cpus = USED_CPUS.load(Ordering::Relaxed);
        }
        next_cpu = used_cpus.trailing_ones() as usize;
        let new_used_cpus = used_cpus | (1 << next_cpu);
        if let Err(uc) =
            USED_CPUS.compare_exchange(used_cpus, new_used_cpus, Ordering::SeqCst, Ordering::SeqCst)
        {
            used_cpus = uc;
        } else {
            break;
        }
    }

    let conf = {
        // It is OK to put conf on the global heap.
        let mut conf = Box::new(Thread {
            main: thread_start,
            stack: unsafe {
                let boxed = mem::MaybeUninit::<Box<[usize; 1024], RawLabeledArena>>::uninit();
                boxed.assume_init()
            },
            userdata: unsafe {
                let boxed = mem::MaybeUninit::<Box<dyn FnMut(), RawLabeledArena>>::uninit();
                boxed.assume_init()
            },
            arena,
        });
        let raw_conf = conf.as_mut() as *mut _;
        let raw_arena = RawLabeledArena::from(conf.arena.as_ref().unwrap_or_else(|| &ALLOCATOR));
        let stack = Box::new_in([0; 1024], raw_arena);
        let userdata = Box::new_in(move || {
            gic::init();
            exception::load_table();
            init_thread(raw_conf as *const _);
            f();
            cpu_off_graceful(); // early-termination channel
        }, raw_arena);

        unsafe {
            ptr::write(&mut conf.stack, stack);
            ptr::write(&mut conf.userdata, userdata);
        }

        Box::into_raw(conf)
    };
    (next_cpu, conf as *mut _)
}

pub fn cpu_off_graceful() {
    InterruptDisabled::with(|| {
        WAIT_LIST.map(|wlist| wlist.retain(|w| w.cpu != current_core()));
    });

    let cpu = current_core();
    let mut used_cpus = USED_CPUS.load(Ordering::Relaxed);
    loop {
        let new_used_cpus = used_cpus & !(1 << cpu);
        if let Err(uc) = USED_CPUS.compare_exchange(
            used_cpus,
            new_used_cpus,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            used_cpus = uc;
        } else {
            break;
        }
    }

    unsafe {

        crate::UART.map(|u| u.write_fmt(format_args!("before drop\n")));
        current_thread()
            .map(|curr| {
                crate::UART.map(|u| u.write_fmt(format_args!("before drop 2\n")));
                drop(Box::from_raw(curr as *mut _)); // drop explicitly
                crate::UART.map(|u| u.write_fmt(format_args!("after drop 0\n")));
            });

        crate::UART.map(|u| u.write_fmt(format_args!("after drop\n")));
        init_thread(0 as *const _);
        crate::UART.map(|u| u.write_fmt(format_args!("after drop 2\n")));
        cpu_off();
    }
}

pub fn spawn<F: 'static + FnMut()>(f: F) {
    let (next_cpu, conf) = prepare(f, None);
    unsafe {
        while // TODO handle all corner cases
            cpu_on(next_cpu, conf as *mut _)
            < 0
        {}
    }
}

#[derive(Clone, Copy)]
pub struct Task {
    pub cpu: usize,
    pub alive_until: u64,
}

impl Task {
    pub fn new(cpu: usize, lifetime: Duration) -> Self {
        let alive_until = timer::current_ticks() + timer::convert_to_ticks(lifetime);
        Self { cpu, alive_until }
    }
}

pub fn launch<F: 'static + FnMut()>(arena: Option<LabeledArena>, lifetime: Duration, f: F) {
    let (next_cpu, conf) = prepare(f, arena);
    let task = Task::new(next_cpu, lifetime);
    unsafe {
        while
            cpu_on(next_cpu, conf as *mut _)
            < 0
        {}
    }

    InterruptDisabled::with(|| {
        WAIT_LIST.map(|wlist| {
            wlist.push(task);
            wlist.sort_by(|a, b| {
                b.alive_until.partial_cmp(&a.alive_until).unwrap()
            })
        });
    });
}

fn init_thread(conf: *const u8) {
    unsafe {
        asm!("msr TPIDR_EL1, {}",
             in(reg) conf as u64);
    }
}

fn current_thread<'a>() -> Option<&'a mut Thread<Box<dyn FnMut(), RawLabeledArena>>> {
    let conf: u64;
    unsafe {
        asm!("mrs {}, TPIDR_EL1",
             out(reg) conf);
    }
    if conf == 0 {
        None
    } else {
        Some(unsafe {
            &mut *(conf as *mut _)
        })
    }
}

pub fn local_arena<'a>() -> Option<&'a LabeledArena> {
    current_thread()
        .and_then(|curr| {
            curr.arena.as_ref()
        })
}
