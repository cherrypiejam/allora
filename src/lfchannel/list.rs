//! A read and write wait-free linked list

use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};
use core::mem::MaybeUninit;
use core::alloc::Allocator;

use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::alloc::Global;

struct Node<T> {
    elem: T,
    next: AtomicPtr<Node<T>>, // kept for GC
}

impl<T> Node<T> {
    fn new(elem: T) -> Self {
        Node { elem, next: AtomicPtr::new(ptr::null_mut()) }
    }

    fn zeroed() -> Self {
        let elem = unsafe { MaybeUninit::<T>::zeroed().assume_init() };
        Node::new(elem)
    }
}

pub struct List<T, A: Allocator + Clone = Global> {
    head: AtomicPtr<Node<T>>,
    alloc: A,
}

impl<T> List<T> {
    pub fn new() -> List<T> {
        Self::new_in(Global)
    }
}

impl<T, A: Allocator + Clone> List<T, A> {
    pub fn new_in(alloc: A) -> List<T, A> {
        let dummy = Box::into_raw(Box::new_in(Node::<T>::zeroed(), alloc.clone()));
        List { head: AtomicPtr::new(dummy), alloc }
    }

    pub fn push(&self, elem: T) {
        let node = Box::new_in(Node::new(elem), self.alloc.clone());
        let head = self.head.load(Ordering::Acquire);
        let next = unsafe { (*head).next.load(Ordering::Relaxed) };
        node.next.store(next, Ordering::Relaxed);
        unsafe {
            if (*head)
                .next
                .compare_exchange(next, Box::into_raw(node), Ordering::Release, Ordering::Relaxed)
                != Ok(next)
            {
                panic!("data corrupted");
            }
        }
    }

    fn start_read(&self) -> *mut Node<T> {
        let head = self.head.load(Ordering::Acquire);
        unsafe {
            (*head).next.load(Ordering::Relaxed)
        }
    }

    pub fn first(&self) -> &T {
        unsafe {
            &(*self.start_read()).elem
        }
    }
}


impl<T: Clone> List<T> {
    pub fn to_vec(&self) -> Vec<T> {
        self.to_vec_in(Global)
    }
}

impl<T: Clone, A: Allocator + Clone> List<T, A> {
    pub fn to_vec_in<B: Allocator>(&self, alloc: B) -> Vec<T, B> {
        let mut vec = Vec::new_in(alloc);
        let mut cur = self.head.load(Ordering::Acquire);
        loop {
            cur = unsafe { (*cur).next.load(Ordering::Relaxed) };
            if cur.is_null() {
                break
            } else {
                let item = unsafe { (*cur).elem.clone() };
                vec.push(item);
            }
        }
        vec
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_list() {
        let list = List::<i32>::new();
        list.push(10);
        assert!(*list.first() == 10);
        list.push(20);
        assert!(*list.first() == 20);
    }
}
