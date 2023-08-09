use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use core::cell::Cell;
use core::alloc::Allocator;

use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::alloc::Global;

use super::list::List;


#[derive(Clone)]
struct Entry<T: Clone> {
    msg: T,
    version: u64,
}

struct Channel<T: Clone, A: Allocator + Clone = Global> {
    chan: List<Entry<T>, A>,
    version: AtomicU64,
    destroy: AtomicBool,
}

impl<T: Clone, A: Allocator + Clone> Channel<T, A> {
    fn new_in(alloc: A) -> Self {
        Channel {
            chan: List::new_in(alloc),
            version: AtomicU64::new(0),
            destroy: AtomicBool::new(false),
        }
    }
}

pub fn channel<T: Clone>() -> (Sender<T>, Receiver<T>) {
    channel_in(Global)
}


pub fn channel_in<T: Clone, A: Allocator + Clone>(alloc: A) -> (Sender<T, A>, Receiver<T, A>) {
    let channel = Box::into_raw(Box::new_in(Channel::new_in(alloc.clone()), alloc));
    let s = Sender { channel };
    let r = Receiver { channel };
    (s, r)
}


//////////////
// Sender
//////////////

pub struct Sender<T: Clone, A: Allocator + Clone = Global> {
    channel: *mut Channel<T, A>,
}

impl<T: Clone, A: Allocator + Clone> Sender<T, A> {
    pub fn send(&self, msg: T) {
        let channel = unsafe { &*self.channel };
        let version = channel.version.fetch_add(1, Ordering::Relaxed) + 1;
        let entry = Entry { msg, version };
        channel.chan.push(entry);
    }
}

unsafe impl<T: Send + Clone, A: Send + Allocator + Clone> Send for Sender<T, A> {}


//////////////
// Inner Receiver
//////////////

pub struct Receiver<T: Clone, A: Allocator + Clone = Global> {
    channel: *const Channel<T, A>,
}

unsafe impl<T: Send + Clone, A: Send + Allocator + Clone> Send for Receiver<T, A> {}

//////////////
// Receiver: inner receiver could be in a read-only memory to the current receiver; This wraps it in a writable memory
//////////////

pub struct WrapperReceiver<T: Clone, A: Allocator + Clone> {
    receiver: Receiver<T, A>,
    last_seen: Cell<u64>,
}

impl<T: Clone, A: Allocator + Clone> WrapperReceiver<T, A> {
    pub fn new(receiver: Receiver<T, A>) -> Self {
        Self { receiver, last_seen: Cell::new(0) }
    }

    pub fn recv(&self) -> Option<Vec<T>> {
        self.recv_in(Global)
    }

    pub fn recv_in<B: Allocator + Clone>(&self, alloc: B) -> Option<Vec<T, B>> {
        let channel = unsafe { &*self.receiver.channel };
        let destroyed = channel.destroy.load(Ordering::Relaxed);
        if destroyed {
            None
        } else {
            let last_seen = self.last_seen.get();
            if channel.chan.first().version <= last_seen {
                None
            } else {
                let history = channel.chan.to_vec_in(alloc.clone());
                self.last_seen.set(history.first().unwrap().version);
                let mut vec = Vec::new_in(alloc);
                history
                    .into_iter()
                    .filter(|x| x.version > last_seen)
                    .map(|x| x.msg)
                    .for_each(|msg| {
                        vec.push(msg);
                    });
                Some(vec)
            }
        }
    }
}

unsafe impl<T: Send + Clone, A: Send + Allocator + Clone> Send for WrapperReceiver<T, A> {}
// impl<T> !Sync for WrapperReceiver<T> {}


#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test_case]
    fn test_channel() {
        let (tx, rx) = channel::<i32>();
        let rx = WrapperReceiver::new(rx);

        tx.send(1337);
        tx.send(1338);
        tx.send(1339);
        assert_eq!(Some(vec![1339, 1338, 1337]), rx.recv());

        tx.send(1340);
        tx.send(1341);
        assert_eq!(Some(vec![1341, 1340]), rx.recv());
        assert_eq!(None, rx.recv());
        assert_eq!(None, rx.recv());
    }

}
