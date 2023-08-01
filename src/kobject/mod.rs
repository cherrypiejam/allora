mod container;
pub use container::Container;

use core::alloc::Allocator;

use crate::label::Buckle;
use crate::memory::page_tree::PageTree;

type KObjectRef = usize;

// all metadata are stored in a global array, indexed by its page number
// all KO object are stored at the beginning of pages
pub struct KObjectMeta<A: Allocator + Clone> {
    pub id: KObjectRef,
    pub parent: Option<KObjectRef>,
    pub label: Option<Buckle<A>>,
    pub alloc: Option<A>, // if oom, get one page from its page tree
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

