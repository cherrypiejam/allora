use core::mem;
use core::ptr;
use core::ptr::NonNull;

type Link = Option<NonNull<ChunkHeader>>;

struct ChunkHeader {
    size: usize,
    next: Link,
    prev: Link,
}

impl ChunkHeader {
    fn start(&self) -> usize {
        &*self as *const _ as usize
    }

    fn end(&self) -> usize {
        self.start().checked_add(self.size).expect("Check add overflow")
    }

    fn aligned_start(&self, align: usize) -> usize {
        assert_eq!(align & (align - 1), 0); // Assume 2^n align
        (self.start() + (align - 1)) & !(align - 1)
    }

    fn check_alloc(&self, size: usize, align: usize) -> Option<usize> {
        let start = self.aligned_start(align);
        let end = self.end();
        let available_size = end - start;
        let exceeded_size = available_size - size;
        if available_size >= size && exceeded_size >= mem::size_of::<ChunkHeader>() {
            Some(start)
            // unsafe { Some(NonNull::new_unchecked(chunk_start as *mut u8)) }
        } else {
            None
        }
    }
}

#[derive(Default)]
pub struct Arena {
    head: Link,
    heap_start: usize,
    heap_size: usize,
}

impl Arena {
    pub fn empty() -> Arena {
        Default::default()
    }

    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.add_chunk(heap_start, heap_size)
    }

    unsafe fn add_chunk(&mut self, addr: usize, size: usize) {
        assert!(size >= mem::size_of::<ChunkHeader>());
        let next = self.head.take();
        let chunk_header = ChunkHeader { size, next, prev: None };
        let chunk_ptr = addr as *mut ChunkHeader;
        ptr::write(chunk_ptr, chunk_header);
        let new = NonNull::new_unchecked(chunk_ptr);
        if let Some(mut old) = next {
            old.as_mut().prev = Some(new);
        }
        self.head = Some(new);
    }

    fn get_chunk(&mut self, size: usize, align: usize) -> Option<(NonNull<ChunkHeader>, usize)> {
        unsafe {
            let mut cursor = self.head;
            while let Some(chunk) = cursor.map(|mut c| c.as_mut()) {
                if let Some(addr) = chunk.check_alloc(size, align) {
                    let prev = chunk.prev.take();
                    let next = chunk.next.take();
                    match (prev, next) {
                        (Some(mut p), Some(mut n)) => {
                            p.as_mut().next = next;
                            n.as_mut().prev = prev;
                        }
                        (_, Some(mut n)) => {
                            n.as_mut().prev = prev;
                            self.head = next;
                        }
                        (Some(mut p), _) => {
                            p.as_mut().next = next;
                        }
                        _ => panic!("what the heck?")
                    }
                    return cursor.map(|c| (c, addr));
                }
            }
            None
        }
    }

    fn try_alloc(&mut self, size: usize, align: usize) -> Option<NonNull<u8>> {
        unsafe {
            let mut cursor = self.head;
            while let Some(chunk) = cursor.map(|mut c| c.as_mut()) {
                if let Some(alloc_ptr) = chunk.check_alloc(size, align) {
                    let prev = chunk.prev.take();
                    let next = chunk.next.take();
                    match (prev, next) {
                        (Some(mut p), Some(mut n)) => {
                            p.as_mut().next = next;
                            n.as_mut().prev = prev;
                        }
                        (_, Some(mut n)) => {
                            n.as_mut().prev = prev;
                            self.head.replace(n);
                        }
                        (Some(mut p), _) => {
                            p.as_mut().next = next;
                        }
                        _ => panic!("what the heck?")
                    }
                    let addr = chunk as *mut _ as usize;
                    let new_addr = addr.checked_add(size).expect("Checked add");
                    let new_size = chunk.size.checked_sub(size).expect("Checked sub");
                    self.add_chunk(new_addr, new_size);
                    return Some(alloc_ptr);
                }
                cursor = chunk.next
            }
            None
        }
    }

    fn split(&mut self) -> Arena { todo!() }

    fn merge(&mut self, arena: Arena) { todo!() }
}

#[cfg(test)]
mod test {
    use crate::*;
    use core::fmt::Write;

    #[test_case]
    fn test_something(uart: &mut uart::UART) {
        assert_eq!(1, 1);
    }
}
