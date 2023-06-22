#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Label {
    Low,
    High,
}

impl Label {
    pub fn can_flow_to(&self, rhs: &Label) -> bool {
        self <= rhs
    }
}
