use core::mem::size_of;

mod container;
mod label;
mod thread;

pub use container::Container;
pub use label::Label;
pub use thread::Thread;

use crate::label::Buckle;
use crate::mm::page_tree::PageTree;
use crate::mm::koarena::KObjectArena;
use crate::mm::{pa, PAGE_SIZE};
use crate::KOBJECTS;

pub type KObjectRef = usize;
const INVALID_KOBJECT_REF: usize = usize::MAX;

// All metadata are stored in a global array, indexed by its page number
// All KO object are stored at the beginning of their page
pub struct KObjectMeta {
    pub id: KObjectRef,
    pub parent: Option<KObjectRef>,
    pub label: Option<Buckle<KObjectArena>>, // FIXME: reference or object?
    pub alloc: KObjectArena, // if oom, get one page from its page tree
    pub kind: KObjectKind,
    pub free_pages: PageTree,
}

pub enum KObjectKind {
    None,
    Container,
    Label,
    Thread,
    Page(KObjectRef),
}

// Safety: ?
pub trait IsKObjectRef {
    fn map_meta<U, F: FnOnce(&mut KObjectMeta) -> U>(&self, f: F) -> Option<U>;
    // fn meta(&self) -> &'a KObjectMeta;
    // fn meta_mut(&self) -> &'a mut KObjectMeta;
    // fn container(&self) -> &'a Container;
    // fn container_mut(&mut self) -> &'a mut Container;
}

impl<'a> IsKObjectRef for KObjectRef {

    fn map_meta<U, F: FnOnce(&mut KObjectMeta) -> U>(&self, f: F) -> Option<U> {
        KOBJECTS
            .map(|(ks, ofs)| {
                let id = *self - *ofs;
                f(&mut ks[id])
            })
    }


    // fn meta(&self) -> &'a KObjectMeta {
        // KOBJECTS
            // .map(|(ks, ofs)| {
                // let id = *self - *ofs;
                // &ks[id]
            // })
            // .unwrap()
    // }

    // fn meta_mut(&self) -> &'a mut KObjectMeta {
        // KOBJECTS
            // .map(|(ks, ofs)| {
                // let id = *self - *ofs;
                // &mut ks[id]
            // })
            // .unwrap()
    // }

    // fn container(&self) -> &'a Container {
        // unsafe {
            // (pa!(*self) as *mut Container)
                // .as_ref()
                // .unwrap()
        // }
    // }
}

fn is_valid_kobj(koref: KObjectRef) -> bool {
    KOBJECTS
        .lock()
        .as_mut()
        .and_then(|(ks, ofs)| {
            let index = koref - *ofs;
            if index < ks.len() {
                match ks[index].kind {
                    KObjectKind::None | KObjectKind::Page(_) => None,
                    _ => Some(()),
                }
            } else {
                None
            }
        })
        .is_some()
}

unsafe fn kobject_create(kind: KObjectKind, page: usize) -> KObjectRef {
    KOBJECTS.map(|(ks, ofs)| {
        let ct_id = page - *ofs;
        let ct_meta = &mut ks[ct_id];

        // ct_meta.label =

        match kind {
            KObjectKind::Container => {
                ct_meta.kind = KObjectKind::Container;
                ct_meta.alloc.as_mut().lock().append(
                    pa!(page) + size_of::<Container>(),
                    PAGE_SIZE - size_of::<Container>(),
                );
            }
            KObjectKind::Thread => {
                ct_meta.kind = KObjectKind::Thread;
                ct_meta.alloc.as_mut().lock().append(
                    pa!(page) + size_of::<Thread>(),
                    PAGE_SIZE - size_of::<Thread>(),
                );
            }
            _ => unimplemented!(),
        }
    });

    page
}
