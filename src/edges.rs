#[repr(u8)]
#[derive(Copy, Clone)]
pub enum Edge {
    Top = 1,
    Right = 1 << 1,
    Bottom = 1 << 2,
    Left = 1 << 3,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Edges(u8);

impl Edges {
    pub(super) fn new(edges: &[Edge]) -> Self {
        let mut val = 0u8;
        for edge in edges.iter() {
            val |= *edge as u8;
        }

        Self(val)
    }

    pub fn has_edge(&self, edge: Edge) -> bool {
        self.0 & edge as u8 > 0
    }
}
