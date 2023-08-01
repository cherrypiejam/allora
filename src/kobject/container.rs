use core::alloc::Allocator;
use alloc::vec::Vec;

use super::KObjectRef;

pub struct Container<A: Allocator + Clone> {
    pub slots: Vec<KObjectRef, A>,
}
