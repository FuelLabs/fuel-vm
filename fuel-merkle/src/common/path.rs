use crate::common::{
    Bit,
    Msb,
};

/// The side of a child node in a binary tree.
pub enum Side {
    Left,
    Right,
}

impl From<Bit> for Side {
    fn from(bit: Bit) -> Self {
        match bit {
            Bit::_0 => Side::Left,
            Bit::_1 => Side::Right,
        }
    }
}

pub trait Path {
    /// Which child node to follow at the given index.
    fn get_instruction(&self, index: u32) -> Option<Side>;

    fn common_path_length(&self, other: &[u8]) -> u32;
}

impl<T> Path for T
where
    T: Msb,
{
    fn get_instruction(&self, index: u32) -> Option<Side> {
        self.get_bit_at_index_from_msb(index).map(Into::into)
    }

    fn common_path_length(&self, other: &[u8]) -> u32 {
        self.common_prefix_count(other)
    }
}
