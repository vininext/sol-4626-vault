use bytemuck::{Pod, Zeroable};

#[derive(Clone, Copy, PartialEq, Eq, Default, Pod, Zeroable)]
#[repr(transparent)]
pub struct BoolU8(pub u8);

impl BoolU8 {
    pub const TRUE: u8 = 1;
    pub const FALSE: u8 = 0;

    pub fn new(value: bool) -> Self {
        Self(if value { Self::TRUE } else { Self::FALSE })
    }

    pub fn is_true(&self) -> bool {
        self.0 == Self::TRUE
    }
}
