use core::ptr::NonNull;
use core::alloc::{Layout, Allocator, AllocError};

use super::chunk_list::ChunkList;
use crate::mutex::Mutex;

#[derive(Debug)]
pub struct Arena {
    chunks: ChunkList,
}

unsafe impl Send for Arena {}

impl Arena {
    pub const fn empty() -> Arena {
        Self { chunks: ChunkList::empty() }
    }

    pub unsafe fn new(start: usize, size: usize) -> Arena {
        Self { chunks: ChunkList::new(start, size) }
    }

    pub unsafe fn append(&mut self, start: usize, size: usize) {
        self.chunks.append(start, size)
    }

    pub fn allocate(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        let layout = ChunkList::align_layout(layout);
        self.chunks.pop_first_fit(layout)
    }

    pub unsafe fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout) {
        self.chunks.push(
            ptr.as_ptr() as usize,
            ChunkList::align_layout(layout).size(),
        )
    }
}

unsafe impl<'a> Allocator for &'a Mutex<Arena> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.lock()
            .allocate(layout)
            .map(|p| NonNull::slice_from_raw_parts(p, layout.size()))
            .ok_or_else(|| AllocError)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        self.lock()
            .deallocate(ptr, layout)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::*;
    const SIZE: usize = 500_000_000;

    fn init_arena(size: usize) -> Mutex<Arena> {
        unsafe {
            let start = &HEAP_START as *const _ as usize;
            Mutex::new(Arena::new(start, size))
        }
    }

    #[test_case]
    fn test_alloc() {
        let arena = init_arena(SIZE);
        for _ in 0..10000 {
            let _ = Box::new_in(1, &arena);
        }
    }

    #[test_case]
    fn test_alloc_large() {
        let arena = init_arena(SIZE);
        for _ in 0..10000 {
            let _ = Box::new_in([0u8; 4096], &arena);
        }
        for _ in 0..10000 {
            let _ = Box::leak(Box::new_in([0u8; 4096], &arena));
        }
    }
}

