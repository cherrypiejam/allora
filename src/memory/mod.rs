pub mod arena;
pub mod page;

pub use page::{PAGE_SIZE, Error};

pub fn page_align_down(addr: usize) -> usize {
    align_down(addr, PAGE_SIZE)
}

pub fn page_align_up(addr: usize) -> usize {
    align_up(addr, PAGE_SIZE)
}

// Must be a power of 2 align
fn align_down(addr: usize, align: usize) -> usize {
    assert_eq!(align & (align - 1), 0);
    addr & !(align - 1)
}

fn align_up(addr: usize, align: usize) -> usize {
    align_down(addr + (align - 1), align)
}
