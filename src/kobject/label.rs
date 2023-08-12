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
}

impl IsLabel for Label {
    fn lub(self, rhs: Self) -> Self {
        // TODO: problem?
        Label {
            inner: self.inner.lub(rhs.inner)
        }
    }

    fn glb(self, rhs: Self) -> Self {
        Label {
            inner: self.inner.glb(rhs.inner)
        }
    }

    fn can_flow_to(&self, rhs: &Self) -> bool {
        self.inner.can_flow_to(&rhs.inner)
    }
}

impl HasPrivilege for Label {
    type Privilege = Component<KObjectArena>;

    fn downgrade(self, privilege: &Self::Privilege) -> Self {
        todo!()
    }

    fn downgrade_to(self, target: Self, privilege: &Self::Privilege) -> Self {
        todo!()
    }

    fn can_flow_to_with_privilege(&self, rhs: &Self, privilege: &Self::Privilege) -> bool {
        todo!()
    }
}
