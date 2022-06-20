use fuel_storage::Storage;

use alloc::borrow::Cow;
use hashbrown::HashMap;

#[derive(Debug)]
pub struct StorageMap<Key, Value> {
    map: HashMap<Key, Value>,
}

impl<Key, Value> Default for StorageMap<Key, Value> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Key, Value> StorageMap<Key, Value> {
    pub fn new() -> Self {
        Self {
            map: HashMap::<Key, Value>::new(),
        }
    }
}

impl<Key, Value> Storage<Key, Value> for StorageMap<Key, Value>
where
    Key: Eq + core::hash::Hash + Clone,
    Value: Clone,
{
    type Error = core::convert::Infallible;

    fn insert(&mut self, key: &Key, value: &Value) -> Result<Option<Value>, Self::Error> {
        self.map.insert(key.clone(), value.clone());
        let v = Some(value.clone());
        Ok(v)
    }

    fn remove(&mut self, key: &Key) -> Result<Option<Value>, Self::Error> {
        let value = self.map.remove(key);
        Ok(value)
    }

    fn get(&self, key: &Key) -> Result<Option<Cow<Value>>, Self::Error> {
        let result = self.map.get(key);
        let value = result.map(|value| Cow::Borrowed(value));
        Ok(value)
    }

    fn contains_key(&self, key: &Key) -> Result<bool, Self::Error> {
        let contains = self.map.contains_key(key);
        Ok(contains)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    struct TestKey(u32);

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    struct TestValue(u32);

    #[test]
    fn test_get_returns_value_for_given_key() {
        let key = TestKey(0);
        let mut store = StorageMap::<TestKey, TestValue>::new();
        let _ = store.insert(&key, &TestValue(0));

        assert_eq!(store.get(&key).unwrap(), Some(Cow::Borrowed(&TestValue(0))));
    }
    #[test]
    fn test_get_returns_none_for_invalid_key() {
        let key = TestKey(0);
        let invalid_key = TestKey(1);
        let mut store = StorageMap::<TestKey, TestValue>::new();
        let _ = store.insert(&key, &TestValue(0));

        assert_eq!(store.get(&invalid_key).unwrap(), None);
    }

    #[test]
    fn test_insert_existing_key_updates_value_for_given_key() {
        let key = TestKey(0);
        let mut store = StorageMap::<TestKey, TestValue>::new();
        let _ = store.insert(&key, &TestValue(0));
        let _ = store.insert(&key, &TestValue(1));

        assert_eq!(store.get(&key).unwrap(), Some(Cow::Borrowed(&TestValue(1))));
    }

    #[test]
    fn test_remove_deletes_the_value_for_given_key() {
        let key = TestKey(0);
        let mut store = StorageMap::<TestKey, TestValue>::new();
        let _ = store.insert(&key, &TestValue(0));
        let _ = store.remove(&key);

        assert_eq!(store.get(&key).unwrap(), None);
    }

    #[test]
    fn test_remove_returns_the_deleted_value_for_given_key() {
        let key = TestKey(0);
        let mut store = StorageMap::<TestKey, TestValue>::new();
        let _ = store.insert(&key, &TestValue(0));

        assert_eq!(store.remove(&key).unwrap(), Some(TestValue(0)));
    }

    #[test]
    fn test_remove_returns_none_for_invalid_key() {
        let invalid_key = TestKey(0);
        let mut store = StorageMap::<TestKey, TestValue>::new();

        assert_eq!(store.remove(&invalid_key).unwrap(), None);
    }

    #[test]
    fn test_contains_key_returns_true_for_valid_key() {
        let key = TestKey(0);
        let mut store = StorageMap::<TestKey, TestValue>::new();
        let _ = store.insert(&key, &TestValue(0));

        assert_eq!(store.contains_key(&key).unwrap(), true);
    }

    #[test]
    fn test_contains_key_returns_false_for_invalid_key() {
        let invalid_key = TestKey(0);
        let store = StorageMap::<TestKey, TestValue>::new();

        assert_eq!(store.contains_key(&invalid_key).unwrap(), false);
    }
}
