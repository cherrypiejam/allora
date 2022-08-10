use core::ptr::NonNull;
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
}


pub struct LockedList<T> {
    list: Mutex<List<T>>,
}

impl<T> LockedList<T> {
    pub const fn new() -> Self {
        let list = Mutex::new(List::new());
        LockedList { list }
    }

    pub fn pop(&mut self) -> Option<T> {
        self.list.lock().pop()
    }
}

use core::fmt::Write;
use crate::uart::UART;
pub fn linked_list_debug_run(uart: &Mutex<Option<UART>>)
{
    let mut list: List<i32> = List::new();
    uart.map(|u| writeln!(u, "{:?}", list.pop()));
    list.push(1);
    list.push(2);
    list.push(3);
    uart.map(|u| writeln!(u, "{:?}", list.pop()));
    uart.map(|u| writeln!(u, "{:?}", list.pop()));
    uart.map(|u| writeln!(u, "{:?}", list.pop()));
    list.push(4);
    list.push(5);
    uart.map(|u| writeln!(u, "{:?}", list.pop()));
    uart.map(|u| writeln!(u, "{:?}", list.pop()));
    list.push(6);
    uart.map(|u| writeln!(u, "{:?}", list.pop()));
    uart.map(|u| writeln!(u, "{:?}", list.pop()));
    uart.map(|u| writeln!(u, "{:?}", list.pop()));
    uart.map(|u| writeln!(u, "{:?}", list.pop()));
    list.push(777);
    uart.map(|u| writeln!(u, "{:?}", list.pop()));
    uart.map(|u| writeln!(u, "{:?}", list.pop()));
}
