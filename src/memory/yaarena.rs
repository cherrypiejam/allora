use core::ptr::NonNull;
use core::mem;
use core::alloc::{Layout, Allocator, AllocError};

use super::{align_up, align_down};

use crate::mutex::Mutex;

#[derive(Debug)]
struct CanAllocInfo {
    start: usize,
    end: usize,
    alloc_start_offset: usize,
    alloc_end_offset: usize,
}

#[derive(Debug, Clone)]
struct Chunk {
    size: usize,
    next: Option<NonNull<Chunk>>,
}

impl Chunk {
    fn can_allocate(&self, layout: Layout) -> Option<CanAllocInfo> {
        let start = &*self as *const _ as usize;
        let end = start.checked_add(self.size).unwrap();
        let alloc_start = {
            if start == align_up(start, layout.align()) {
                start
            } else {
                align_up(start + mem::size_of::<Chunk>(), layout.align())
            }
        };

        let alloc_start_offset = alloc_start - start;
        let alloc_end = align_up(alloc_start + layout.size(), mem::align_of::<Chunk>());
        let alloc_end_offset = alloc_end - start;

        let left = end.checked_sub(alloc_end)?;
        if left == 0 || left >= mem::size_of::<Chunk>() {
            Some(CanAllocInfo { start, end, alloc_start_offset, alloc_end_offset })
        } else {
            None
        }
    }
}

// TODO: make it non-blocking
#[derive(Debug, Clone)]
struct ChunkList {
    head: Chunk,
}

impl ChunkList {
    pub const fn empty() -> ChunkList {
        Self { head: Chunk { size: 0, next: None } }
    }

    unsafe fn init_chunk(start: usize, size: usize, next: Option<NonNull<Chunk>>) -> NonNull<Chunk> {
        let aligned_start = align_up(start, mem::align_of::<Chunk>());
        let aligned_size = align_down(size, mem::align_of::<Chunk>());

        let ptr = aligned_start as *mut Chunk;
        ptr.write(Chunk {
            size: aligned_size,
            next,
        });

        NonNull::new_unchecked(ptr)
    }

    pub unsafe fn new(start: usize, size: usize) -> ChunkList {
        Self {
            head: Chunk { size: 0, next: Some(Self::init_chunk(start, size, None)) }
        }
    }

    pub unsafe fn push(&mut self, start: usize, size: usize) {
        let next = Self::init_chunk(start, size, self.head.next);
        self.head.next = Some(next);
    }

    pub unsafe fn append(&mut self, start: usize, size: usize) {
        let next = Self::init_chunk(start, size, None);
        if let Some(last) = self.last() {
            last.next = Some(next);
        } else {
            self.head.next = Some(next);
        }
    }

    pub fn pop_first_fit(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        self.head.next.and_then(|_| {
            let mut cur = (&mut self.head, None); // current, previous
            let mut canalloc = None;
            while let Some(mut next_ptr) = cur.0.next {
                cur.1 = Some(cur.0);
                cur.0 = unsafe { next_ptr.as_mut() };
                if let Some(canalloc_) = cur.0.can_allocate(layout) {
                    canalloc = Some(canalloc_);
                    break;
                }
            }

            canalloc.map(|c| {
                if c.alloc_end_offset == cur.0.size {
                    if c.alloc_start_offset == 0 {
                        cur.1.as_mut().unwrap().next = cur.0.next;
                        unsafe {
                            NonNull::new_unchecked(c.start as *mut _)
                        }
                    } else {
                        let ptr = c.start as *mut Chunk;
                        unsafe {
                            ptr.write(Chunk {
                                size: c.alloc_start_offset,
                                next: cur.0.next,
                            });
                            cur.1.as_mut().unwrap().next = Some(NonNull::new_unchecked(ptr));
                            NonNull::new_unchecked((c.start + c.alloc_start_offset) as *mut _)
                        }
                    }
                } else {
                    let back_ptr = (c.start + c.alloc_end_offset) as *mut Chunk;
                    unsafe {
                        back_ptr.write(Chunk {
                            size: c.end - c.start - c.alloc_end_offset,
                            next: cur.0.next,
                        });
                        cur.1.as_mut().unwrap().next = Some(NonNull::new_unchecked(back_ptr));
                    }
                    if c.alloc_start_offset == 0 {
                        unsafe {
                            NonNull::new_unchecked(c.start as *mut _)
                        }
                    } else {
                        let front_ptr = c.start as *mut Chunk;
                        unsafe {
                            front_ptr.write(Chunk {
                                size: c.alloc_start_offset,
                                next: Some(NonNull::new_unchecked(back_ptr)),
                            });
                            cur.1.as_mut().unwrap().next = Some(NonNull::new_unchecked(front_ptr));
                            NonNull::new_unchecked((c.start + c.alloc_start_offset) as *mut _)
                        }
                    }
                }
            })
        })
    }

    fn _first(&self) -> Option<&mut Chunk> {
        self.head.next.map(|mut c| unsafe { c.as_mut() })
    }

    fn last(&mut self) -> Option<&mut Chunk> {
        if self.head.next.is_none() {
            None
        } else {
            let mut cur = &mut self.head;
            while let Some(mut next_ptr) = cur.next {
                cur = unsafe { next_ptr.as_mut() };
            }
            Some(cur)
        }
    }

    fn align_layout(layout: Layout) -> Layout {
        let size = layout.size().max(mem::size_of::<Chunk>()); // TODO: size aligned with Chunk?
        Layout::from_size_align(size, layout.align())
            .unwrap()
            .align_to(mem::align_of::<Chunk>())
            .unwrap()
            .pad_to_align()
    }
}

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

    unsafe fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout) {
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
