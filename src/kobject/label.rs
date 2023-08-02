use labeled::buckle2::Buckle2 as Buckle;
use super::{KObjectRef, KObjectArena};

pub struct Label {
    pub inner: Buckle<KObjectArena>,
}
