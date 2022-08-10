use alloc::boxed::Box;
use core::sync::atomic::{AtomicU16, Ordering};

use crate::gic;
use crate::mutex::Mutex;

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

// static USED_CPUS: AtomicU16 = AtomicU16::new(!0b110);
static USED_CPUS: AtomicU16 = AtomicU16::new(!0b1110);

pub fn spawn<F: 'static + FnMut()>(mut f: F) {
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
            unsafe { cpu_off(next_cpu) };
        }),
    }));
    unsafe {
        cpu_on(next_cpu, conf as *mut _);
    }
}

// use crate::mutex::Mutex;
use crate::uart::UART;
use core::fmt::Write;
pub fn spawnf<F: 'static + FnMut()>(mut f: F, uart: &Mutex<Option<UART>>, count: i32)
{
    // Wait until there is a free CPU in the bit map
    // XXX if ordering is relaxed, how to make sure next_cpu is the used one
    let mut used_cpus = USED_CPUS.load(Ordering::Relaxed);
    let mut next_cpu;
    uart.map(|u| write!(u, "---------spawn{}\n", count));
    loop {
        loop {
        // uart.map(|u| write!(u, "----used_cpus: {:#b}\n", used_cpus));
            if used_cpus != !0 {
                break;
            }
            used_cpus = USED_CPUS.load(Ordering::Relaxed);
        }
        next_cpu = used_cpus.trailing_ones() as usize;
        // FIXME seems wrong? `used_cpus | (used_cpus << next_cpu)`
        // let new_used_cpus = used_cpus | (used_cpus << next_cpu);
        let new_used_cpus = used_cpus | (0b1 << next_cpu);
        uart.map(|u| write!(u, "{}      next_cpu: {}\n", count, next_cpu));
        uart.map(|u| write!(u, "{}     used_cpus: {:#b}\n", count, used_cpus));
        uart.map(|u| write!(u, "{} new_used_cpus: {:#b}\n", count, new_used_cpus));

        if let Err(uc) =
            USED_CPUS.compare_exchange(used_cpus, new_used_cpus, Ordering::SeqCst, Ordering::SeqCst)
        {
            used_cpus = uc;
        } else {
            used_cpus = new_used_cpus;
            break;
        }
    }

    uart.map(|u| write!(u, "{} new_used_cpus: {:#b}!!\n", count, used_cpus));
    uart.map(|u| write!(u, "{}              : {}!!\n", count, next_cpu));
    let conf = Box::into_raw(Box::new(Thread {
        main: thread_start,
        stack: Box::new([0; 1024]),
        userdata: Box::new(move || {
            gic::init();
            f();
            loop {
                // 110101111
                //      ^321
                // 110101111 & !(110101111000)
                // 110101111 &  (001010000111)
                //    110101111
                // 001010000111
                // ...010000111
                // let new_used_cpus = used_cpus & !(used_cpus << next_cpu);
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
            unsafe { cpu_off(next_cpu) };
        }),
    }));
    unsafe {
        cpu_on(next_cpu, conf as *mut _);
    }
}
