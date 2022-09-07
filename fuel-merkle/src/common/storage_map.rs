use fuel_storage::{Mappable, StorageInspect, StorageMutate};

use alloc::borrow::Cow;
use hashbrown::HashMap;

#[derive(Debug)]
pub struct StorageMap<Type: Mappable> {
    map: HashMap<Type::Key, Type::GetValue>,
}

impl<Type: Mappable> Default for StorageMap<Type> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Type: Mappable> StorageMap<Type> {
    pub fn new() -> Self {
        Self {
            map: Default::default(),
        }
    }
}

impl<Type> StorageInspect<Type> for StorageMap<Type>
where
    Type: Mappable,
    Type::Key: Eq + core::hash::Hash + Clone,
    Type::GetValue: Clone,
{
    type Error = core::convert::Infallible;

    fn get(&self, key: &Type::Key) -> Result<Option<Cow<Type::GetValue>>, Self::Error> {
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
    Type::Key: Eq + core::hash::Hash + Clone,
    Type::SetValue: Clone,
    Type::GetValue: Clone + From<Type::SetValue>,
{
    fn insert(
        &mut self,
        key: &Type::Key,
        value: &Type::SetValue,
    ) -> Result<Option<Type::GetValue>, Self::Error> {
        let previous = self.map.remove(key);

        self.map.insert(key.clone(), value.clone().into());
        Ok(previous)
    }

    fn remove(&mut self, key: &Type::Key) -> Result<Option<Type::GetValue>, Self::Error> {
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
        type Key = TestKey;
        type SetValue = TestValue;
        type GetValue = Self::SetValue;
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

        assert_eq!(store.remove(&key).unwrap(), Some(TestValue(0)));
    }

    #[test]
    fn test_remove_returns_none_for_invalid_key() {
        let invalid_key = TestKey(0);
        let mut store = StorageMap::<TestTable>::new();

        assert_eq!(store.remove(&invalid_key).unwrap(), None);
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
