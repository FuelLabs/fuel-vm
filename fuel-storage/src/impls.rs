use crate::{
    Mappable, MerkleRoot, MerkleRootStorage, StorageInspect, StorageMut, StorageMutate, StorageRef,
};
use alloc::borrow::Cow;

impl<'a, T: StorageInspect<Type> + ?Sized, Type: Mappable> StorageInspect<Type> for &'a T {
    type Error = T::Error;

    fn get(&self, key: &Type::Key) -> Result<Option<Cow<'_, Type::GetValue>>, Self::Error> {
        <T as StorageInspect<Type>>::get(self, key)
    }

    fn contains_key(&self, key: &Type::Key) -> Result<bool, Self::Error> {
        <T as StorageInspect<Type>>::contains_key(self, key)
    }
}

impl<'a, T: StorageInspect<Type> + ?Sized, Type: Mappable> StorageInspect<Type> for &'a mut T {
    type Error = T::Error;

    fn get(&self, key: &Type::Key) -> Result<Option<Cow<'_, Type::GetValue>>, Self::Error> {
        <T as StorageInspect<Type>>::get(self, key)
    }

    fn contains_key(&self, key: &Type::Key) -> Result<bool, Self::Error> {
        <T as StorageInspect<Type>>::contains_key(self, key)
    }
}

impl<'a, T: StorageMutate<Type> + ?Sized, Type: Mappable> StorageMutate<Type> for &'a mut T {
    fn insert(
        &mut self,
        key: &Type::Key,
        value: &Type::SetValue,
    ) -> Result<Option<Type::GetValue>, Self::Error> {
        <T as StorageMutate<Type>>::insert(self, key, value)
    }

    fn remove(&mut self, key: &Type::Key) -> Result<Option<Type::GetValue>, Self::Error> {
        <T as StorageMutate<Type>>::remove(self, key)
    }
}

impl<'a, T: MerkleRootStorage<Key, Type> + ?Sized, Key, Type: Mappable> MerkleRootStorage<Key, Type>
    for &'a mut T
{
    fn root(&mut self, key: &Key) -> Result<MerkleRoot, Self::Error> {
        <T as MerkleRootStorage<Key, Type>>::root(self, key)
    }
}

impl<'a, T: StorageInspect<Type>, Type: Mappable> StorageRef<'a, T, Type> {
    #[inline(always)]
    pub fn get(self, key: &Type::Key) -> Result<Option<Cow<'a, Type::GetValue>>, T::Error> {
        self.0.get(key)
    }

    #[inline(always)]
    pub fn contains_key(self, key: &Type::Key) -> Result<bool, T::Error> {
        self.0.contains_key(key)
    }
}

impl<'a, T: StorageInspect<Type>, Type: Mappable> StorageMut<'a, T, Type> {
    #[inline(always)]
    pub fn get(self, key: &Type::Key) -> Result<Option<Cow<'a, Type::GetValue>>, T::Error> {
        // Workaround, because compiler doesn't convert the lifetime to `'a` by default.
        let self_: &'a T = self.0;
        self_.get(key)
    }

    #[inline(always)]
    pub fn contains_key(self, key: &Type::Key) -> Result<bool, T::Error> {
        self.0.contains_key(key)
    }
}

impl<'a, T: StorageMutate<Type>, Type: Mappable> StorageMut<'a, T, Type> {
    #[inline(always)]
    pub fn insert(
        self,
        key: &Type::Key,
        value: &Type::SetValue,
    ) -> Result<Option<Type::GetValue>, T::Error> {
        self.0.insert(key, value)
    }

    #[inline(always)]
    pub fn remove(self, key: &Type::Key) -> Result<Option<Type::GetValue>, T::Error> {
        self.0.remove(key)
    }
}

impl<'a, T: StorageMutate<Type>, Type: Mappable> StorageMut<'a, T, Type> {
    #[inline(always)]
    pub fn root<Key>(self, key: &Key) -> Result<MerkleRoot, T::Error>
    where
        T: MerkleRootStorage<Key, Type>,
    {
        self.0.root(key)
    }
}
