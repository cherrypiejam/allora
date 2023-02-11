use alloc::boxed::Box;
use core::sync::atomic::{AtomicU16, Ordering};
use core::time::Duration;

use crate::gic;
use crate::arena::Arena;
use crate::timer;
use crate::exception;
use crate::TASK_LIST;

#[repr(C)]
struct Thread<T: Sized> {
    main: extern "C" fn(Box<Self>),
    stack: Box<[usize; 1024]>,
    userdata: T,
}

extern "C" {
    fn cpu_on(core: usize, main: *mut core::ffi::c_void);
    fn cpu_off(core: usize);
}

extern "C" fn thread_start(mut conf: Box<Thread<Box<dyn FnMut()>>>) {
    (conf.userdata)()
}

static USED_CPUS: AtomicU16 = AtomicU16::new(!0b1110);

fn prepare<F: 'static + FnMut()>(mut f: F, timed: bool) -> (usize, *mut u8) {
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
        let new_used_cpus = used_cpus | (0b1 << next_cpu);
        if let Err(uc) =
            USED_CPUS.compare_exchange(used_cpus, new_used_cpus, Ordering::SeqCst, Ordering::SeqCst)
        {
            used_cpus = uc;
        } else {
            used_cpus = new_used_cpus;
            break;
        }
    }

    let conf = Box::into_raw(Box::new(Thread {
        main: thread_start,
        stack: Box::new([0; 1024]),
        userdata: Box::new(move || {
            gic::init();
            f();
            loop {
                let new_used_cpus = used_cpus & !(0b1 << next_cpu);
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
            if !timed {
                unsafe { cpu_off(next_cpu); }
            }
        }),
    }));
    (next_cpu, conf as *mut _)
}

pub fn spawn<F: 'static + FnMut()>(f: F) {
    let (next_cpu, conf) = prepare(f, false);
    unsafe { cpu_on(next_cpu, conf as *mut _); }
}

pub struct Task {
    cpu: usize,
    pub alive_until: u64, // until this tick, cpu_off(core), to remove termination channel, no unsafe {
}

pub fn launch<F: 'static + FnMut()>(arena: Arena, lifetime: Duration, f: F) {
    let (next_cpu, conf) = prepare(f, true);
    unsafe { cpu_on(next_cpu, conf as *mut _); }
    let task = Task {
        cpu: next_cpu,
        alive_until: timer::current_ticks() + timer::convert_to_ticks(lifetime)
    };
    exception::interrupt_disable();
    TASK_LIST.map(|t| {
        t.push(task);
        t.sort_by(|a, b| {
            a.alive_until.partial_cmp(&b.alive_until).unwrap()
        })
    });
    exception::interrupt_enable();
}

