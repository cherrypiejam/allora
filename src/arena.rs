#![allow(unused)]
use core::fmt::Write;
use core::mem;
use core::ptr;
use core::ptr::NonNull;
use core::alloc::Layout;

type ChunkLink = Option<NonNull<Chunk>>;


fn align_up(addr: usize, align: usize) -> usize {
    assert_eq!(align & (align - 1), 0); // Assume a power of 2 align
    (addr + (align - 1)) & !(align - 1)
}

struct Chunk {
    size: usize,
    next: ChunkLink,
    prev: ChunkLink,
}

impl Chunk {
    fn new(size: usize) -> Self {
        Self { size, next: None, prev: None }
    }

    fn start(&self) -> usize {
        &*self as *const _ as usize
    }

    fn end(&self) -> usize {
        self.start().checked_add(self.size).expect("Check add overflow")
    }

    fn aligned_start(&self, align: usize) -> usize {
        Self::align_up(self.start())
    }

    fn check_alloc(&self, size: usize, align: usize) -> Option<usize> {
        let start = self.aligned_start(align);
        let end = self.end();

        let available_size = end.checked_sub(start)?;
        let exceeded_size = available_size.checked_sub(size)?;
        if available_size >= size && exceeded_size >= mem::size_of::<Chunk>() {
            Some(start)
        } else {
            None
        }
    }

    fn align_up(addr: usize) -> usize {
        align_up(addr, mem::align_of::<Chunk>())
    }
}

struct ChunkList {
    head: ChunkLink,
}

impl ChunkList {
    fn new() -> Self {
        Self { head: None }
    }

    // unsafe fn init(&mut self, addr: usize) {
        // let dummy = Chunk::new(0);
        // let ptr = addr as *mut Chunk;
        // ptr::write(ptr, dummy);
        // self.head = Some(NonNull::new_unchecked(ptr));
    // }

    fn push(&mut self, mut chunk: NonNull<Chunk>) {
        unsafe {
            let next = self.head.take();
            chunk.as_mut().next = next;
            chunk.as_mut().prev = None;
            if let Some(mut next_chunk) = next {
                next_chunk.as_mut().prev = Some(chunk);
            }
            self.head = Some(chunk)
        }
    }

    unsafe fn push_region(&mut self, addr: usize, size: usize) {
        assert!(size >= mem::size_of::<Chunk>());
        let chunk_header = Chunk::new(size);
        let chunk_ptr = addr as *mut Chunk;
        ptr::write(chunk_ptr, chunk_header);
        self.push(NonNull::new_unchecked(chunk_ptr))
    }

    fn pop(&mut self) -> Option<NonNull<Chunk>> {
        None
    }

    fn pop_first_fit(&mut self, size: usize, align: usize) -> Option<(NonNull<Chunk>, usize)> {
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
                        _ => {
                            self.head = next;
                        }
                    }
                    return cursor.map(|c| (c, addr));
                }
                cursor = chunk.next
            }
            None
        }
    }
}


pub struct Arena {
    chunk_list: ChunkList,
    heap_start: usize,
    heap_size: usize,
}

impl Arena {
    pub fn empty() -> Self {
        Self {
            chunk_list: ChunkList::new(),
            heap_start: 0,
            heap_size: 0
        }
    }

    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.chunk_list.push_region(heap_start, heap_size)
    }

    fn allocate_first_fit(&mut self, size: usize, align: usize, uart: &mut crate::uart::UART) -> Option<NonNull<u8>> {
        self.chunk_list
            .pop_first_fit(size, align)
            .map(|(chunk, addr)| {
                let new_addr = addr.checked_add(size)
                    .map(|a| align_up(a, mem::align_of::<Chunk>()))
                    .unwrap();
                // let _ = uart.write_fmt(format_args!("{}, {}\n", new_addr, new_size));
                unsafe {
                    let new_size = chunk.as_ref().end().checked_sub(new_addr).unwrap();
                    self.chunk_list.push_region(new_addr, new_size);
                    NonNull::new_unchecked(addr as *mut _)
                }
            })
    }

    unsafe fn allocate(&mut self, layout: Layout, uart: &mut crate::uart::UART) -> *mut u8 {
        let layout = layout
            .align_to(mem::align_of::<Chunk>())
            .unwrap()
            .pad_to_align();
        let size = layout.size().max(mem::size_of::<Chunk>());
        let align = layout.align();
        match self.allocate_first_fit(size, align, uart) {
            Some(ptr) => ptr.as_ptr(),
            None => ptr::null_mut(),
        }
    }

    fn deallocate() { todo!() }

    fn split(&mut self, size: usize, uart: &mut crate::uart::UART) -> Option<Arena> {
        // Only look for the entire chunk of data
        self.allocate_first_fit(size, mem::align_of::<Chunk>(), uart)
            .map(|ptr| unsafe {
                let mut arena = Arena::empty();
                arena.init(ptr.as_ptr() as usize, size);
                arena
            })
    }

    fn merge(&mut self, arena: Arena) {
        if let Some(chunk) = arena.chunk_list.head {
            self.chunk_list.push(chunk)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::*;
    use core::fmt::Write;
    use core::alloc::Layout;
    use core::mem;
    const HEAP_SIZE: usize = 500_000_000;

    unsafe fn init_arena() -> Arena {
        let mut arena = Arena::empty();
        arena.init(&HEAP_START as *const _ as usize, HEAP_SIZE);
        arena
    }

    #[test_case]
    fn test_something(uart: &mut uart::UART) {
        unsafe {
            let mut arena = init_arena();
            let layout = Layout::from_size_align(100, 1).unwrap();
            for _ in 0..10000 {
                let p = arena.allocate(layout, uart);
                assert!(!p.is_null());
                let a = arena.split(1000, uart);
                assert!(a.is_some());
                let _ = arena.merge(a.unwrap());
            }
        }
    }

    #[test_case]
    fn test_split_merge_on_demand(uart: &mut uart::UART) {
        unsafe {
            let mut arena = init_arena();
            for _ in 0..1000 {
                let a = arena.split(1000, uart);
                assert!(a.is_some());
                let _ = arena.merge(a.unwrap());
            }
        }
    }

    #[test_case]
    #[allow(invalid_value)]
    fn test_split_merge_batch(uart: &mut uart::UART) {
        unsafe {
            let mut arena = init_arena();
            let mut arena_list: [Option<Arena>; 1000] = mem::MaybeUninit::uninit().assume_init();
            for elem in arena_list.iter_mut() {
                let new_arena = arena.split(1000, uart);
                assert!(new_arena.is_some());
                ptr::write(elem, new_arena);
            }
            for elem in arena_list.iter_mut() {
                arena.merge(elem.take().unwrap());
            }
        }
    }

    #[test_case]
    fn test_align_up(_uart: &mut uart::UART) {
        assert_eq!(align_up(0b10100, 0b100), 0b10100);
        assert_eq!(align_up(0b10101, 0b100), 0b11000);
        assert_eq!(align_up(0b10110, 0b100), 0b11000);
        assert_eq!(align_up(0b10111, 0b100), 0b11000);
    }

}
