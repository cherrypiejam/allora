mod container;
pub use container::Container;

use crate::label::Buckle;
use crate::mm::page_tree::PageTree;
use crate::mm::koarena::KObjectArena;

type KObjectRef = usize;

// all metadata are stored in a global array, indexed by its page number
// all KO object are stored at the beginning of pages
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
}

pub enum KObjectType {
    Container(KObjectRef),
}

