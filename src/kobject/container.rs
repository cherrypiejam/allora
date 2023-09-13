use alloc::vec::Vec;

use super::{KObjectRef, KObjectArena, KObjectPtr};
use super::{kobject_create, INVALID_KOBJ_ID};
use super::Label;
use super::Thread;
use super::ThreadRef;

use crate::collections::list::List;

#[derive(Clone)]
pub enum TimeSlice {
    Routine,
    Execute(ThreadRef),
    Redirect(KObjectRef<Container>),
}

pub struct Container {
    pub slots: Vec<KObjectPtr, KObjectArena>,
    pub scheduler: Option<KObjectRef<Thread>>,
    pub known_containers: Option<List<KObjectRef<Container>, KObjectArena>>,
    pub time_slices: Option<Vec<TimeSlice, KObjectArena>>,
}

unsafe impl Send for Container {}

impl Drop for Container {
    fn drop(&mut self) {
        // 1. update meta data
        // 2. destroy allocator
        // 3. return pages used by the allocator to its parent's free_pages
    }
}

impl Container {
    pub unsafe fn create(page: usize) -> KObjectRef<Container> {
        let ct_ref = kobject_create!(Container, page);
        ct_ref
            .as_ptr()
            .write(Container {
                slots: Vec::new_in(ct_ref.meta().alloc.clone()),
                scheduler: None,
                known_containers: None,
                time_slices: None,
            });

        ct_ref
    }

    pub fn get_slot(&mut self) -> Option<usize> {
        let invalid_koptr = unsafe { KObjectPtr::new(INVALID_KOBJ_ID) };
        if let Some(pos) = self.find_slot(invalid_koptr) {
            Some(pos)
        } else {
            self.slots.push(invalid_koptr); // NOTE: push may fail
                                            // should not always succeed
            Some(self.slots.len() - 1)
        }
    }

    pub fn find_slot(&self, ko_ref: KObjectPtr) -> Option<usize> {
        self.slots
            .iter()
            .position(|&slot| slot == ko_ref.into())

    }

    pub fn set_slot<T>(&mut self, slot_id: usize, ko_ref: KObjectRef<T>) {
        self.slots[slot_id] = ko_ref.into();
    }

    fn free() {}
}
