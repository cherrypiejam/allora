use labeled::buckle2::{Buckle2 as Buckle, Component};
use labeled::{Label as IsLabel, HasPrivilege};

use super::{KObjectRef, KObjectArena};
use super::kobject_create;

pub struct Label {
    inner: Buckle<KObjectArena>,
}

impl Label {
    pub unsafe fn create(pg: usize, input: &str) -> KObjectRef<Label> {
        let lb_ref = kobject_create!(Label, pg);

        lb_ref.map_meta(|lb_meta| {
            lb_ref.as_ptr().write(Label {
                inner: Buckle::parse_in(input, lb_meta.alloc.clone()).unwrap(),
            });
        });

        lb_ref
    }

    // IsLabel and HasPrivilege contain trait functions that consume the struct
    // We write our own here because it requires extra custom allocator for
    // the allocation.
    // TODO: add these methods when needed

    pub fn can_flow_to(&self, rhs: &Self) -> bool {
        self.inner.can_flow_to(&rhs.inner)
    }

    pub fn can_flow_to_with_privilege(&self, rhs: &Self, privilege: &Privilege) -> bool {
        self.inner.can_flow_to_with_privilege(&rhs.inner, &privilege.inner)
    }

    // fn lub() {}
    // fn glb() {}
    // fn downgrade() {}
    // fn downgrade_to() {}
}

pub struct Privilege {
    inner: Component<KObjectArena>,
}
