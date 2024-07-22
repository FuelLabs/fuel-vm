//! Temporal registry implementation used for tests

use std::collections::HashMap;

use crate::{
    block_section::WriteTo,
    key::RawKey,
    table::{
        access::*,
        add_keys,
        KeyPerTable,
    },
    tables,
    ChangesPerTable,
    Compactable,
    CompactionContext,
    DecompactionContext,
    Key,
    Table,
};

type TableName = &'static str;

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
    ) -> anyhow::Result<(C::Compact, ChangesPerTable)> {
        let key_limits = target.count();
        let safe_keys_start = add_keys(self.next_keys, key_limits);

        let mut ctx = DummyCompactionCtx {
            start_keys: self.next_keys,
            next_keys: self.next_keys,
            safe_keys_start,
            changes: ChangesPerTable::from_start_keys(self.next_keys),
            reg: self,
        };

        let compacted = target.compact(&mut ctx)?;
        let changes = ctx.finalize();
        self.apply_changes(&changes);
        Ok((compacted, changes))
    }

    fn apply_changes(&mut self, changes: &ChangesPerTable) {
        macro_rules! for_tables {
            ($($name:ident),*$(,)?) => { paste::paste! {{
                #[allow(non_snake_case)]
                let ChangesPerTable {
                    $($name: [<$name _changes>],)*
                } = changes;

                $(
                    let mut key = [< $name _changes >].start_key.raw();
                    for value in [< $name _changes >].values.iter() {
                        self.values.insert((tables::$name::NAME, key), postcard::to_stdvec(&value).unwrap());
                        key = key.next();
                    }
                )*
            } }};
        }

        for_tables!(AssetId, Address, ContractId, ScriptCode, Witness)
    }

    fn resolve_key<T: Table>(&self, key: Key<T>) -> anyhow::Result<T::Type> {
        self.values
            .get(&(<T as Table>::NAME, key.raw()))
            .ok_or_else(|| anyhow::anyhow!("Key not found: {:?}", key))
            .and_then(|bytes| postcard::from_bytes(bytes).map_err(|e| e.into()))
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
    changes: ChangesPerTable,
}

impl<'a> DummyCompactionCtx<'a> {
    /// Convert a value to a key
    /// If necessary, store the value in the changeset and allocate a new key.
    fn value_to_key<T: Table>(&mut self, value: T::Type) -> anyhow::Result<Key<T>>
    where
        KeyPerTable: AccessCopy<T, Key<T>> + AccessMut<T, Key<T>>,
        ChangesPerTable: AccessRef<T, WriteTo<T>> + AccessMut<T, WriteTo<T>>,
    {
        // Check if the value is within the current changeset
        if let Some(key) =
            <ChangesPerTable as AccessRef<T, WriteTo<T>>>::get(&self.changes)
                .lookup_value(&value)
        {
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
        <ChangesPerTable as AccessMut<T, WriteTo<T>>>::get_mut(&mut self.changes)
            .values
            .push(value);
        Ok(key)
    }
}

impl<'a> CompactionContext for DummyCompactionCtx<'a> {
    fn finalize(self) -> ChangesPerTable {
        self.changes
    }

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
