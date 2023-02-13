use alloc::boxed::Box;
use core::sync::atomic::{AtomicU16, Ordering, AtomicBool};
use core::time::Duration;
use alloc::sync::Arc;

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
    pub fn cpu_off();
}

extern "C" fn thread_start(mut conf: Box<Thread<Box<dyn FnMut()>>>) {
    (conf.userdata)()
}

static USED_CPUS: AtomicU16 = AtomicU16::new(!0b1110);

fn prepare<F: 'static + FnMut()>(mut f: F, hold: Option<Arc<AtomicBool>>) -> (usize, *mut u8) {
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
            crate::UART.map(|u| u.write_bytes(b"before something \n"));
            f();
            crate::UART.map(|u| u.write_bytes(b"after hahah\n"));
            hold.as_ref().map(|h| {
                while
                    h.compare_exchange(false, false, Ordering::SeqCst, Ordering::SeqCst)
                    != Ok(false)
                {}
            });
            crate::UART.map(|u| u.write_bytes(b"after 2 hahah\n"));

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
            unsafe { cpu_off(); }
        }),
    }));
    (next_cpu, conf as *mut _)
}

pub fn spawn<F: 'static + FnMut()>(f: F) {
    let (next_cpu, conf) = prepare(f, None);
    unsafe { cpu_on(next_cpu, conf as *mut _); }
}

#[derive(Debug)]
pub struct Task {
    pub cpu: usize,
    pub alive_until: u64, // until this tick, cpu_off(core), to remove termination channel, no unsafe {
    pub hold: Arc<AtomicBool>,
}

impl Task {
    pub fn new(cpu: usize, lifetime: Duration, hold: Arc<AtomicBool>) -> Self {
        let alive_until = timer::current_ticks() + timer::convert_to_ticks(lifetime);
        Self { cpu, alive_until, hold }
    }
}

pub fn launch<F: 'static + FnMut()>(arena: Option<Arena>, lifetime: Duration, f: F) {
    let hold = Arc::new(AtomicBool::new(true));
    let (next_cpu, conf) = prepare(f, Some(Arc::clone(&hold)));
    unsafe { cpu_on(next_cpu, conf as *mut _); }
    let task = Task::new(next_cpu, lifetime, hold);
    exception::interrupt_disable();
    TASK_LIST.map(|t| {
        t.push(task);
        t.sort_by(|a, b| {
            b.alive_until.partial_cmp(&a.alive_until).unwrap()
        })
    });
    exception::interrupt_enable();
}

