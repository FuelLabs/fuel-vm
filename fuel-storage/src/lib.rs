use std::borrow::Cow;
use std::error::Error;

use auto_impl::auto_impl;

/// Merkle root alias type
pub type MerkleRoot = [u8; 32];

/// Base map trait for Fuel infrastructure
///
/// Generics:
///
/// - K: Key that maps to a value
/// - V: Stored value
#[auto_impl(&mut)]
pub trait Storage<K, V>
where
    V: Clone,
{
    /// Error implementation of the storage functions
    type Error: Error;

    /// Append `K->V` mapping to the storage.
    ///
    /// If `K` was already mappped to a value, return the replaced value as `Ok(Some(V))`. Return
    /// `Ok(None)` otherwise.
    fn insert(&mut self, key: &K, value: &V) -> Result<Option<V>, Self::Error>;

    /// Remove `K->V` mapping from the storage.
    ///
    /// Return `Ok(Some(V))` if the value was present. If the key wasn't found, return
    /// `Ok(None)`.
    fn remove(&mut self, key: &K) -> Result<Option<V>, Self::Error>;

    /// Retrieve `Cow<V>` such as `K->V`.
    fn get<'a>(&'a self, key: &K) -> Result<Option<Cow<'a, V>>, Self::Error>;

    /// Return `true` if there is a `K` mapping to a value in the storage.
    fn contains_key(&self, key: &K) -> Result<bool, Self::Error>;
}

/// Base trait for Fuel Merkle storage
///
/// Generics:
///
/// - P: Domain of the merkle tree
/// - K: Key that maps to a value
/// - V: Stored value
#[auto_impl(&mut)]
pub trait MerkleStorage<P, K, V>
where
    V: Clone,
{
    /// Error implementation of the merkle storage functions
    type Error: Error;

    /// Append `P->K->V` mapping to the storage.
    ///
    /// If `K` was already mappped to a value, return the replaced value as `Ok(Some(V))`. Return
    /// `Ok(None)` otherwise.
    fn insert(&mut self, parent: &P, key: &K, value: &V) -> Result<Option<V>, Self::Error>;

    /// Remove `P->K->V` mapping from the storage.
    ///
    /// Return `Ok(Some(V))` if the value was present. If the key wasn't found, return
    /// `Ok(None)`.
    fn remove(&mut self, parent: &P, key: &K) -> Result<Option<V>, Self::Error>;

    /// Retrieve `Cow<V>` such as `P->K->V`.
    fn get<'a>(&'a self, parent: &P, key: &K) -> Result<Option<Cow<'a, V>>, Self::Error>;

    /// Return `true` if there is a `P->K` mapping to a value in the storage.
    fn contains_key(&self, parent: &P, key: &K) -> Result<bool, Self::Error>;

    /// Return the merkle root of the domain of `P`.
    ///
    /// The cryptographic primitive is an arbitrary choice of the implementor and this trait won't
    /// impose any restrictions to that.
    fn root(&mut self, parent: &P) -> Result<MerkleRoot, Self::Error>;
}
