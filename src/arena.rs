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

    unsafe fn get_chunk(&mut self, size: usize, align: usize) -> Option<(NonNull<ChunkHeader>, usize)> {
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
            cursor = chunk.next
        }
        None
    }

    fn try_allocate(&mut self, size: usize, align: usize) -> Option<NonNull<u8>> {
        unsafe {
            self.get_chunk(size, align)
                .map(|(c, a)| (c.as_ref(), a))
                .map(|(chunk, addr)| {
                    let new_addr = addr.checked_add(size).unwrap();
                    let new_size = chunk.end().checked_sub(new_addr).unwrap();
                    self.add_chunk(new_addr, new_size);
                    NonNull::new_unchecked(addr as *mut _)
                })
        }
    }

    fn allocate() { todo!() }

    fn deallocate() { todo!() }

    fn split(&mut self) -> Arena { todo!() }

    fn merge(&mut self, arena: Arena) { todo!() }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::*;
    use core::fmt::Write;
    const HEAP_SIZE: usize = 500_000_000;

    #[test_case]
    fn test_something(uart: &mut uart::UART) {
        unsafe {
            let heap_start = &HEAP_START as *const _ as usize;
            let heap_size = HEAP_SIZE;
            let mut arena = Arena::empty();
            arena.init(heap_start, heap_size);
            assert_eq!(1, 1);
        }
    }
}
