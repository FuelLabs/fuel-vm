use crate::*;
use core::borrow::Borrow;
use core::fmt::Display;
use core::ops::DerefMut;

/// Base map trait for Fuel infrastructure
///
/// Generics:
///
/// - K: Key that maps to a value
/// - V: Stored value
/// - R: Wrapper type that borrows V. V can be used as blanket implementation `Borrow<V> for V`
/// - E: Error type that implements Display
pub trait Storage<K, V, R, E>
where
    R: Borrow<V>,
    E: Display,
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

    /// Retrieve `R` such as `K->V, R: Borrow<V>`.
    ///
    /// `R` is used instead of `V` to allow reference to inlined values in the storage
    /// implementation instead of copying them.
    fn get(&self, key: &K) -> Result<Option<R>, E>;

    /// Return `true` if there is a `K` mapping to a value in the storage.
    fn contains_key(&self, key: &K) -> Result<bool, E>;
}

impl<K, V, R, S, I, E> Storage<K, V, R, E> for I
where
    R: Borrow<V>,
    E: Display,
    S: Storage<K, V, R, E>,
    I: DerefMut<Target = S>,
{
    fn insert(&mut self, key: &K, value: &V) -> Result<Option<V>, E> {
        <S as Storage<K, V, R, E>>::insert(self.deref_mut(), key, value)
    }

    fn remove(&mut self, key: &K) -> Result<Option<V>, E> {
        <S as Storage<K, V, R, E>>::remove(self.deref_mut(), key)
    }

    fn get(&self, key: &K) -> Result<Option<R>, E> {
        <S as Storage<K, V, R, E>>::get(self.deref(), key)
    }

    fn contains_key(&self, key: &K) -> Result<bool, E> {
        <S as Storage<K, V, R, E>>::contains_key(self.deref(), key)
    }
}

/// Base trait for Fuel Merkle storage
///
/// Generics:
///
/// - P: Domain of the merkle tree
/// - K: Key that maps to a value
/// - V: Stored value
/// - R: Wrapper type that borrows V. V can be used as blanket implementation `Borrow<V> for V`
/// - E: Error type that implements Display
pub trait MerkleStorage<P, K, V, R, E>
where
    R: Borrow<V>,
    E: Display,
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

    /// Retrieve `R` such as `P->K->V, R: Borrow<V>`.
    ///
    /// `R` is used instead of `V` to allow reference to inlined values in the storage
    /// implementation instead of copying them.
    fn get(&self, parent: &P, key: &K) -> Result<Option<R>, E>;

    /// Return `true` if there is a `P->K` mapping to a value in the storage.
    fn contains_key(&self, parent: &P, key: &K) -> Result<bool, E>;

    /// Return the merkle root of the domain of `P`.
    ///
    /// The cryptographic primitive is an arbitrary choice of the implementor and this trait won't
    /// impose any restrictions to that.
    fn root(&mut self, parent: &P) -> Result<Bytes32, E>;
}

impl<P, K, V, R, X, I, E> MerkleStorage<P, K, V, R, E> for I
where
    R: Borrow<V>,
    E: Display,
    X: MerkleStorage<P, K, V, R, E>,
    I: DerefMut<Target = X>,
{
    fn insert(&mut self, parent: &P, key: &K, value: &V) -> Result<Option<V>, E> {
        <X as MerkleStorage<P, K, V, R, E>>::insert(self.deref_mut(), parent, key, value)
    }

    fn remove(&mut self, parent: &P, key: &K) -> Result<Option<V>, E> {
        <X as MerkleStorage<P, K, V, R, E>>::remove(self.deref_mut(), parent, key)
    }

    fn get(&self, parent: &P, key: &K) -> Result<Option<R>, E> {
        <X as MerkleStorage<P, K, V, R, E>>::get(self.deref(), parent, key)
    }

    fn contains_key(&self, parent: &P, key: &K) -> Result<bool, E> {
        <X as MerkleStorage<P, K, V, R, E>>::contains_key(self.deref(), parent, key)
    }

    fn root(&mut self, parent: &P) -> Result<Bytes32, E> {
        <X as MerkleStorage<P, K, V, R, E>>::root(self.deref_mut(), parent)
    }
}
