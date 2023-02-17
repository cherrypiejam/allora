use alloc::boxed::Box;
use core::sync::atomic::{AtomicU16, Ordering};
use core::time::Duration;
use core::arch::asm;

use crate::{gic, timer, exception, TASK_LIST};
use crate::utils::current_core;
use crate::arena::Arena;

#[repr(C)]
struct Thread<T: Sized> {
    main: extern "C" fn(Box<Self>),
    stack: Box<[usize; 1024]>,
    userdata: T,
}

extern "C" {
    fn cpu_on(core: usize, main: *mut core::ffi::c_void) -> isize;
    fn cpu_off();
}

extern "C" fn thread_start(mut conf: Box<Thread<Box<dyn FnMut()>>>) {
    (conf.userdata)()
}

static USED_CPUS: AtomicU16 = AtomicU16::new(!0b1110);

fn prepare<F: 'static + FnMut()>(mut f: F, wait_after_finish: bool) -> (usize, *mut u8) {
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

    let conf = Box::into_raw(Box::new(Thread {
        main: thread_start,
        stack: Box::new([0; 1024]),
        userdata: Box::new(move || {
            gic::init();
            exception::load_table();
            // unsafe { gic::GIC::new(0).enable() };

            f();

            if !wait_after_finish {
                cpu_off_graceful();
            } else {
                loop {
                    unsafe { asm!("wfi"); }
                }
            }
        }),
    }));
    (next_cpu, conf as *mut _)
}

pub fn cpu_off_graceful() {
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
    unsafe { cpu_off(); }
}

pub fn spawn<F: 'static + FnMut()>(f: F) {
    let (next_cpu, conf) = prepare(f, false);
    unsafe {
        while // TODO handle all corner cases
            cpu_on(next_cpu, conf as *mut _)
            != -4
        {}
    }
}

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

pub fn launch<F: 'static + FnMut()>(arena: Option<Arena>, lifetime: Duration, f: F) {
    let (next_cpu, conf) = prepare(f, true);
    let task = Task::new(next_cpu, lifetime);
    unsafe {
        while
            cpu_on(next_cpu, conf as *mut _)
            != -4
        {}
    }
    exception::interrupt_disable();
    TASK_LIST.map(|t| {
        t.push(task);
        t.sort_by(|a, b| {
            b.alive_until.partial_cmp(&a.alive_until).unwrap()
        })
    });
    exception::interrupt_enable();
}

