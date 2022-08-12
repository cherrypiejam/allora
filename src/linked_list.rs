use core::ptr::NonNull;
use core::marker::PhantomData;
use alloc::boxed::Box;
use crate::mutex::Mutex;

struct Node<T> {
    elem: T,
    next: Option<NonNull<Node<T>>>,
}

impl<T> Node<T> {
    fn new(elem: T) -> Self {
        Node { elem, next: None }
    }
}

pub struct List<T> {
    head: Option<NonNull<Node<T>>>,
    tail: Option<NonNull<Node<T>>>,
}

impl<T> List<T> {
    pub const fn new() -> Self {
        List { head: None, tail: None }
    }

    // pop front
    pub fn pop(&mut self) -> Option<T> {
        let node = self.head.take();
        node.map(|n| {
            let mut node = unsafe { Box::from_raw(n.as_ptr()) };
            self.head = node.next.take();
            if self.head.is_none() {
                self.tail = None;
            }
            node.elem
        })
    }

    // push back
    pub fn push(&mut self, elem: T) {
        let node = NonNull::new(
            Box::into_raw(Box::new(Node::new(elem)))
        );
        let old_tail = self.tail.take();
        if let Some(old_tail) = old_tail {
            unsafe {
                (*old_tail.as_ptr())
                    .next = node;
            }
        } else {
            assert_eq!(self.head, None);
            assert_eq!(self.tail, None);
            self.head = node;
        }
        self.tail = node;
    }

    pub fn len(&self) -> usize {
        let mut count = 0;
        let mut cur = &self.head;
        while let Some(node) = cur {
            cur = unsafe { &(*node.as_ptr()).next };
            count += 1;
        }
        count
    }

    fn iter(&self) -> ListIter<'_, T> {
        ListIter {
            cur: self.head,
            _marker: PhantomData
        }
    }
}

unsafe impl<T> Send for List<T> {} // FIXME
unsafe impl<T: Sync> Sync for List<T> {}

struct ListIter<'a, T> {
    cur: Option<NonNull<Node<T>>>,
    _marker: PhantomData<&'a T>, // like it owns &'a T
}

impl<'a, T> Iterator for ListIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.cur.map(|cur| {
            let cur = unsafe { &*cur.as_ptr() };
            self.cur = cur.next;
            &cur.elem
        })
    }
}


// pub struct LockedList<T> {
    // list: Mutex<List<T>>,
// }

// impl<T> LockedList<T> {
    // pub const fn new() -> Self {
        // let list = Mutex::new(List::new());
        // LockedList { list }
    // }

    // pub fn pop(&mut self) -> Option<T> {
        // self.list.lock().pop()
    // }

    // pub fn push(&mut )
// }

use core::fmt::Write;
use crate::uart::UART;
pub fn linked_list_debug_run(uart: &Mutex<Option<UART>>)
{
    let mut list: List<i32> = List::new();
    list.push(0);
    list.push(1);
    list.push(2);
    list.push(3);
    list.push(4);
    list.push(5);
    assert_eq!(list.len(), 6);
    for (i, &e) in list.iter().enumerate() {
        assert_eq!(e, i as i32);
    }
    assert_eq!(list.pop(), Some(0));
    assert_eq!(list.pop(), Some(1));
    assert_eq!(list.pop(), Some(2));
    list.push(6);
    assert_eq!(list.len(), 4);
    assert_eq!(list.pop(), Some(3));
    assert_eq!(list.pop(), Some(4));
    assert_eq!(list.pop(), Some(5));
    assert_eq!(list.pop(), Some(6));
    assert_eq!(list.pop(), None);
    assert_eq!(list.pop(), None);
    assert_eq!(list.pop(), None);
    assert_eq!(list.len(), 0);
    uart.map(|u| {
        writeln!(u, "linked_list_debug_run passed")
    });
}
