use crate::*;

use std::borrow::Cow;
use std::error::Error;
use std::ops::{Deref, DerefMut};

/// Base map trait for Fuel infrastructure
///
/// Generics:
///
/// - K: Key that maps to a value
/// - V: Stored value
/// - E: Error type that implements Display
pub trait Storage<K, V, E>
where
    V: Clone,
    E: Error,
{
    /// Append `K->V` mapping to the storage.
    ///
    /// If `K` was already mappped to a value, return the replaced value as `Ok(Some(V))`. Return
    /// `Ok(None)` otherwise.
    fn insert(&mut self, key: &K, value: &V) -> Result<Option<V>, E>;

    /// Remove `K->V` mapping from the storage.
    ///
    /// Return `Ok(Some(V))` if the value was present. If the key wasn't found, return
    /// `Ok(None)`.
    fn remove(&mut self, key: &K) -> Result<Option<V>, E>;

    /// Retrieve `Cow<V>` such as `K->V`.
    fn get<'a>(&'a self, key: &K) -> Result<Option<Cow<'a, V>>, E>;

    /// Return `true` if there is a `K` mapping to a value in the storage.
    fn contains_key(&self, key: &K) -> Result<bool, E>;
}

impl<K, V, E, S> Storage<K, V, E> for &mut S
where
    V: Clone,
    E: Error,
    S: Storage<K, V, E>,
{
    fn insert(&mut self, key: &K, value: &V) -> Result<Option<V>, E> {
        <S as Storage<K, V, E>>::insert(self.deref_mut(), key, value)
    }

    fn remove(&mut self, key: &K) -> Result<Option<V>, E> {
        <S as Storage<K, V, E>>::remove(self.deref_mut(), key)
    }

    fn get(&self, key: &K) -> Result<Option<Cow<'_, V>>, E> {
        <S as Storage<K, V, E>>::get(self.deref(), key)
    }

    fn contains_key(&self, key: &K) -> Result<bool, E> {
        <S as Storage<K, V, E>>::contains_key(self.deref(), key)
    }
}

/// Base trait for Fuel Merkle storage
///
/// Generics:
///
/// - P: Domain of the merkle tree
/// - K: Key that maps to a value
/// - V: Stored value
/// - E: Error type that implements Display
pub trait MerkleStorage<P, K, V, E>
where
    V: Clone,
    E: Error,
{
    /// Append `P->K->V` mapping to the storage.
    ///
    /// If `K` was already mappped to a value, return the replaced value as `Ok(Some(V))`. Return
    /// `Ok(None)` otherwise.
    fn insert(&mut self, parent: &P, key: &K, value: &V) -> Result<Option<V>, E>;

    /// Remove `P->K->V` mapping from the storage.
    ///
    /// Return `Ok(Some(V))` if the value was present. If the key wasn't found, return
    /// `Ok(None)`.
    fn remove(&mut self, parent: &P, key: &K) -> Result<Option<V>, E>;

    /// Retrieve `Cow<V>` such as `P->K->V`.
    fn get<'a>(&'a self, parent: &P, key: &K) -> Result<Option<Cow<'a, V>>, E>;

    /// Return `true` if there is a `P->K` mapping to a value in the storage.
    fn contains_key(&self, parent: &P, key: &K) -> Result<bool, E>;

    /// Return the merkle root of the domain of `P`.
    ///
    /// The cryptographic primitive is an arbitrary choice of the implementor and this trait won't
    /// impose any restrictions to that.
    fn root(&mut self, parent: &P) -> Result<Bytes32, E>;
}

impl<P, K, V, E, S> MerkleStorage<P, K, V, E> for &mut S
where
    V: Clone,
    E: Error,
    S: MerkleStorage<P, K, V, E>,
{
    fn insert(&mut self, parent: &P, key: &K, value: &V) -> Result<Option<V>, E> {
        <S as MerkleStorage<P, K, V, E>>::insert(self.deref_mut(), parent, key, value)
    }

    fn remove(&mut self, parent: &P, key: &K) -> Result<Option<V>, E> {
        <S as MerkleStorage<P, K, V, E>>::remove(self.deref_mut(), parent, key)
    }

    fn get(&self, parent: &P, key: &K) -> Result<Option<Cow<'_, V>>, E> {
        <S as MerkleStorage<P, K, V, E>>::get(self.deref(), parent, key)
    }

    fn contains_key(&self, parent: &P, key: &K) -> Result<bool, E> {
        <S as MerkleStorage<P, K, V, E>>::contains_key(self.deref(), parent, key)
    }

    fn root(&mut self, parent: &P) -> Result<Bytes32, E> {
        <S as MerkleStorage<P, K, V, E>>::root(self.deref_mut(), parent)
    }
}
