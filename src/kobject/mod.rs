use core::mem::size_of;
use core::marker::PhantomData;

mod container;
mod label;
mod thread;

pub use container::Container;
pub use label::Label;
pub use thread::{Thread, STACK_SIZE};

use crate::mm::page_tree::PageTree;
use crate::mm::koarena::KObjectArena;
use crate::mm::{pa, PAGE_SIZE};
use crate::KOBJECTS;

const INVALID_KOBJECT_ID: usize = usize::MAX;

pub struct ThreadRef<'a>(pub KObjectRef<'a, Thread>);
unsafe impl Send for ThreadRef<'_> {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KObjectKind {
    None,
    Container,
    Label,
    Thread,
}

// All metadata are stored in a global array, indexed by its page number
// All KO object are stored at the beginning of their page
pub struct KObjectMeta<'a, 'b> {
    pub koptr: KObjectPtr,
    pub parent: Option<KObjectRef<'a, Container>>,
    pub label: Option<KObjectRef<'b, Label>>,
    pub alloc: KObjectArena, // if oom, get one page from its page tree
    pub kind: KObjectKind,
    pub free_pages: PageTree,
}

impl KObjectMeta<'_, '_> {
    pub fn empty() -> Self {
        KObjectMeta {
            koptr: unsafe { KObjectPtr::null() },
            parent: None,
            label: None,
            alloc: KObjectArena::empty(),
            kind: KObjectKind::None,
            free_pages: PageTree::empty(),
        }
    }
}


// KObjectPtr is essentially a void pointer pointing to a page. Because of that,
// it is considered to be safe. Casting from KObjectPtr to KObjectRef may fail
//
// KObjectRef is a typed reference to a kobject. It suppose to always point
// to a valid kobject. Casting it back to KObjectPtr should never fail
//
// Current implementation is faulty in following ways:
// 1. Ref Clone vs Copy
// 2. Ref doesn't guarentee it always pointing to a valid object.
//      What if an kernel object is freed?
// 3. Ref doesn't ensure its type matches its meta data. This is because accessing
//      meta data acquires a lock, which may lead to a deadlock


#[derive(Clone, Copy, PartialEq)]
pub struct KObjectPtr {
    id: usize
}

impl KObjectPtr {
    pub unsafe fn new(id: usize) -> Self {
        KObjectPtr { id }
    }

    pub unsafe fn null() -> Self {
        KObjectPtr::new(INVALID_KOBJECT_ID)
    }

    pub fn is_null(&self) -> bool {
        if self.id == INVALID_KOBJECT_ID {
            true
        } else {
            false
        }
    }

    pub fn map_meta<U, F: FnOnce(&mut KObjectMeta) -> U>(&self, f: F) -> Option<U> {
        if self.is_null() {
            None
        } else {
            KOBJECTS
                .map(|(ks, ofs)| {
                    let id = self.id - *ofs;
                    f(&mut ks[id])
                })
        }
    }
}

impl<T> From<KObjectRef<'_, T>> for KObjectPtr {
    fn from(value: KObjectRef<'_, T>) -> Self {
        KObjectPtr { id: value.id }
    }
}

// #[derive(Clone)]
pub struct KObjectRef<'a, T> {
    id: usize,
    _type: PhantomData<&'a T>
}

// impl<T: Clone> Copy for KObjectRef<T> {}
impl<T> Clone for KObjectRef<'_, T> {
    fn clone(&self) -> Self {
        unsafe {
            KObjectRef::new(self.id)
        }
    }
}
impl<T> Copy for KObjectRef<'_, T> {}

impl<T> KObjectRef<'_, T> {
    pub unsafe fn new(id: usize) -> Self {
        KObjectRef { id, _type: PhantomData }
    }

    pub fn as_ptr(&self) -> *mut T {
        pa!(self.id) as *mut T
    }

    pub fn as_ref(&self) -> &T {
        unsafe { &*self.as_ptr() }
    }

    pub fn as_mut(&self) -> &mut T {
        unsafe { &mut *self.as_ptr() }
    }

    pub fn map_meta<U, F: FnOnce(&mut KObjectMeta) -> U>(&self, f: F) -> Option<U> {
        KObjectPtr::from(*self)
            .map_meta(f)
    }
}


macro_rules! impl_from_koptr_for_koref {
    ($t: ident) => {
        impl From<KObjectPtr> for KObjectRef<'_, $t> {
            fn from(value: KObjectPtr) -> Self {
                unsafe {
                    KObjectRef::new(value.id)
                }
                // let kind = value
                    // .map_meta(|ko_meta| ko_meta.kind); // can have deadlock
                // match kind {
                    // Some(KObjectKind::$t) => unsafe {
                        // KObjectRef::new(value.id)
                    // }
                    // _ => panic!("invalid kobject pointer")
                // }
            }
        }
    };
}

impl_from_koptr_for_koref!(Container);
impl_from_koptr_for_koref!(Thread);
impl_from_koptr_for_koref!(Label);



macro_rules! kobject_create {
    ($kind: ident, $page_id: expr) => {
        crate::kobject::__kobject_create::<$kind>(KObjectKind::$kind, $page_id)
    };
}

pub(crate) use kobject_create;

unsafe fn __kobject_create<'a, T>(kind: KObjectKind, page_id: usize) -> KObjectRef<'a, T>
where
    KObjectRef<'a, T>: From<KObjectPtr>
{
    KOBJECTS.map(|(kobjs, ofs)| {
        let id = page_id - *ofs;

        assert!(kobjs[id].kind == KObjectKind::None);

        kobjs[id] = KObjectMeta {
            koptr: KObjectPtr::new(page_id),
            parent: None,
            label: None,
            alloc: KObjectArena::new(
                pa!(page_id) + size_of::<T>(),
                PAGE_SIZE - size_of::<T>()
            ),
            kind,
            free_pages: PageTree::empty(),
        };

        kobjs[id].koptr.into()
    })
    .unwrap()
}
