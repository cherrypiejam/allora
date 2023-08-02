use core::alloc::Allocator;
use alloc::vec::Vec;

use super::{KObjectRef, KObjectArena};

pub struct Container {
    pub slots: Vec<KObjectRef, KObjectArena>,
}

impl Drop for Container {
    fn drop(&mut self) {
        // 1. update meta data
        // 2. destroy allocator
        // 3. return pages used by the allocator to its parent's free_pages
    }
}

impl Container {
    pub fn new_in(alloc: KObjectArena) -> Self {
        Container { slots: Vec::new_in(alloc) }
    }

    pub fn get_slot() {}
    pub fn find_slot() {}
    pub fn set_slot() {}
}
