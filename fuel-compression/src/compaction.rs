use std::{
    marker::PhantomData,
    mem::MaybeUninit,
};

use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    registry::{
        access::{
            self,
            *,
        },
        add_keys,
        block_section::WriteTo,
        next_keys,
        ChangesPerTable,
        CountPerTable,
        KeyPerTable,
        RegistryDb,
        Table,
    },
    Key,
};

/// Context for compaction, i.e. converting data to reference-based format
#[must_use]
pub struct CompactionContext<'a, R> {
    /// The registry
    reg: &'a mut R,
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
impl<'a, R: RegistryDb> CompactionContext<'a, R> {
    /// Run the compaction for the given target, returning the compacted data.
    /// Changes are applied to the registry, and then returned as well.
    pub fn run<C: Compactable>(
        reg: &'a mut R,
        target: C,
    ) -> anyhow::Result<(C::Compact, ChangesPerTable)> {
        let start_keys = next_keys(reg)?;
        let next_keys = start_keys;
        let key_limits = target.count();
        let safe_keys_start = add_keys(next_keys, key_limits);

        let mut ctx = Self {
            reg,
            start_keys,
            next_keys,
            safe_keys_start,
            changes: ChangesPerTable::from_start_keys(start_keys),
        };

        let compacted = target.compact(&mut ctx)?;
        ctx.changes.apply_to_registry(ctx.reg)?;
        Ok((compacted, ctx.changes))
    }
}

impl<'a, R: RegistryDb> CompactionContext<'a, R> {
    /// Convert a value to a key
    /// If necessary, store the value in the changeset and allocate a new key.
    pub fn to_key<T: Table>(&mut self, value: T::Type) -> anyhow::Result<Key<T>>
    where
        KeyPerTable: access::AccessCopy<T, Key<T>> + access::AccessMut<T, Key<T>>,
        ChangesPerTable:
            access::AccessRef<T, WriteTo<T>> + access::AccessMut<T, WriteTo<T>>,
    {
        // Check if the value is within the current changeset
        if let Some(key) =
            <ChangesPerTable as AccessRef<T, WriteTo<T>>>::get(&self.changes)
                .lookup_value(&value)
        {
            return Ok(key);
        }

        // Check if the registry contains this value already
        if let Some(key) = self.reg.index_lookup::<T>(&value)? {
            let start: Key<T> = self.start_keys.value();
            let end: Key<T> = self.safe_keys_start.value();
            // Check if the value is in the possibly-overwritable range
            if !key.is_between(start, end) {
                return Ok(key);
            }
        }
        // Allocate a new key for this
        let key = <KeyPerTable as AccessMut<T, Key<T>>>::get_mut(&mut self.next_keys)
            .take_next();
        <ChangesPerTable as access::AccessMut<T, WriteTo<T>>>::get_mut(&mut self.changes)
            .values
            .push(value);
        Ok(key)
    }
}

/// Convert data to reference-based format
pub trait Compactable {
    /// The compacted type with references
    type Compact: Clone + Serialize + for<'a> Deserialize<'a>;

    /// Count max number of each key type, for upper limit of overwritten keys
    fn count(&self) -> CountPerTable;

    /// Convert to compacted format
    fn compact<R: RegistryDb>(
        &self,
        ctx: &mut CompactionContext<R>,
    ) -> anyhow::Result<Self::Compact>;

    /// Convert from compacted format
    fn decompact<R: RegistryDb>(compact: Self::Compact, reg: &R) -> anyhow::Result<Self>
    where
        Self: Sized;
}

macro_rules! identity_compaction {
    ($t:ty) => {
        impl Compactable for $t {
            type Compact = Self;

            fn count(&self) -> CountPerTable {
                CountPerTable::default()
            }

            fn compact<R: RegistryDb>(
                &self,
                _ctx: &mut CompactionContext<R>,
            ) -> anyhow::Result<Self::Compact> {
                Ok(*self)
            }

            fn decompact<R: RegistryDb>(
                compact: Self::Compact,
                _reg: &R,
            ) -> anyhow::Result<Self> {
                Ok(compact)
            }
        }
    };
}

identity_compaction!(u8);
identity_compaction!(u16);
identity_compaction!(u32);
identity_compaction!(u64);
identity_compaction!(u128);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArrayWrapper<const S: usize, T: Serialize + for<'a> Deserialize<'a>>(
    #[serde(with = "serde_big_array::BigArray")] pub [T; S],
);

// TODO: use try_map when stabilized: https://github.com/rust-lang/rust/issues/79711
#[allow(unsafe_code)]
fn try_map_array<const S: usize, T, R, E, F: FnMut(T) -> Result<R, E>>(
    arr: [T; S],
    mut f: F,
) -> Result<[R; S], E> {
    // SAFETY: we are claiming to have initialized an array of `MaybeUninit`s,
    // which do not require initialization.
    let mut tmp: [MaybeUninit<R>; S] = unsafe { MaybeUninit::uninit().assume_init() };

    // Dropping a `MaybeUninit` does nothing, so we can just overwrite the array.
    for (i, v) in arr.into_iter().enumerate() {
        tmp[i] = MaybeUninit::new(f(v)?);
    }

    // SAFETY: Every element is initialized.
    Ok(tmp.map(|v| unsafe { v.assume_init() }))
}

impl<const S: usize, T> Compactable for [T; S]
where
    T: Compactable + Clone + Serialize + for<'a> Deserialize<'a>,
{
    type Compact = ArrayWrapper<S, T::Compact>;

    fn count(&self) -> CountPerTable {
        let mut count = CountPerTable::default();
        for item in self.iter() {
            count += item.count();
        }
        count
    }

    fn compact<R: RegistryDb>(
        &self,
        ctx: &mut CompactionContext<R>,
    ) -> anyhow::Result<Self::Compact> {
        Ok(ArrayWrapper(try_map_array(self.clone(), |v: T| {
            v.compact(ctx)
        })?))
    }

    fn decompact<R: RegistryDb>(compact: Self::Compact, reg: &R) -> anyhow::Result<Self> {
        try_map_array(compact.0, |v: T::Compact| T::decompact(v, reg))
    }
}

impl<T> Compactable for Vec<T>
where
    T: Compactable + Clone + Serialize + for<'a> Deserialize<'a>,
{
    type Compact = Vec<T::Compact>;

    fn count(&self) -> CountPerTable {
        let mut count = CountPerTable::default();
        for item in self.iter() {
            count += item.count();
        }
        count
    }

    fn compact<R: RegistryDb>(
        &self,
        ctx: &mut CompactionContext<R>,
    ) -> anyhow::Result<Self::Compact> {
        self.iter().map(|item| item.compact(ctx)).collect()
    }

    fn decompact<R: RegistryDb>(compact: Self::Compact, reg: &R) -> anyhow::Result<Self> {
        compact
            .into_iter()
            .map(|item| T::decompact(item, reg))
            .collect()
    }
}

impl<T> Compactable for PhantomData<T> {
    type Compact = ();

    fn count(&self) -> CountPerTable {
        CountPerTable::default()
    }

    fn compact<R: RegistryDb>(
        &self,
        _ctx: &mut CompactionContext<R>,
    ) -> anyhow::Result<Self::Compact> {
        Ok(())
    }

    fn decompact<R: RegistryDb>(
        _compact: Self::Compact,
        _reg: &R,
    ) -> anyhow::Result<Self> {
        Ok(Self)
    }
}

#[cfg(test)]
mod tests {
    use fuel_compression::{
        tables,
        Compactable,
        CompactionContext,
        CountPerTable,
        Key,
        RegistryDb,
    };
    use fuel_derive::Compact;
    use fuel_types::{
        Address,
        AssetId,
    };
    use serde::{
        Deserialize,
        Serialize,
    };

    #[derive(Debug, Clone, PartialEq)]
    struct ManualExample {
        a: Address,
        b: Address,
        c: u64,
    }

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct ManualExampleCompact {
        a: Key<tables::Address>,
        b: Key<tables::Address>,
        c: u64,
    }

    impl Compactable for ManualExample {
        type Compact = ManualExampleCompact;

        fn count(&self) -> CountPerTable {
            CountPerTable::Address(2)
        }

        fn compact<R: RegistryDb>(
            &self,
            ctx: &mut CompactionContext<R>,
        ) -> anyhow::Result<Self::Compact> {
            let a = ctx.to_key::<tables::Address>(*self.a)?;
            let b = ctx.to_key::<tables::Address>(*self.b)?;
            Ok(ManualExampleCompact { a, b, c: self.c })
        }

        fn decompact<R: RegistryDb>(
            compact: Self::Compact,
            reg: &R,
        ) -> anyhow::Result<Self> {
            let a = Address::from(reg.read::<tables::Address>(compact.a)?);
            let b = Address::from(reg.read::<tables::Address>(compact.b)?);
            Ok(Self { a, b, c: compact.c })
        }
    }

    #[derive(Debug, Clone, PartialEq, Compact)]
    struct AutomaticExample {
        #[da_compress(registry = "AssetId")]
        a: AssetId,
        #[da_compress(registry = "AssetId")]
        b: AssetId,
        c: u32,
    }

    #[test]
    fn test_compaction_properties() {
        let a = ManualExample {
            a: Address::from([1u8; 32]),
            b: Address::from([2u8; 32]),
            c: 3,
        };
        assert_eq!(a.count().Address, 2);
        assert_eq!(a.count().AssetId, 0);

        let b = AutomaticExample {
            a: AssetId::from([1u8; 32]),
            b: AssetId::from([2u8; 32]),
            c: 3,
        };
        assert_eq!(b.count().Address, 0);
        assert_eq!(b.count().AssetId, 2);
    }

    #[test]
    fn test_compaction_roundtrip() {
        let target = ManualExample {
            a: Address::from([1u8; 32]),
            b: Address::from([2u8; 32]),
            c: 3,
        };
        let mut registry = fuel_compression::InMemoryRegistry::default();
        let (compacted, _) =
            CompactionContext::run(&mut registry, target.clone()).unwrap();
        let decompacted = ManualExample::decompact(compacted, &registry).unwrap();
        assert_eq!(target, decompacted);

        let target = AutomaticExample {
            a: AssetId::from([1u8; 32]),
            b: AssetId::from([2u8; 32]),
            c: 3,
        };
        let mut registry = fuel_compression::InMemoryRegistry::default();
        let (compacted, _) =
            fuel_compression::CompactionContext::run(&mut registry, target.clone())
                .unwrap();
        let decompacted = AutomaticExample::decompact(compacted, &registry).unwrap();
        assert_eq!(target, decompacted);
    }
}
