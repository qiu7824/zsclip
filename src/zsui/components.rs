#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ZsTabSpec {
    pub id: &'static str,
    pub label: &'static str,
}

impl ZsTabSpec {
    pub const fn new(id: &'static str, label: &'static str) -> Self {
        Self { id, label }
    }
}
