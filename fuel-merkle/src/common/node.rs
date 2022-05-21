use core::mem;

pub trait Node {
    type Key;

    fn key_size_in_bits() -> usize {
        mem::size_of::<Self::Key>() * 8
    }

    fn height(&self) -> u32;
    fn leaf_key(&self) -> Self::Key;
    fn is_leaf(&self) -> bool;
}

pub trait ParentNode: Node {
    fn left_child(&self) -> Self;
    fn right_child(&self) -> Self;
}
