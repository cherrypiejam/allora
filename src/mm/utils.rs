// Must be a power of 2 align
pub fn align_down(addr: usize, align: usize) -> usize {
    assert_eq!(align & (align - 1), 0);
    addr & !(align - 1)
}

pub fn align_up(addr: usize, align: usize) -> usize {
    align_down(addr + (align - 1), align)
}
