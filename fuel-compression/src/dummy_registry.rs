//! Temporal registry implementation used for tests

use std::collections::HashMap;

use crate::{
    key::RawKey,
    table::{
        access::*,
        KeyPerTable,
    },
    tables,
    Compactable,
    CompactionContext,
    DecompactionContext,
    Key,
    Table,
    TableName,
};

/// Temporal registry implementation used for tests.
#[derive(Default)]
pub struct DummyRegistry {
    next_keys: KeyPerTable,
    values: HashMap<(TableName, RawKey), Vec<u8>>,
}

impl DummyRegistry {
    /// Run the compaction for the given target, returning the compacted data.
    /// Applies the changes to the registry, but also returns them for block inclusion.
    pub fn compact<C: Compactable>(
        &mut self,
        target: C,
    ) -> anyhow::Result<(C::Compact, Changes)> {
        let key_limits = target.count();
        let safe_keys_start = self.next_keys.offset_by(key_limits);

        let mut ctx = DummyCompactionCtx {
            start_keys: self.next_keys,
            next_keys: self.next_keys,
            safe_keys_start,
            changes: Changes::default(),
            reg: self,
        };

        let compacted = target.compact(&mut ctx)?;
        let changes = ctx.changes;
        for change in changes.changes.iter() {
            self.values.insert((change.0, change.1), change.2.clone());
        }
        Ok((compacted, changes))
    }

    fn resolve_key<T: Table>(&self, key: Key<T>) -> anyhow::Result<T::Type> {
        self.values
            .get(&(<T as Table>::NAME, key.raw()))
            .ok_or_else(|| anyhow::anyhow!("Key not found: {:?}", key))
            .and_then(|bytes| postcard::from_bytes(bytes).map_err(|e| e.into()))
    }
}

/// Changeset for the registry
#[derive(Default)]
pub struct Changes {
    changes: Vec<(TableName, RawKey, Vec<u8>)>,
}
impl Changes {
    fn lookup_value<T: Table>(&self, value: &<T as Table>::Type) -> Option<Key<T>> {
        // Slow linear search. This is test-only code, so it's ok.
        for change in self.changes.iter() {
            if change.0 == <T as Table>::NAME
                && change.2 == postcard::to_stdvec(value).unwrap()
            {
                return Some(Key::<T>::from_raw(change.1));
            }
        }
        None
    }
}

/// Compaction session for
pub struct DummyCompactionCtx<'a> {
    /// The registry
    reg: &'a DummyRegistry,
    /// These are the keys where writing started
    start_keys: KeyPerTable,
    /// The next keys to use for each table
    next_keys: KeyPerTable,
    /// Keys in range next_keys..safe_keys_start
    /// could be overwritten by the compaction,
    /// and cannot be used for new values.
    safe_keys_start: KeyPerTable,
    changes: Changes,
}

impl<'a> DummyCompactionCtx<'a> {
    /// Convert a value to a key
    /// If necessary, store the value in the changeset and allocate a new key.
    fn value_to_key<T: Table>(&mut self, value: T::Type) -> anyhow::Result<Key<T>>
    where
        KeyPerTable: AccessCopy<T, Key<T>> + AccessMut<T, Key<T>>,
    {
        // Check if the value is within the current changeset
        if let Some(key) = self.changes.lookup_value(&value) {
            return Ok(key);
        }

        // Check if the registry contains this value already.
        // This is a slow linear search, but since this is test-only code, it's ok.
        let encoded = postcard::to_stdvec(&value).unwrap();
        for ((table_name, raw_key), bytes) in self.reg.values.iter() {
            if *table_name == <T as Table>::NAME && *bytes == encoded {
                let key = Key::<T>::from_raw(*raw_key);
                // Check if the value is in the possibly-overwritable range
                let start: Key<T> = self.start_keys.value();
                let end: Key<T> = self.safe_keys_start.value();
                if !key.is_between(start, end) {
                    return Ok(key);
                }
            }
        }

        // Allocate a new key for this
        let key = <KeyPerTable as AccessMut<T, Key<T>>>::get_mut(&mut self.next_keys)
            .take_next();
        self.changes.changes.push((
            <T as Table>::NAME,
            key.raw(),
            postcard::to_stdvec(&value).unwrap(),
        ));
        Ok(key)
    }
}

impl<'a> CompactionContext for DummyCompactionCtx<'a> {
    fn to_key_AssetId(
        &mut self,
        value: [u8; 32],
    ) -> anyhow::Result<Key<tables::AssetId>> {
        self.value_to_key(value)
    }

    fn to_key_Address(
        &mut self,
        value: [u8; 32],
    ) -> anyhow::Result<Key<tables::Address>> {
        self.value_to_key(value)
    }

    fn to_key_ContractId(
        &mut self,
        value: [u8; 32],
    ) -> anyhow::Result<Key<tables::ContractId>> {
        self.value_to_key(value)
    }

    fn to_key_ScriptCode(
        &mut self,
        value: Vec<u8>,
    ) -> anyhow::Result<Key<tables::ScriptCode>> {
        self.value_to_key(value)
    }

    fn to_key_Witness(&mut self, value: Vec<u8>) -> anyhow::Result<Key<tables::Witness>> {
        self.value_to_key(value)
    }
}

impl DecompactionContext for DummyRegistry {
    fn read_AssetId(
        &self,
        key: Key<tables::AssetId>,
    ) -> anyhow::Result<<tables::AssetId as Table>::Type> {
        self.resolve_key(key)
    }

    fn read_Address(
        &self,
        key: Key<tables::Address>,
    ) -> anyhow::Result<<tables::Address as Table>::Type> {
        self.resolve_key(key)
    }

    fn read_ContractId(
        &self,
        key: Key<tables::ContractId>,
    ) -> anyhow::Result<<tables::ContractId as Table>::Type> {
        self.resolve_key(key)
    }

    fn read_ScriptCode(
        &self,
        key: Key<tables::ScriptCode>,
    ) -> anyhow::Result<<tables::ScriptCode as Table>::Type> {
        self.resolve_key(key)
    }

    fn read_Witness(
        &self,
        key: Key<tables::Witness>,
    ) -> anyhow::Result<<tables::Witness as Table>::Type> {
        self.resolve_key(key)
    }
}
