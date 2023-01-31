use crate::common::{Bit, Msb};

pub enum Instruction {
    Left,
    Right,
}

impl From<Bit> for Instruction {
    fn from(bit: Bit) -> Self {
        match bit {
            Bit::_0 => Instruction::Left,
            Bit::_1 => Instruction::Right,
        }
    }
}

pub trait Path {
    fn get_instruction(&self, index: usize) -> Option<Instruction>;
}

pub trait ComparablePath {
    fn common_path_length(&self, other: &Self) -> usize;
}

impl<T> Path for T
where
    T: Msb,
{
    fn get_instruction(&self, index: usize) -> Option<Instruction> {
        self.get_bit_at_index_from_msb(index).map(Into::into)
    }
}

impl<T> ComparablePath for T
where
    T: Msb,
{
    fn common_path_length(&self, other: &Self) -> usize {
        self.common_prefix_count(other)
    }
}
