#![no_std]

mod impls;

extern crate alloc;

use alloc::borrow::Cow;

/// Merkle root alias type
pub type MerkleRoot = [u8; 32];

/// Mappable type with `Key` and `Value`.
pub trait Mappable {
    /// The type of the value's key.
    type Key;
    /// The value type is used while setting the value to the storage. In most cases, it is the same
    /// as `Self::GetValue`, but it is without restriction and can be used for performance
    /// optimizations.
    ///
    /// # Example
    ///
    /// ```rust
    /// use core::marker::PhantomData;
    /// use fuel_storage::Mappable;
    /// pub struct Contract<'a>(PhantomData<&'a ()>);
    ///
    /// impl<'a> Mappable for Contract<'a> {
    ///     type Key = &'a [u8; 32];
    ///     /// It is optimized to use slice instead of vector.
    ///     type SetValue = [u8];
    ///     type GetValue = Vec<u8>;
    /// }
    /// ```
    type SetValue: ?Sized;
    /// The value type is used while getting the value from the storage.
    type GetValue: Clone;
}

/// Base read storage trait for Fuel infrastructure.
///
/// Generic should implement [`Mappable`] trait with all storage type information.
pub trait StorageInspect<Type: Mappable> {
    type Error;

    /// Retrieve `Cow<Value>` such as `Key->Value`.
    fn get(&self, key: &Type::Key) -> Result<Option<Cow<Type::GetValue>>, Self::Error>;

    /// Return `true` if there is a `Key` mapping to a value in the storage.
    fn contains_key(&self, key: &Type::Key) -> Result<bool, Self::Error>;
}

/// Base storage trait for Fuel infrastructure.
///
/// Generic should implement [`Mappable`] trait with all storage type information.
pub trait StorageMutate<Type: Mappable>: StorageInspect<Type> {
    /// Append `Key->Value` mapping to the storage.
    ///
    /// If `Key` was already mappped to a value, return the replaced value as `Ok(Some(Value))`. Return
    /// `Ok(None)` otherwise.
    fn insert(&mut self, key: &Type::Key, value: &Type::SetValue) -> Result<Option<Type::GetValue>, Self::Error>;

    /// Remove `Key->Value` mapping from the storage.
    ///
    /// Return `Ok(Some(Value))` if the value was present. If the key wasn't found, return
    /// `Ok(None)`.
    fn remove(&mut self, key: &Type::Key) -> Result<Option<Type::GetValue>, Self::Error>;
}

/// Returns the merkle root for the `StorageType` per merkle `Key`. The type should implement the
/// `StorageMutate` for the `StorageType`. Per one storage, it is possible to have several merkle trees
/// under different `Key`.
pub trait MerkleRootStorage<Key, StorageType>: StorageMutate<StorageType>
where
    StorageType: Mappable,
{
    /// Return the merkle root of the stored `Type` in the storage.
    ///
    /// The cryptographic primitive is an arbitrary choice of the implementor and this trait won't
    /// impose any restrictions to that.
    fn root(&mut self, key: &Key) -> Result<MerkleRoot, Self::Error>;
}

/// The wrapper around the storage that supports only methods from `StorageInspect`.
pub struct StorageRef<'a, T: 'a + ?Sized, Type: Mappable>(&'a T, core::marker::PhantomData<Type>);

/// Helper trait for `StorageInspect` to provide user-friendly API to retrieve storage as reference.
///
/// # Example
///
/// ```rust
/// use fuel_storage::{Mappable, StorageInspect, StorageAsRef};
///
/// pub struct Contracts;
///
/// impl Mappable for Contracts {
///     type Key = [u8; 32];
///     type SetValue = [u8];
///     type GetValue = Vec<u8>;
/// }
///
/// pub struct Balances;
///
/// impl Mappable for Balances {
///     type Key = u128;
///     type SetValue = u64;
///     type GetValue = u64;
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
        StorageRef(self, Default::default())
    }
}

impl<'a, T> StorageAsRef for T {}

/// The wrapper around the storage that supports methods from `StorageInspect` and `StorageMutate`.
pub struct StorageMut<'a, T: 'a + ?Sized, Type: Mappable>(&'a mut T, core::marker::PhantomData<Type>);

/// Helper trait for `StorageMutate` to provide user-friendly API to retrieve storage as mutable
/// reference.
///
/// # Example
///
/// ```rust
/// use fuel_storage::{Mappable, StorageMutate, StorageInspect, StorageAsMut};
///
/// pub struct Contracts;
///
/// impl Mappable for Contracts {
///     type Key = [u8; 32];
///     type SetValue = [u8];
///     type GetValue = Vec<u8>;
/// }
///
/// pub struct Balances;
///
/// impl Mappable for Balances {
///     type Key = u128;
///     type SetValue = u64;
///     type GetValue = u64;
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
        StorageMut(self, Default::default())
    }
}

impl<'a, T> StorageAsMut for T {}
