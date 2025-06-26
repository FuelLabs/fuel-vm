#![no_std]
#![deny(
    clippy::arithmetic_side_effects,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::string_slice
)]
#![deny(unsafe_code)]
#![deny(unused_crate_dependencies)]

mod impls;

extern crate alloc;

use core::borrow::Borrow;

use alloc::{
    borrow::{
        Cow,
        ToOwned,
    },
    collections::BTreeMap,
    vec::Vec,
};
use core::ops::Bound::{
    Excluded,
    Unbounded,
};

/// Merkle root alias type
pub type MerkleRoot = [u8; 32];

/// Mappable type with `Key` and `Value`.
///
/// # Example
///
/// ```rust
/// use fuel_storage::Mappable;
/// pub struct Contract;
///
/// impl Mappable for Contract {
///     /// The `[u8; 32]` is a primitive type, so we can't optimize it more.
///     type Key = Self::OwnedKey;
///     type OwnedKey = [u8; 32];
///     /// It is optimized to use slice instead of vector.
///     type Value = [u8];
///     type OwnedValue = Vec<u8>;
/// }
/// ```
pub trait Mappable {
    /// The key type is used during interaction with the storage. In most cases, it is the
    /// same as `Self::OwnedKey`.
    type Key: ?Sized + ToOwned;
    /// The owned type of the `Key` retrieving from the storage.
    type OwnedKey: From<<Self::Key as ToOwned>::Owned> + Borrow<Self::Key> + Clone;
    /// The value type is used while setting the value to the storage. In most cases, it
    /// is the same as `Self::OwnedValue`, but it is without restriction and can be
    /// used for performance optimizations.
    type Value: ?Sized + ToOwned;
    /// The owned type of the `Value` retrieving from the storage.
    type OwnedValue: From<<Self::Value as ToOwned>::Owned> + Borrow<Self::Value> + Clone;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
/// The direction of the key retrieval in the storage.
pub enum Direction {
    /// The next key in the storage.
    Next,
    /// The previous key in the storage.
    Previous,
}

impl Direction {
    /// Returns the next key and value from the `BTreeMap` based on the `start_key` and
    /// `direction`.
    pub fn next_from_map<'a, T, K, V>(
        &self,
        start_key: &T,
        map: &'a BTreeMap<K, V>,
    ) -> Option<(Cow<'a, K>, Cow<'a, V>)>
    where
        T: Ord + ?Sized,
        K: Borrow<T> + Ord + Clone,
        V: Clone,
    {
        let entry = match self {
            Direction::Next => map.range((Excluded(start_key), Unbounded)).next(),
            Direction::Previous => {
                map.range((Unbounded, Excluded(start_key))).next_back()
            }
        };

        let entry = entry.map(|(key, value)| (Cow::Borrowed(key), Cow::Borrowed(value)));

        entry
    }
}

/// Base read storage trait for Fuel infrastructure.
///
/// Generic should implement [`Mappable`] trait with all storage type information.
pub trait StorageInspect<Type: Mappable> {
    type Error;

    /// Retrieve `Cow<Value>` such as `Key->Value`.
    fn get(&self, key: &Type::Key) -> Result<Option<Cow<Type::OwnedValue>>, Self::Error>;

    /// Retrieves the next key and value after `start_key` in a `direction`.
    #[allow(clippy::type_complexity)]
    fn get_next(
        &self,
        start_key: &Type::Key,
        direction: Direction,
    ) -> Result<Option<(Cow<Type::OwnedKey>, Cow<Type::OwnedValue>)>, Self::Error>;

    /// Return `true` if there is a `Key` mapping to a value in the storage.
    fn contains_key(&self, key: &Type::Key) -> Result<bool, Self::Error>;
}

/// Base storage trait for Fuel infrastructure.
///
/// Generic should implement [`Mappable`] trait with all storage type information.
pub trait StorageMutate<Type: Mappable>: StorageInspect<Type> {
    /// Append `Key->Value` mapping to the storage.
    fn insert(
        &mut self,
        key: &Type::Key,
        value: &Type::Value,
    ) -> Result<(), Self::Error> {
        self.replace(key, value).map(|_| ())
    }

    /// Append `Key->Value` mapping to the storage.
    ///
    /// If `Key` was already mappped to a value, return the replaced value as
    /// `Ok(Some(Value))`. Return `Ok(None)` otherwise.
    fn replace(
        &mut self,
        key: &Type::Key,
        value: &Type::Value,
    ) -> Result<Option<Type::OwnedValue>, Self::Error>;

    /// Remove `Key->Value` mapping from the storage.
    fn remove(&mut self, key: &Type::Key) -> Result<(), Self::Error> {
        self.take(key).map(|_| ())
    }

    /// Remove `Key->Value` mapping from the storage.
    ///
    /// Return `Ok(Some(Value))` if the value was present. If the key wasn't found, return
    /// `Ok(None)`.
    fn take(&mut self, key: &Type::Key) -> Result<Option<Type::OwnedValue>, Self::Error>;
}

/// Base storage trait for Fuel infrastructure.
///
/// Allows checking the size of the value stored at a given key.
/// Checking the size of a value is a cheap operation and should not require
/// copying the value into a buffer.
pub trait StorageSize<Type: Mappable>: StorageInspect<Type> {
    /// Return the number of bytes stored at this key.
    fn size_of_value(&self, key: &Type::Key) -> Result<Option<usize>, Self::Error>;
}

/// Base storage trait for Fuel infrastructure.
///
/// Allows reading the raw bytes of the value stored at a given key
/// into a provided buffer.
///
/// This trait should skip any deserialization and simply copy the raw bytes.
pub trait StorageRead<Type: Mappable>: StorageInspect<Type> + StorageSize<Type> {
    /// Read the value stored at the given key into the provided buffer if the value
    /// exists. Errors if the buffer cannot be filled completely, or if attempting
    /// to read past the end of the value.
    ///
    /// Does not perform any deserialization.
    ///
    /// Returns `Ok(true)` if the value does exist, and `Ok(false)` otherwise.
    fn read(
        &self,
        key: &Type::Key,
        offset: usize,
        buf: &mut [u8],
    ) -> Result<bool, Self::Error>;

    /// Same as `read` but allocates a new buffer and returns it.
    ///
    /// Checks the size of the value and allocates a buffer of that size.
    fn read_alloc(&self, key: &Type::Key) -> Result<Option<Vec<u8>>, Self::Error>;
}

/// Base storage trait for Fuel infrastructure.
///
/// Allows writing the raw bytes of the value stored to a given key
/// from a provided buffer.
///
/// This trait should skip any serialization and simply copy the raw bytes
/// to the storage.
pub trait StorageWrite<Type: Mappable>: StorageMutate<Type> {
    /// Write the bytes to the given key from the provided buffer.
    ///
    /// Does not perform any serialization.
    fn write_bytes(&mut self, key: &Type::Key, buf: &[u8]) -> Result<(), Self::Error>;

    /// Write the bytes to the given key from the provided buffer and
    /// return the previous bytes if it existed.
    ///
    /// Does not perform any serialization.
    ///
    /// Returns the previous value if it existed.
    fn replace_bytes(
        &mut self,
        key: &Type::Key,
        buf: &[u8],
    ) -> Result<Option<Vec<u8>>, Self::Error>;

    /// Removes a bytes from the storage and returning it without deserializing it.
    fn take_bytes(&mut self, key: &Type::Key) -> Result<Option<Vec<u8>>, Self::Error>;
}

/// Returns the merkle root for the `StorageType` per merkle `Key`. Per one storage, it is
/// possible to have several merkle trees under different `Key`.
pub trait MerkleRootStorage<Key, StorageType>: StorageInspect<StorageType>
where
    StorageType: Mappable,
{
    /// Return the merkle root of the stored `Type` in the storage.
    ///
    /// The cryptographic primitive is an arbitrary choice of the implementor and this
    /// trait won't impose any restrictions to that.
    fn root(&self, key: &Key) -> Result<MerkleRoot, Self::Error>;
}

/// The wrapper around the storage that supports only methods from `StorageInspect`.
pub struct StorageRef<'a, T: 'a + ?Sized, Type: Mappable>(
    &'a T,
    core::marker::PhantomData<Type>,
);

/// Helper trait for `StorageInspect` to provide user-friendly API to retrieve storage as
/// reference.
///
/// # Example
///
/// ```rust
/// use fuel_storage::{Mappable, StorageInspect, StorageAsRef};
///
/// pub struct Contracts;
///
/// impl Mappable for Contracts {
///     type Key = Self::OwnedKey;
///     type OwnedKey = [u8; 32];
///     type Value = [u8];
///     type OwnedValue = Vec<u8>;
/// }
///
/// pub struct Balances;
///
/// impl Mappable for Balances {
///     type Key = Self::OwnedKey;
///     type OwnedKey = u128;
///     type Value = Self::OwnedValue;
///     type OwnedValue = u64;
/// }
///
/// pub trait Logic: StorageInspect<Contracts> + StorageInspect<Balances> {
///     fn run(&self) {
///         // You can specify which storage do you want to call with `storage::<Type>()`
///         let _ = self.storage::<Contracts>().get(&[0; 32]);
///         let _ = self.storage::<Balances>().get(&123);
///     }
/// }
/// ```
pub trait StorageAsRef {
    #[inline(always)]
    fn storage<Type>(&self) -> StorageRef<Self, Type>
    where
        Type: Mappable,
    {
        self.storage_as_ref()
    }

    #[inline(always)]
    fn storage_as_ref<Type>(&self) -> StorageRef<Self, Type>
    where
        Type: Mappable,
    {
        StorageRef(self, Default::default())
    }
}

impl<T> StorageAsRef for T {}

/// The wrapper around the storage that supports methods from `StorageInspect` and
/// `StorageMutate`.
pub struct StorageMut<'a, T: 'a + ?Sized, Type: Mappable>(
    &'a mut T,
    core::marker::PhantomData<Type>,
);

/// Helper trait for `StorageMutate` to provide user-friendly API to retrieve storage as
/// mutable reference.
///
/// # Example
///
/// ```rust
/// use fuel_storage::{Mappable, StorageMutate, StorageInspect, StorageAsMut};
///
/// pub struct Contracts;
///
/// impl Mappable for Contracts {
///     type Key = Self::OwnedKey;
///     type OwnedKey = [u8; 32];
///     type Value = [u8];
///     type OwnedValue = Vec<u8>;
/// }
///
/// pub struct Balances;
///
/// impl Mappable for Balances {
///     type Key = Self::OwnedKey;
///     type OwnedKey = u128;
///     type Value = Self::OwnedValue;
///     type OwnedValue = u64;
/// }
///
/// pub trait Logic: StorageInspect<Contracts> + StorageMutate<Balances> {
///     fn run(&mut self) {
///         let mut self_ = self;
///         // You can specify which storage do you want to call with `storage::<Type>()`
///         let _ = self_.storage::<Balances>().insert(&123, &321);
///         let _ = self_.storage::<Contracts>().get(&[0; 32]);
///     }
/// }
/// ```
pub trait StorageAsMut {
    #[inline(always)]
    fn storage<Type>(&mut self) -> StorageMut<Self, Type>
    where
        Type: Mappable,
    {
        self.storage_as_mut()
    }

    #[inline(always)]
    fn storage_as_mut<Type>(&mut self) -> StorageMut<Self, Type>
    where
        Type: Mappable,
    {
        StorageMut(self, Default::default())
    }
}

impl<T> StorageAsMut for T {}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {

    #[test]
    fn direction_next_from_map__next() {
        use super::*;

        let map: BTreeMap<u32, u32> = BTreeMap::from([(1, 10), (2, 20), (4, 40)]);
        let direction = Direction::Next;

        assert_eq!(
            direction.next_from_map(&0, &map),
            Some((Cow::Borrowed(&1), Cow::Borrowed(&10)))
        );
        assert_eq!(
            direction.next_from_map(&1, &map),
            Some((Cow::Borrowed(&2), Cow::Borrowed(&20)))
        );
        assert_eq!(
            direction.next_from_map(&2, &map),
            Some((Cow::Borrowed(&4), Cow::Borrowed(&40)))
        );
        assert_eq!(
            direction.next_from_map(&3, &map),
            Some((Cow::Borrowed(&4), Cow::Borrowed(&40)))
        );
        assert_eq!(direction.next_from_map(&4, &map), None);
    }

    #[test]
    fn direction_next_from_map__previous() {
        use super::*;

        let map: BTreeMap<u32, u32> = BTreeMap::from([(1, 10), (2, 20), (4, 40)]);
        let direction = Direction::Previous;

        assert_eq!(direction.next_from_map(&1, &map), None);
        assert_eq!(
            direction.next_from_map(&2, &map),
            Some((Cow::Borrowed(&1), Cow::Borrowed(&10)))
        );
        assert_eq!(
            direction.next_from_map(&3, &map),
            Some((Cow::Borrowed(&2), Cow::Borrowed(&20)))
        );
        assert_eq!(
            direction.next_from_map(&4, &map),
            Some((Cow::Borrowed(&2), Cow::Borrowed(&20)))
        );
        assert_eq!(
            direction.next_from_map(&5, &map),
            Some((Cow::Borrowed(&4), Cow::Borrowed(&40)))
        );
        assert_eq!(
            direction.next_from_map(&6, &map),
            Some((Cow::Borrowed(&4), Cow::Borrowed(&40)))
        );
    }
}
