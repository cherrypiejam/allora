use alloc::vec::Vec;
use core::mem::size_of;

use super::{KObjectRef, KObjectArena, KObjectKind};
use super::{kobject_create, IsKObjectRef, INVALID_KOBJECT_REF};

use crate::mm::{pa, PAGE_SIZE};
use crate::KOBJECTS;

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
    pub unsafe fn create(pg: usize) -> KObjectRef {
        let ct_ref = kobject_create(KObjectKind::Container, pg);
        let ct_ptr = pa!(ct_ref) as *mut Container;

        ct_ptr.write(Container {
            slots: Vec::new_in(ct_ref.map_meta(|m| m.alloc.clone()).unwrap())
        });

        ct_ref
    }

    pub fn get_slot(&mut self) -> Option<usize> {
        if let Some(pos) = self.find_slot(INVALID_KOBJECT_REF) {
            Some(pos)
        } else {
            self.slots.push(INVALID_KOBJECT_REF); // NOTE: push may fail
                                                  // should not always succeed
            Some(self.slots.len() - 1)
        }
    }

    pub fn find_slot(&self, ko_ref: KObjectRef) -> Option<usize> {
        self.slots
            .iter()
            .position(|&slot| slot == ko_ref)

    }

    pub fn set_slot(&mut self, slot_id: usize, ko_ref: KObjectRef) {
        self.slots[slot_id] = ko_ref;
    }
}
