#![allow(unused)]
use core::fmt::Write;
use core::mem;
use core::ptr::{self, NonNull};
use core::alloc::{Layout, Allocator, GlobalAlloc, AllocError};

use crate::mutex::{Mutex, MutexGuard};
use crate::label::Label;
use crate::bitmap::Bitmap;

use super::PAGE_SIZE;
use super::utils::{align_up, align_down};

type ChunkLink = Option<NonNull<Chunk>>;

#[derive(Debug)]
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
        self.start().checked_add(self.size).expect("Checked add overflow")
    }

    fn aligned_start(&self, align: usize) -> usize {
        align_up(self.start(), align)
    }

    fn check_alloc(&self, layout: Layout) -> Option<usize> {
        let start = align_up(self.start(), layout.align());
        let end = self.end();

        let available_size = end.checked_sub(start)?;
        let exceeded_size = available_size.checked_sub(layout.size())?;

        if available_size >= layout.size() &&
            exceeded_size >= mem::size_of::<Chunk>() {
            Some(start)
        } else {
            None
        }
    }

    fn size_align(layout: Layout) -> Layout {
        let size = layout.size().max(mem::size_of::<Chunk>());
        Layout::from_size_align(size, layout.align())
            .unwrap()
            .align_to(mem::align_of::<Chunk>())
            .unwrap()
            .pad_to_align()
    }
}

#[derive(Debug)]
struct ChunkList {
    head: ChunkLink,
}

impl ChunkList {
    const fn new() -> Self {
        Self { head: None }
    }

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
        // self.push(NonNull::new(chunk_ptr).expect("push region"))
    }

    fn pop_first_fit(&mut self, layout: Layout) -> Option<(NonNull<Chunk>, usize)> {
        let mut cursor = self.head;
        while let Some(chunk) = cursor.map(|mut c| unsafe { c.as_mut() }) {
            if let Some(addr) = chunk.check_alloc(layout) {
                let prev = chunk.prev.take();
                let next = chunk.next.take();
                unsafe {
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
                }
                return cursor.map(|c| (c, addr));
            }
            cursor = chunk.next
        }
        None
    }

}

#[derive(Debug)]
pub struct Arena {
    chunk_list: ChunkList,
    heap_start: usize,
    heap_size: usize,
}

unsafe impl Send for Arena {}

impl Arena {
    pub const fn empty() -> Self {
        Self {
            chunk_list: ChunkList::new(),
            heap_start: 0,
            heap_size: 0
        }
    }

    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_size = heap_size;
        self.chunk_list.push_region(heap_start, heap_size);
    }

    fn allocate_first_fit(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        let layout = Chunk::size_align(layout);
        self.chunk_list
            .pop_first_fit(layout)
            .map(|(chunk, addr)| {
                let new_addr = addr.checked_add(layout.size()).unwrap();
                unsafe {
                    let new_size = chunk.as_ref().end().checked_sub(new_addr).unwrap();
                    self.chunk_list.push_region(new_addr, new_size);
                    NonNull::new(addr as *mut _).unwrap()
                }
            })
    }

    fn allocate(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        self.allocate_first_fit(layout)
    }

    unsafe fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout) {
        let size = Chunk::size_align(layout).size();
        self.chunk_list.push_region(ptr.as_ptr() as usize, size)
    }

    pub fn split(&mut self, layout: Layout) -> Option<Arena> {
        let layout = layout
            .align_to(PAGE_SIZE)
            .unwrap()
            .pad_to_align();
        // Only look for an entire chunk of data
        self.allocate_first_fit(layout)
            .map(|ptr| unsafe {
                let mut arena = Arena::empty();
                arena.init(ptr.as_ptr() as usize, layout.size());
                arena
            })
    }

    pub fn join(&mut self, arena: Arena) {
        // Inherently take over all memory the arena originally owns,
        // even if the arena got splitted after.
        // FIXME: If join an arena splited from another arena,
        // the `heap_start` and `heap_size` won't be updated because
        // it doesn't technically own that part of the memory.
        // This may lead to some issues but let's tackle it later.
        // FIXME: Another issue is that if self creates another arena
        // which is later added to the global arena, when self is added
        // back to the global arena, the global arena would has a duplicated
        // chunk. Alternatively, we can push free chunks to the list, but
        // doing so can cause permanent fragmentations. Even wrose, if we
        // preempt a thread, we cannot reclaim all the memory.
        unsafe {
            self.chunk_list.push_region(arena.heap_start, arena.heap_size)
        }
    }
}

pub struct LabeledArena {
    inner: Mutex<Arena>,
    label: Label,
}

impl LabeledArena {
    pub const fn empty(label: Label) -> Self {
        Self {
            inner: Mutex::new(Arena::empty()),
            label,
        }
    }

    pub fn new(arena: Arena, label: Label) -> Self {
        Self {
            inner: Mutex::new(arena),
            label,
        }
    }

    pub fn lock(&self) -> MutexGuard<Arena> {
        self.inner.lock()
    }

    pub fn label(&self) -> Label {
        self.label.clone()
    }

    pub fn split(&self, layout: Layout, label: Label) -> Option<LabeledArena> {
        self.lock()
            .split(layout)
            .map(|a| {
                LabeledArena {
                    inner: Mutex::new(a),
                    label,
                }
            })
    }

    pub fn join(&self, arena: LabeledArena) {
        self.lock()
            .join(arena.inner.into_inner())
    }
}

unsafe impl<'a> Allocator for &'a LabeledArena {
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

#[derive(Debug, Clone, Copy)]
pub struct RawLabeledArena(NonNull<LabeledArena>);

impl From<&LabeledArena> for RawLabeledArena {
    fn from(value: &LabeledArena) -> Self {
        unsafe {
            Self(NonNull::new_unchecked(value as *const _ as *mut _))
        }
    }
}

unsafe impl Allocator for RawLabeledArena {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let arena = unsafe { self.0.as_ref() };
        arena
            .lock()
            .allocate(layout)
            .map(|p| NonNull::slice_from_raw_parts(p, layout.size()))
            .ok_or_else(|| AllocError)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        let arena = unsafe { self.0.as_ref() };
        arena
            .lock()
            .deallocate(ptr, layout)
    }
}

unsafe impl GlobalAlloc for LabeledArena {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.lock()
            .allocate(layout)
            .map(|p| p.as_ptr())
            .unwrap_or_else(|| ptr::null_mut())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.lock()
            .deallocate(NonNull::new_unchecked(ptr), layout)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::*;
    use core::alloc::Layout;
    const HEAP_SIZE: usize = 500_000_000;

    unsafe fn init_arena() -> Arena {
        let mut arena = Arena::empty();
        arena.init(&HEAP_START as *const _ as usize, HEAP_SIZE);
        arena
    }

    #[test_case]
    fn test_alloc() {
        unsafe {
            let mut arena = init_arena();
            let layout = Layout::from_size_align(100, 1).unwrap();
            for _ in 0..10000 {
                let p = arena.allocate(layout);
                assert!(p.is_some());
            }
        }
    }

    #[test_case]
    #[allow(invalid_value)]
    fn test_dealloc() {
        unsafe {
            let mut arena = init_arena();
            let layout = Layout::from_size_align(100, 1).unwrap();
            let mut plist: [*mut u8; 1000] = mem::MaybeUninit::uninit().assume_init();
            for elem in plist.iter_mut() {
                let p = arena.allocate(layout);
                assert!(p.is_some());
                ptr::write(elem, p.unwrap().as_ptr());
            }
            for elem in plist {
                arena.deallocate(NonNull::new_unchecked(elem), layout);
            }
        }
    }

    // #[test_case]
    // fn test_buggy() {
        // let a = [1; 100000];
        // for b in a.iter() {}
    // }

    // #[test_case]
    // #[allow(invalid_value)]
    // fn test_buggy_iter_if_timer_enabled() {
        // // A possible explaination is that some memory copy operations
        // // got corrupted during a context switch.
        // struct FOO {
            // a: Option<u64>,
            // b: u64,
        // }
        // let mut arena_list: [FOO; 1000] = unsafe {
            // mem::MaybeUninit::uninit().assume_init()
        // };
        // let a = core::array::IntoIter::new(arena_list); // Stuck at here
    // }

    #[test_case]
    fn test_split_merge_on_demand() {
        unsafe {
            let mut arena = init_arena();
            for _ in 0..1000 {
                let a = arena.split(Layout::from_size_align_unchecked(1000, 8));
                assert!(a.is_some());
                arena.join(a.unwrap());
            }
        }
    }

    // #[test_case]
    #[allow(invalid_value)]
    fn test_split_merge_batch(uart: &mut uart::UART) {
        unsafe {
            let mut arena = init_arena();
            let mut arena_list: [Arena; 1000] = mem::MaybeUninit::uninit().assume_init();
            for elem in arena_list.iter_mut() {
                let new_arena = arena.split(Layout::from_size_align_unchecked(1000, 8));
                assert!(new_arena.is_some());
                ptr::write(elem, new_arena.unwrap());
            }

            for elem in arena_list {
                arena.join(elem); // FIXME: breaks when enable timer
                break;
            }
        }
    }
}
