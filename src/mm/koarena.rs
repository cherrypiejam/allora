use core::ptr::NonNull;
use core::alloc::{Layout, Allocator, AllocError};

use super::yaarena::Arena;
use crate::mutex::Mutex;

#[derive(Debug)]
pub struct KObjectArena {
    arena: alloc::sync::Arc<Mutex<Arena>>, // TODO arc uses global heap
}

impl KObjectArena {
    pub fn empty() -> Self {
        KObjectArena {
            arena: alloc::sync::Arc::new(Mutex::new(Arena::empty()))
        }
    }

    pub unsafe fn new(start: usize, size: usize) -> Self {
        KObjectArena {
            arena: alloc::sync::Arc::new(Mutex::new(Arena::new(start, size)))
        }
    }

    pub fn as_ref(&self) -> &alloc::sync::Arc<Mutex<Arena>> {
        &self.arena
    }

    pub fn as_mut(&mut self) -> &mut alloc::sync::Arc<Mutex<Arena>> {
        &mut self.arena
    }
}

unsafe impl<'a> Allocator for KObjectArena {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.arena
            .lock()
            .allocate(layout)
            .map(|p| NonNull::slice_from_raw_parts(p, layout.size()))
            .ok_or_else(|| AllocError)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        self.arena
            .lock()
            .deallocate(ptr, layout)
    }
}

impl Clone for KObjectArena {
    fn clone(&self) -> Self {
        KObjectArena { arena: alloc::sync::Arc::clone(&self.arena) }
    }
}

// #[derive(Debug)]
// pub struct KObjectArena {
    // pub arena: Mutex<Arena>,
    // refs: core::sync::atomic::AtomicUsize,
// }

// impl Clone for KObjectArena {
    // fn clone(&self) -> Self {
        // self.refs.fetch_add(1, core::sync::atomic::Ordering::SeqCst)
    // }
// }


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

