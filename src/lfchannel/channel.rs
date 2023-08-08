use super::list;

use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use core::cell::Cell;

use alloc::boxed::Box;
use alloc::vec::Vec;


#[derive(Clone)]
struct Entry<T: Clone> {
    msg: T,
    version: u64,
}

struct Channel<T: Clone> {
    chan: list::List<Entry<T>>,
    version: AtomicU64,
    destroy: AtomicBool,
}

impl<T: Clone> Channel<T> {
    fn new() -> Self {
        Channel {
            chan: list::List::<Entry<T>>::new(),
            version: AtomicU64::new(0),
            destroy: AtomicBool::new(false),
        }
    }
}

pub fn channel<T: Clone>() -> (Sender<T>, Receiver<T>) {
    let channel = Box::into_raw(Box::new(Channel::new()));
    let s = Sender { channel };
    let r = Receiver { channel };
    (s, r)
}

pub struct Sender<T: Clone> {
    channel: *mut Channel<T>,
}

impl<T: Clone> Sender<T> {
    pub fn send(&self, msg: T) {
        let channel = unsafe { &*self.channel };
        let version = channel.version.fetch_add(1, Ordering::Relaxed) + 1;
        let entry = Entry { msg, version };
        channel.chan.push(entry);
    }
}

unsafe impl<T: Send + Clone> Send for Sender<T> {}

pub struct Receiver<T: Clone> {
    channel: *const Channel<T>,
}

unsafe impl<T: Send + Clone> Send for Receiver<T> {}

pub struct WrapperReceiver<T: Clone> {
    receiver: Receiver<T>,
    last_seen: Cell<u64>,
}

impl<T: Clone> WrapperReceiver<T> {
    pub fn new(receiver: Receiver<T>) -> Self {
        Self { receiver, last_seen: Cell::new(0) }
    }

    pub fn recv(&self) -> Option<Vec<T>> {
        let channel = unsafe { &*self.receiver.channel };
        let destroyed = channel.destroy.load(Ordering::Relaxed);
        if destroyed {
            None
        } else {
            let last_seen = self.last_seen.get();
            if channel.chan.first().version <= last_seen {
                None
            } else {
                let vec = channel.chan.to_vec();
                self.last_seen.set(vec.first().unwrap().version);
                Some(
                    vec
                    .into_iter()
                    .filter(|x| x.version > last_seen)
                    .map(|x| x.msg)
                    .collect::<Vec<T>>()
                )
            }
        }
    }
}

unsafe impl<T: Send + Clone> Send for WrapperReceiver<T> {}
// impl<T> !Sync for WrapperReceiver<T> {}


// #[cfg(test)]
// mod tests {
    // use super::*;

    // #[test]
    // fn channel_single_thread() {
        // let (tx, rx) = channel::<i32>();
        // let rx = WrapperReceiver::new(rx);

        // tx.send(1337);
        // tx.send(1338);
        // tx.send(1339);
        // assert_eq!(Some(vec![1339, 1338, 1337]), rx.recv());

        // tx.send(1340);
        // tx.send(1341);
        // assert_eq!(Some(vec![1341, 1340]), rx.recv());
        // assert_eq!(None, rx.recv());
    // }

    // #[test]
    // fn channel_two_threads() {
        // use std::sync::{Arc, Mutex, Condvar};

        // let (tx, rx) = channel::<i32>();

        // tx.send(1337);
        // tx.send(1338);
        // tx.send(1339);

        // let pair = Arc::new((Mutex::new(false), Condvar::new()));
        // let pair2 = pair.clone();

        // let handle = std::thread::spawn(move || {
            // let (lock, cvar) = &*pair2;
            // let rx = WrapperReceiver::new(rx); // In LARPS, the wrapper uses THIS thread's local
                                               // // memory. This is why it is initualized here.
            // assert_eq!(Some(vec![1339, 1338, 1337]), rx.recv());

            // // Done the first round
            // {
                // let mut state = lock.lock().unwrap();
                // *state = true;
                // cvar.notify_one();
            // }

            // // Wait for the second round
            // {
                // let mut state = lock.lock().unwrap();
                // while *state {
                    // state = cvar.wait(state).unwrap();
                // }
            // }
            // assert_eq!(Some(vec![1341, 1340]), rx.recv());
            // assert_eq!(None, rx.recv());
        // });

        // let (lock, cvar) = &*pair;
        // {
            // let mut state = lock.lock().unwrap();
            // while !*state {
                // state = cvar.wait(state).unwrap();
            // }
        // }

        // tx.send(1340);
        // tx.send(1341);
        // {
            // let mut state = lock.lock().unwrap();
            // *state = false;
            // cvar.notify_one();
        // }
        // let _ = handle.join();
    // }

// }
