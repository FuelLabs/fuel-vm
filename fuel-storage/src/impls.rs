use crate::{
    Mappable,
    MerkleRoot,
    MerkleRootStorage,
    StorageInspect,
    StorageMut,
    StorageMutate,
    StorageRead,
    StorageRef,
    StorageSize,
    StorageWrite,
};
use alloc::{
    borrow::Cow,
    vec::Vec,
};

impl<'a, T: StorageInspect<Type> + ?Sized, Type: Mappable> StorageInspect<Type>
    for &'a T
{
    type Error = T::Error;

    fn get(
        &self,
        key: &Type::Key,
    ) -> Result<Option<Cow<'_, Type::OwnedValue>>, Self::Error> {
        <T as StorageInspect<Type>>::get(self, key)
    }

    fn contains_key(&self, key: &Type::Key) -> Result<bool, Self::Error> {
        <T as StorageInspect<Type>>::contains_key(self, key)
    }
}

impl<'a, T: StorageInspect<Type> + ?Sized, Type: Mappable> StorageInspect<Type>
    for &'a mut T
{
    type Error = T::Error;

    fn get(
        &self,
        key: &Type::Key,
    ) -> Result<Option<Cow<'_, Type::OwnedValue>>, Self::Error> {
        <T as StorageInspect<Type>>::get(self, key)
    }

    fn contains_key(&self, key: &Type::Key) -> Result<bool, Self::Error> {
        <T as StorageInspect<Type>>::contains_key(self, key)
    }
}

impl<'a, T: StorageMutate<Type> + ?Sized, Type: Mappable> StorageMutate<Type>
    for &'a mut T
{
    fn insert(
        &mut self,
        key: &Type::Key,
        value: &Type::Value,
    ) -> Result<Option<Type::OwnedValue>, Self::Error> {
        <T as StorageMutate<Type>>::insert(self, key, value)
    }

    fn remove(
        &mut self,
        key: &Type::Key,
    ) -> Result<Option<Type::OwnedValue>, Self::Error> {
        <T as StorageMutate<Type>>::remove(self, key)
    }
}

impl<'a, T: StorageSize<Type> + ?Sized, Type: Mappable> StorageSize<Type> for &'a T {
    fn size_of_value(
        &self,
        key: &<Type as Mappable>::Key,
    ) -> Result<Option<usize>, Self::Error> {
        <T as StorageSize<Type>>::size_of_value(self, key)
    }
}

impl<'a, T: StorageSize<Type> + ?Sized, Type: Mappable> StorageSize<Type> for &'a mut T {
    fn size_of_value(
        &self,
        key: &<Type as Mappable>::Key,
    ) -> Result<Option<usize>, Self::Error> {
        <T as StorageSize<Type>>::size_of_value(self, key)
    }
}

impl<'a, T: StorageRead<Type> + StorageSize<Type> + ?Sized, Type: Mappable>
    StorageRead<Type> for &'a T
{
    fn read(
        &self,
        key: &<Type as Mappable>::Key,
        buf: &mut [u8],
    ) -> Result<Option<usize>, Self::Error> {
        <T as StorageRead<Type>>::read(self, key, buf)
    }

    fn read_alloc(
        &self,
        key: &<Type as Mappable>::Key,
    ) -> Result<Option<alloc::vec::Vec<u8>>, Self::Error> {
        <T as StorageRead<Type>>::read_alloc(self, key)
    }
}

impl<'a, T: StorageRead<Type> + StorageSize<Type> + ?Sized, Type: Mappable>
    StorageRead<Type> for &'a mut T
{
    fn read(
        &self,
        key: &<Type as Mappable>::Key,
        buf: &mut [u8],
    ) -> Result<Option<usize>, Self::Error> {
        <T as StorageRead<Type>>::read(self, key, buf)
    }

    fn read_alloc(
        &self,
        key: &<Type as Mappable>::Key,
    ) -> Result<Option<alloc::vec::Vec<u8>>, Self::Error> {
        <T as StorageRead<Type>>::read_alloc(self, key)
    }
}

impl<'a, T: StorageWrite<Type> + ?Sized, Type: Mappable> StorageWrite<Type>
    for &'a mut T
{
    fn write(&mut self, key: &Type::Key, buf: &[u8]) -> Result<usize, Self::Error> {
        <T as StorageWrite<Type>>::write(self, key, buf)
    }

    fn replace(
        &mut self,
        key: &Type::Key,
        buf: &[u8],
    ) -> Result<(usize, Option<Vec<u8>>), Self::Error> {
        <T as StorageWrite<Type>>::replace(self, key, buf)
    }

    fn take(&mut self, key: &Type::Key) -> Result<Option<Vec<u8>>, Self::Error> {
        <T as StorageWrite<Type>>::take(self, key)
    }
}

impl<'a, T: MerkleRootStorage<Key, Type> + ?Sized, Key, Type: Mappable>
    MerkleRootStorage<Key, Type> for &'a mut T
{
    fn root(&self, key: &Key) -> Result<MerkleRoot, Self::Error> {
        <T as MerkleRootStorage<Key, Type>>::root(self, key)
    }
}

impl<'a, T: StorageInspect<Type>, Type: Mappable> StorageRef<'a, T, Type> {
    #[inline(always)]
    pub fn get(
        self,
        key: &Type::Key,
    ) -> Result<Option<Cow<'a, Type::OwnedValue>>, T::Error> {
        self.0.get(key)
    }

    #[inline(always)]
    pub fn contains_key(self, key: &Type::Key) -> Result<bool, T::Error> {
        self.0.contains_key(key)
    }
}

impl<'a, T, Type: Mappable> StorageRef<'a, T, Type> {
    #[inline(always)]
    pub fn root<Key>(self, key: &Key) -> Result<MerkleRoot, T::Error>
    where
        T: MerkleRootStorage<Key, Type>,
    {
        self.0.root(key)
    }
}

impl<'a, T: StorageRead<Type>, Type: Mappable> StorageRef<'a, T, Type> {
    #[inline(always)]
    pub fn read(
        &self,
        key: &<Type as Mappable>::Key,
        buf: &mut [u8],
    ) -> Result<Option<usize>, T::Error> {
        self.0.read(key, buf)
    }

    #[inline(always)]
    pub fn read_alloc(
        &self,
        key: &<Type as Mappable>::Key,
    ) -> Result<Option<alloc::vec::Vec<u8>>, T::Error> {
        self.0.read_alloc(key)
    }
}

impl<'a, T: StorageInspect<Type>, Type: Mappable> StorageMut<'a, T, Type> {
    #[inline(always)]
    pub fn get(
        self,
        key: &Type::Key,
    ) -> Result<Option<Cow<'a, Type::OwnedValue>>, T::Error> {
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
        value: &Type::Value,
    ) -> Result<Option<Type::OwnedValue>, T::Error> {
        self.0.insert(key, value)
    }

    #[inline(always)]
    pub fn remove(self, key: &Type::Key) -> Result<Option<Type::OwnedValue>, T::Error> {
        self.0.remove(key)
    }
}

impl<'a, T, Type: Mappable> StorageMut<'a, T, Type> {
    #[inline(always)]
    pub fn root<Key>(self, key: &Key) -> Result<MerkleRoot, T::Error>
    where
        T: MerkleRootStorage<Key, Type>,
    {
        self.0.root(key)
    }
}

impl<'a, T: StorageWrite<Type>, Type: Mappable> StorageMut<'a, T, Type> {
    #[inline(always)]
    pub fn write(&mut self, key: &Type::Key, buf: &[u8]) -> Result<usize, T::Error> {
        self.0.write(key, buf)
    }

    #[inline(always)]
    pub fn replace(
        &mut self,
        key: &Type::Key,
        buf: &[u8],
    ) -> Result<(usize, Option<Vec<u8>>), T::Error>
    where
        T: StorageSize<Type>,
    {
        self.0.replace(key, buf)
    }

    #[inline(always)]
    pub fn take(&mut self, key: &Type::Key) -> Result<Option<Vec<u8>>, T::Error> {
        self.0.take(key)
    }
}
