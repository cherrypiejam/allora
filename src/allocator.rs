// ref https://os.phil-opp.com/allocator-designs/

use alloc::alloc::{GlobalAlloc, Layout};
use core::{ptr, ptr::NonNull};
use crate::mutex;

const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024];

struct ListNode {
    next: Option<&'static mut ListNode>,
}

pub struct FixedBlockAllocator {
    list_heads: [Option<&'static mut ListNode>; BLOCK_SIZES.len()],
    heap: linked_list_allocator::Heap,
}

impl FixedBlockAllocator {
    pub const fn new() -> Self {
        const EMPTY: Option<&'static mut ListNode> = None;
        FixedBlockAllocator {
            list_heads: [EMPTY; BLOCK_SIZES.len()],
            heap: linked_list_allocator::Heap::empty(),
        }
    }

    pub unsafe fn init(&mut self, heap_bottom: usize, heap_size: usize) {
        self.heap.init(heap_bottom, heap_size);
    }

    fn heap_alloc(&mut self, layout: Layout) -> *mut u8 {
        match self.heap.allocate_first_fit(layout) {
            Ok(ptr) => ptr.as_ptr(),
            Err(_) => ptr::null_mut(),
        }
    }

}

fn list_index(layout: &Layout) -> Option<usize> {
    let layout_size = layout.size().max(layout.align());
    BLOCK_SIZES.iter().position(|&x| x >= layout_size)
}

unsafe impl GlobalAlloc for mutex::Mutex<FixedBlockAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();

        let index = list_index(&layout);
        match index {
            Some(index) => {
                let list_head = allocator.list_heads[index].take();
                match list_head {
                    Some(node) => {
                        allocator.list_heads[index] = node.next.take();
                        node as *mut ListNode as *mut u8
                    }
                    None => {
                        let block_size = BLOCK_SIZES[index];
                        let block_align = block_size;
                        let block_layout = Layout::from_size_align(block_size, block_align).unwrap()
                            .align_to(layout.align()).unwrap()
                            .pad_to_align();
                        // alloc a block
                        allocator.heap_alloc(block_layout)
                    }
                }
            }
            None => {
                // alloc as requested
                allocator.heap_alloc(layout)
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut allocator = self.lock();

        let index = list_index(&layout);
        match index {
            Some(index) => {
                // put it back in free list instead
                let node = ListNode {
                    next: allocator.list_heads[index].take(),
                };
                let node_ptr = ptr as *mut ListNode;
                ptr::write(node_ptr, node);
                allocator.list_heads[index] = Some(&mut *(node_ptr));
            }
            None => {
                let ptr = NonNull::new(ptr).expect("dealloc a null pointer!");
                allocator.heap.deallocate(ptr, layout);
            }
        }
    }
}



