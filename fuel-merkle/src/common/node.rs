use crate::common::{Bytes32, Bytes8};
use core::fmt::{Display, Formatter};
use core::mem;

pub trait Node {
    type Key;

    fn key_size_in_bits() -> usize {
        mem::size_of::<Self::Key>() * 8
    }

    fn height(&self) -> u32;
    fn leaf_key(&self) -> Self::Key;
    fn is_leaf(&self) -> bool;
    fn is_node(&self) -> bool;
}

pub trait ParentNode: Node
where
    Self: Sized,
    <Self as Node>::Key: Copy,
    ChildErrorKey<<Self as Node>::Key>: Display,
{
    type Error;

    fn left_child(&self) -> ChildResult<Self>;
    fn right_child(&self) -> ChildResult<Self>;
}

#[allow(type_alias_bounds)]
pub type ChildResult<T: ParentNode> = Result<T, ChildError<T::Key, T::Error>>;

pub struct ChildErrorKey<Key>(Key);

impl Display for ChildErrorKey<Bytes8> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", u64::from_be_bytes(self.0))
    }
}

impl Display for ChildErrorKey<Bytes32> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
pub enum ChildError<Key, E>
where
    Key: Copy,
    ChildErrorKey<Key>: Display,
{
    #[cfg_attr(feature = "std", error("Child with key {} was not found in storage", ChildErrorKey(*.0)))]
    ChildNotFound(Key),
    #[cfg_attr(feature = "std", error("Node is a leaf with no children"))]
    NodeIsLeaf,
    #[cfg_attr(feature = "std", error(transparent))]
    Error(E),
}

impl<Key, E> From<E> for ChildError<Key, E>
where
    Key: Copy,
    ChildErrorKey<Key>: Display,
{
    fn from(e: E) -> Self {
        Self::Error(e)
    }
}
