use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use core::mem::MaybeUninit;
use core::alloc::Allocator;

use alloc::alloc::Global;
use alloc::borrow::ToOwned;
use alloc::boxed::Box;

struct Node<T> {
    elem: T,
    next: AtomicPtr<Node<T>>,
}

impl<T> Node<T> {
    fn new(elem: T) -> Self {
        Node { elem, next: AtomicPtr::new(ptr::null_mut()) }
    }

    unsafe fn zeroed() -> Self {
        let elem = MaybeUninit::<T>::zeroed().assume_init();
        Self::new(elem)
    }

    // fn next<'a>(&self) -> Option<&'a Node<T>> {
        // let mut next = self.next.load(Ordering::SeqCst);
        // unsafe {
            // while !next.is_null()
                // && (*next).removed.load(Ordering::SeqCst) {
                // next = (*next).next.load(Ordering::SeqCst);
            // }
            // next.as_ref()
        // }
    // }

    // fn next_mut<'a>(&mut self) -> Option<&'a mut Node<T>> {
        // let mut next = self.next.load(Ordering::SeqCst);
        // unsafe {
            // while !next.is_null()
                // && (*next).removed.load(Ordering::SeqCst) {
                // next = (*next).next.load(Ordering::SeqCst);
            // }
            // next.as_mut()
        // }
    // }
}

// impl<T> Drop for Node<T> {
    // fn drop(&mut self) {
        // crate::debug!("Node gets dropped! ")
    // }
// }

struct List<T, A: Allocator + Clone = Global> {
    head: AtomicPtr<Node<T>>,
    alloc: A,
}

impl<T> List<T> {
    pub fn new() -> List<T> {
        Self::new_in(Global)
    }

    // unsafe fn first(&self) -> Option<&T> {
        // self.head.load(Ordering::SeqCst)
            // .as_ref()
            // .and_then(|n| {
                // n.next.load(Ordering::SeqCst)
                    // .as_ref()
                    // .map(|n| &n.elem)
            // })
    // }

    // unsafe fn second(&self) -> Option<&T> {
        // self.head.load(Ordering::SeqCst)
            // .as_ref()
            // .and_then(|n| {
                // n.next.load(Ordering::SeqCst)
                    // .as_ref()
                    // .and_then(|n| {
                        // n.next.load(Ordering::SeqCst)
                            // .as_ref()
                            // .map(|n| &n.elem)
                    // })
            // })
    // }
}

impl<T, A: Allocator + Clone> List<T, A> {
    pub fn new_in(alloc: A) -> List<T, A> {
        let dummy = Box::into_raw(Box::new_in(
            unsafe { Node::<T>::zeroed() },
            alloc.clone()
        ));
        List { head: AtomicPtr::new(dummy), alloc }
    }

    pub fn push(&mut self, elem: T) {
        let node = Box::into_raw(Box::new_in(Node::new(elem), self.alloc.clone()));
        unsafe {
            // blocks only when multiple writers
            while let Err(_) = self.try_push(node) {}
        }
    }

    unsafe fn try_push(&mut self, new: *mut Node<T>) -> Result<(), ()> {
        let head = self.head.load(Ordering::SeqCst).as_ref().ok_or(())?;
        let old = head.next.load(Ordering::SeqCst);
        (*new).next.store(old, Ordering::SeqCst);

        head.next
            .compare_exchange(old, new, Ordering::SeqCst, Ordering::SeqCst)
            .map(|_| ())
            .map_err(|_| ())
    }

    // TODO: pop is remove first
    // pub fn pop(&mut self) -> Option<T> {
        // unsafe {
            // loop {
                // if let Ok(node) = self.try_pop() {
                    // let node = Box::from_raw_in(raw, alloc)
                // }
            // }
        // }
    // }

    // unsafe fn try_pop(&mut self) -> Option<*mut Node<T>> {
        // let head = self.head.load(Ordering::Acquire).as_ref()?;
        // let old = head.next_mut()?;
        // let new = old.next_mut()?;

        // head.next
            // .compare_exchange(old, new, Ordering::Release, Ordering::Relaxed)
            // .map(|_| new as *mut _)
            // .ok()
    // }

    pub fn pop(&mut self) -> Option<T> {
        self.remove_nth(1)
    }

    pub fn remove_nth(&mut self, nth: usize) -> Option<T> {

        let mut cur = unsafe { &mut *self.head.load(Ordering::SeqCst) };
        let mut prev = None;

        for _ in 0..nth {
            if let Some(next) = unsafe {
                cur.next.load(Ordering::SeqCst).as_mut()
            } {
                prev = Some(cur);
                cur = next;
            } else {
                return None
            }
        }

        if let Some(prev) = prev {
            let new = cur.next.load(Ordering::SeqCst);
            if let Ok(_) =
                prev.next
                    .compare_exchange(cur, new, Ordering::SeqCst, Ordering::SeqCst)
            {
                Some(unsafe {
                    Box::from_raw_in(cur, self.alloc.clone()).elem
                })
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter(unsafe {
            self.head
                .load(Ordering::Acquire)
                .as_ref()
                .and_then(|head| {
                    head.next
                        .load(Ordering::SeqCst)
                        .as_ref()
                })

        })
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut(unsafe {
            self.head
                .load(Ordering::Acquire)
                .as_ref()
                .and_then(|head| {
                    head.next
                        .load(Ordering::SeqCst)
                        .as_mut()
                })
        })
    }

    pub fn len(&self) -> usize {
        self.iter().fold(0, |a, _| a + 1)
    }

}

// impl<T: PartialEq, A: Allocator + Clone> List<T, A> {
    // pub fn remove(&mut self, elem: T) -> bool {
        // let mut cur = self.head.load(Ordering::Acquire);
        // let mut prev = cur;
        // while !cur.is_null() {
            // unsafe {
                // if (*cur).elem == elem {
                    // break
                // } else {
                    // prev = cur;
                    // cur = Self::next(cur);
                // }
            // }
        // }
        // let node = unsafe { &mut *cur };

        // if node
            // .removed
            // .compare_exchange(false, true, Ordering::Release, Ordering::Relaxed)
            // .is_ok()
        // {
            // let next = node.next.load(Ordering::Acquire);
            // unsafe { (*prev).next.store(next, Ordering::Release); }

            // drop(unsafe {
                // Box::from_raw_in(cur, self.alloc.clone()) // GC
            // });

            // true
        // } else {
            // false
        // }
    // }

    // unsafe fn next(node: *mut Node<T>) -> *mut Node<T> {
        // let mut cur = (*node).next.load(Ordering::Relaxed);
        // while !cur.is_null() {
            // if !(*cur).removed.load(Ordering::Relaxed) {
                // break
            // } else {
                // cur = (*cur).next.load(Ordering::Relaxed);
            // }
        // }
        // cur
    // }
// }

impl<T, A: Allocator + Clone> IntoIterator for List<T, A> {
    type Item = T;

    type IntoIter = IntoIter<T, A>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self)
    }
}

struct IntoIter<T, A: Allocator + Clone = Global>(List<T, A>);

impl<T, A: Allocator + Clone> Iterator for IntoIter<T, A> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        todo!() // pop first
    }
}


struct Iter<'a, T>(Option<&'a Node<T>>);

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .map(|node| {
                self.0 = unsafe {
                    node.next.load(Ordering::SeqCst).as_ref()
                };
                &node.elem
            })
    }
}

struct IterMut<'a, T>(Option<&'a mut Node<T>>);

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .take()
            .map(|node| {
                self.0 = unsafe {
                    node.next.load(Ordering::SeqCst).as_mut()
                };
                &mut node.elem
            })
    }

}


#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_collection_list() {
        let mut list = List::<usize>::new();
        let items = [0, 1, 2, 3, 4, 5];
        items.iter().rev().for_each(|&i| list.push(i));
        list.iter().enumerate().for_each(|(i, e)| {
            assert_eq!(i, *e as usize);
        });
        items.iter().for_each(|&i| assert_eq!(list.pop(), Some(i)));
        assert_eq!(list.pop(), None);
        assert_eq!(list.len(), 0);
    }
}
