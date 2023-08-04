mod container;
mod label;
mod thread;

pub use container::Container;
pub use label::Label;
pub use thread::Thread;

use crate::label::Buckle;
use crate::mm::page_tree::PageTree;
use crate::mm::koarena::KObjectArena;
use crate::mm::pa;
use crate::KOBJECTS;

type KObjectRef = usize;

// All metadata are stored in a global array, indexed by its page number
// All KO object are stored at the beginning of their page
pub struct KObjectMeta {
    pub id: KObjectRef,
    pub parent: Option<KObjectRef>,
    pub label: Option<Buckle<KObjectArena>>,
    pub alloc: KObjectArena, // if oom, get one page from its page tree
    pub kind: KObjectKind,
    pub free_pages: PageTree,
}

pub enum KObjectKind {
    NoType,
    Container,
    Label,
    Thread,
    Page(KObjectRef),
}

pub enum KObjectType {
    Container(KObjectRef),
}

// Safety: ?
trait IsKObjectRef<'a> {
    fn meta(&self) -> &'a KObjectMeta;
    fn meta_mut(&self) -> &'a mut KObjectMeta;
    fn container(&self) -> &'a Container;
    // fn container_mut(&mut self) -> &'a mut Container;
}

impl<'a> IsKObjectRef<'a> for KObjectRef {
    fn meta(&self) -> &'a KObjectMeta {
        todo!()
    }

    fn meta_mut(&self) -> &'a mut KObjectMeta {
        todo!()
    }

    fn container(&self) -> &'a Container {
        unsafe {
            (pa!(*self) as *mut Container)
                .as_ref()
                .unwrap()
        }
    }
}

fn is_valid_kobj(koref: KObjectRef) -> bool {
    KOBJECTS
        .lock()
        .as_mut()
        .and_then(|(ks, ofs)| {
            let index = koref - *ofs;
            if index < ks.len() {
                match ks[index].kind {
                    KObjectKind::NoType | KObjectKind::Page(_) => None,
                    _ => Some(()),
                }
            } else {
                None
            }
        })
        .is_some()
}
