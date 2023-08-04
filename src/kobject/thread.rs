use alloc::boxed::Box;

use super::{KObjectRef, KObjectArena};

#[repr(C)]
#[derive(Default)]
pub struct SavedFrame {
    pub sp: usize,
}

#[repr(C)]
pub struct Thread<T: Sized> {
    pub main: extern "C" fn(Box<Self, KObjectArena>),
    pub stack: Box<[usize; 256], KObjectArena>,
    pub saved: SavedFrame,
    pub userdata: T,
}

