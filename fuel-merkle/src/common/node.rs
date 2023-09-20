use crate::common::{
    Bytes32,
    Bytes8,
};

use alloc::string::String;
use core::{
    fmt,
    mem,
};

pub trait KeyFormatting {
    type PrettyType: fmt::Display;

    fn pretty(&self) -> Self::PrettyType;
}

pub trait Node {
    type Key: KeyFormatting;

    fn key_size_in_bits() -> usize {
        mem::size_of::<Self::Key>() * 8
    }

    fn height(&self) -> u32;
    fn leaf_key(&self) -> Self::Key;
    fn is_leaf(&self) -> bool;
    fn is_node(&self) -> bool;
}

pub trait ParentNode: Sized + Node {
    type Error;

    fn left_child(&self) -> ChildResult<Self>;
    fn right_child(&self) -> ChildResult<Self>;
}

#[allow(type_alias_bounds)]
pub type ChildResult<T: ParentNode> = Result<T, ChildError<T::Key, T::Error>>;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
pub enum ChildError<Key, E>
where
    Key: KeyFormatting,
{
    #[cfg_attr(feature = "std", error("Child with key {} was not found in storage", .0.pretty()))]
    ChildNotFound(Key),
    #[cfg_attr(feature = "std", error("Node is a leaf with no children"))]
    NodeIsLeaf,
    #[cfg_attr(feature = "std", error(transparent))]
    Error(E),
}

impl<Key, E> From<E> for ChildError<Key, E>
where
    Key: KeyFormatting,
{
    fn from(e: E) -> Self {
        Self::Error(e)
    }
}

impl KeyFormatting for Bytes8 {
    type PrettyType = u64;

    fn pretty(&self) -> Self::PrettyType {
        u64::from_be_bytes(*self)
    }
}

impl KeyFormatting for Bytes32 {
    type PrettyType = String;

    fn pretty(&self) -> Self::PrettyType {
        hex::encode(self)
    }
}
