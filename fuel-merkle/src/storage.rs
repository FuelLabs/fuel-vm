use alloc::borrow::Cow;
use core::convert::Infallible;

// Re-export fuel-storage traits
pub use fuel_storage::{
    Mappable,
    StorageInspect,
    StorageMutate,
};

pub trait StorageInspectInfallible<Type: Mappable> {
    fn get(&self, key: &Type::Key) -> Option<Cow<'_, Type::OwnedValue>>;
    fn contains_key(&self, key: &Type::Key) -> bool;
}

pub trait StorageMutateInfallible<Type: Mappable> {
    fn insert(&mut self, key: &Type::Key, value: &Type::Value);
    fn remove(&mut self, key: &Type::Key);
}

impl<S, Type> StorageInspectInfallible<Type> for S
where
    S: StorageInspect<Type, Error = Infallible>,
    Type: Mappable,
{
    fn get(&self, key: &Type::Key) -> Option<Cow<'_, Type::OwnedValue>> {
        <Self as StorageInspect<Type>>::get(self, key)
            .expect("Expected get() to be infallible")
    }

    fn contains_key(&self, key: &Type::Key) -> bool {
        <Self as StorageInspect<Type>>::contains_key(self, key)
            .expect("Expected contains_key() to be infallible")
    }
}

impl<S, Type> StorageMutateInfallible<Type> for S
where
    S: StorageMutate<Type, Error = Infallible>,
    Type: Mappable,
{
    fn insert(&mut self, key: &Type::Key, value: &Type::Value) {
        <Self as StorageMutate<Type>>::insert(self, key, value)
            .expect("Expected insert() to be infallible")
    }

    fn remove(&mut self, key: &Type::Key) {
        <Self as StorageMutate<Type>>::remove(self, key)
            .expect("Expected remove() to be infallible")
    }
}
