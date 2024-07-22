use std::{
    marker::PhantomData,
    mem::MaybeUninit,
};

use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    CompactionContext,
    CountPerTable,
    DecompactionContext,
    Key,
};

/// Convert data to reference-based format
pub trait Compactable {
    /// The compacted type with references
    type Compact: Clone + Serialize + for<'a> Deserialize<'a>;

    /// Count max number of each key type, for upper limit of overwritten keys
    fn count(&self) -> CountPerTable;

    /// Convert to compacted format
    fn compact(&self, ctx: &mut dyn CompactionContext) -> anyhow::Result<Self::Compact>;

    /// Convert from compacted format
    fn decompact(
        compact: Self::Compact,
        ctx: &dyn DecompactionContext,
    ) -> anyhow::Result<Self>
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

            fn compact(
                &self,
                _ctx: &mut dyn CompactionContext,
            ) -> anyhow::Result<Self::Compact> {
                Ok(*self)
            }

            fn decompact(
                compact: Self::Compact,
                _ctx: &dyn DecompactionContext,
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

    fn compact(&self, ctx: &mut dyn CompactionContext) -> anyhow::Result<Self::Compact> {
        Ok(ArrayWrapper(try_map_array(self.clone(), |v: T| {
            v.compact(ctx)
        })?))
    }

    fn decompact(
        compact: Self::Compact,
        ctx: &dyn DecompactionContext,
    ) -> anyhow::Result<Self> {
        try_map_array(compact.0, |v: T::Compact| T::decompact(v, ctx))
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

    fn compact(&self, ctx: &mut dyn CompactionContext) -> anyhow::Result<Self::Compact> {
        self.iter().map(|item| item.compact(ctx)).collect()
    }

    fn decompact(
        compact: Self::Compact,
        ctx: &dyn DecompactionContext,
    ) -> anyhow::Result<Self> {
        compact
            .into_iter()
            .map(|item| T::decompact(item, ctx))
            .collect()
    }
}

impl<T> Compactable for PhantomData<T> {
    type Compact = ();

    fn count(&self) -> CountPerTable {
        CountPerTable::default()
    }

    fn compact(&self, _ctx: &mut dyn CompactionContext) -> anyhow::Result<Self::Compact> {
        Ok(())
    }

    fn decompact(
        _compact: Self::Compact,
        _ctx: &dyn DecompactionContext,
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

    use crate::DecompactionContext;

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

        fn compact(
            &self,
            ctx: &mut dyn CompactionContext,
        ) -> anyhow::Result<Self::Compact> {
            let a = ctx.to_key_Address(*self.a)?;
            let b = ctx.to_key_Address(*self.b)?;
            Ok(ManualExampleCompact { a, b, c: self.c })
        }

        fn decompact(
            compact: Self::Compact,
            ctx: &dyn DecompactionContext,
        ) -> anyhow::Result<Self> {
            let a = Address::from(ctx.read_Address(compact.a)?);
            let b = Address::from(ctx.read_Address(compact.b)?);
            Ok(Self { a, b, c: compact.c })
        }
    }

    #[derive(Debug, Clone, PartialEq, Compact)]
    struct AutomaticExample {
        #[da_compress(registry = ::fuel_compression::tables::AssetId)]
        a: AssetId,
        #[da_compress(registry = ::fuel_compression::tables::AssetId)]
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
