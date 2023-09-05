use alloc::vec::Vec;

use super::{KObjectRef, KObjectArena, KObjectPtr};
use super::{kobject_create, INVALID_KOBJ_ID};
use super::Label;
use super::ThreadRef;

#[derive(Clone)]
pub enum TSlice {
    None,
    Thread(ThreadRef),
    Slices(KObjectRef<TimeSlices>),
}

pub struct TimeSlices {
    pub slices: Vec<TSlice, KObjectArena>,
}
unsafe impl Send for TimeSlices {}

impl TimeSlices {
    pub unsafe fn create(page: usize) -> KObjectRef<TimeSlices> {
        let ct_ref = kobject_create!(TimeSlices, page);
        ct_ref
            .as_ptr()
            .write(TimeSlices {
                slices: Vec::new_in(ct_ref.meta().alloc.clone()),
            });

        ct_ref
    }

    pub fn push(&mut self, slice: TSlice) {
        self.slices.push(slice)
    }

    pub fn get_slice(&mut self, slice: usize) -> Option<&TSlice> {

        self.slices.get(slice)

        // if self.slices.is_empty() {
            // None
        // } else {
            // let old_hand = self.hand;
            // self.hand = (self.hand + 1) % self.slices.len();
            // Some(self.slices[old_hand].clone())
        // }
    }
}
