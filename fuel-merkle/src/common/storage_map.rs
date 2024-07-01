use crate::{
    alloc::borrow::ToOwned,
    storage::{
        Mappable,
        StorageInspect,
        StorageMutate,
    },
};

use alloc::borrow::Cow;
use hashbrown::HashMap;

#[derive(Debug, Clone)]
pub struct StorageMap<Type>
where
    Type: Mappable,
{
    map: HashMap<Type::OwnedKey, Type::OwnedValue>,
}

impl<Type> Default for StorageMap<Type>
where
    Type: Mappable,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Type> StorageMap<Type>
where
    Type: Mappable,
{
    pub fn new() -> Self {
        Self {
            map: Default::default(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }
}

impl<Type> StorageInspect<Type> for StorageMap<Type>
where
    Type: Mappable,
    Type::Key: Eq + core::hash::Hash,
    Type::OwnedKey: Eq + core::hash::Hash + core::borrow::Borrow<Type::Key>,
{
    type Error = core::convert::Infallible;

    fn get(&self, key: &Type::Key) -> Result<Option<Cow<Type::OwnedValue>>, Self::Error> {
        let result = self.map.get(key);
        let value = result.map(Cow::Borrowed);
        Ok(value)
    }

    fn contains_key(&self, key: &Type::Key) -> Result<bool, Self::Error> {
        let contains = self.map.contains_key(key);
        Ok(contains)
    }
}

impl<Type> StorageMutate<Type> for StorageMap<Type>
where
    Type: Mappable,
    Type::Key: Eq + core::hash::Hash,
    Type::OwnedKey: Eq + core::hash::Hash + core::borrow::Borrow<Type::Key>,
{
    fn replace(
        &mut self,
        key: &Type::Key,
        value: &Type::Value,
    ) -> Result<Option<Type::OwnedValue>, Self::Error> {
        let previous = self
            .map
            .insert(key.to_owned().into(), value.to_owned().into());
        Ok(previous)
    }

    fn take(&mut self, key: &Type::Key) -> Result<Option<Type::OwnedValue>, Self::Error> {
        let value = self.map.remove(key);
        Ok(value)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    struct TestKey(u32);

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    struct TestValue(u32);

    struct TestTable;

    impl Mappable for TestTable {
        type Key = Self::OwnedKey;
        type OwnedKey = TestKey;
        type OwnedValue = TestValue;
        type Value = Self::OwnedValue;
    }

    #[test]
    fn test_get_returns_value_for_given_key() {
        let key = TestKey(0);
        let mut store = StorageMap::<TestTable>::new();
        let _ = store.insert(&key, &TestValue(0));

        assert_eq!(store.get(&key).unwrap(), Some(Cow::Borrowed(&TestValue(0))));
    }
    #[test]
    fn test_get_returns_none_for_invalid_key() {
        let key = TestKey(0);
        let invalid_key = TestKey(1);
        let mut store = StorageMap::<TestTable>::new();
        let _ = store.insert(&key, &TestValue(0));

        assert_eq!(store.get(&invalid_key).unwrap(), None);
    }

    #[test]
    fn test_insert_existing_key_updates_value_for_given_key() {
        let key = TestKey(0);
        let mut store = StorageMap::<TestTable>::new();
        let _ = store.insert(&key, &TestValue(0));
        let _ = store.insert(&key, &TestValue(1));

        assert_eq!(store.get(&key).unwrap(), Some(Cow::Borrowed(&TestValue(1))));
    }

    #[test]
    fn test_remove_deletes_the_value_for_given_key() {
        let key = TestKey(0);
        let mut store = StorageMap::<TestTable>::new();
        let _ = store.insert(&key, &TestValue(0));
        let _ = store.remove(&key);

        assert_eq!(store.get(&key).unwrap(), None);
    }

    #[test]
    fn test_remove_returns_the_deleted_value_for_given_key() {
        let key = TestKey(0);
        let mut store = StorageMap::<TestTable>::new();
        let _ = store.insert(&key, &TestValue(0));

        assert_eq!(store.take(&key).unwrap(), Some(TestValue(0)));
    }

    #[test]
    fn test_remove_returns_none_for_invalid_key() {
        let invalid_key = TestKey(0);
        let mut store = StorageMap::<TestTable>::new();

        assert_eq!(store.take(&invalid_key).unwrap(), None);
    }

    #[test]
    fn test_contains_key_returns_true_for_valid_key() {
        let key = TestKey(0);
        let mut store = StorageMap::<TestTable>::new();
        let _ = store.insert(&key, &TestValue(0));

        assert_eq!(store.contains_key(&key).unwrap(), true);
    }

    #[test]
    fn test_contains_key_returns_false_for_invalid_key() {
        let invalid_key = TestKey(0);
        let store = StorageMap::<TestTable>::new();

        assert_eq!(store.contains_key(&invalid_key).unwrap(), false);
    }
}
