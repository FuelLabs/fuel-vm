#![allow(missing_docs)] // TODO: add documentation

use std::{
    marker::PhantomData,
    mem::MaybeUninit,
};

use serde::{
    Deserialize,
    Serialize,
};

use crate::RawKey;

/// Convert data to reference-based format
pub trait Compressible {
    /// The compacted type with references
    type Compressed: Clone + Serialize + for<'a> Deserialize<'a>;
}

/// This type is compressable by the given compression context
pub trait CompressibleBy<Ctx>: Compressible
where
    Ctx: ?Sized,
{
    /// Perform compression, returning the compressed data,
    /// modifying the context
    fn compress(&self, ctx: &mut Ctx) -> anyhow::Result<Self::Compressed>;
}

/// This type is decompressable by the given decompression context
pub trait DecompressibleBy<Ctx>: Compressible
where
    Ctx: ?Sized,
    Self: Sized,
{
    /// Perform decompression, returning the original data
    fn decompress(c: &Self::Compressed, ctx: &Ctx) -> anyhow::Result<Self>;
}

pub trait CompressionContext<Type>
where
    Type: Compressible,
{
    fn compress(&mut self, value: &Type) -> anyhow::Result<Type::Compressed>;
}

pub trait DecompressionContext<Type>
where
    Type: Compressible,
{
    fn decompress(&self, value: &Type::Compressed) -> anyhow::Result<Type>;
}

pub trait RegistrySubstitutableBy<Ctx>: Compressible
where
    Ctx: ?Sized,
{
    fn substitute(&self, ctx: &mut Ctx, keyspace: &str) -> anyhow::Result<RawKey>;
}

pub trait RegistryDesubstitutableBy<Ctx>: Compressible
where
    Ctx: ?Sized,
    Self: Sized,
{
    fn desubstitute(c: &RawKey, ctx: &Ctx, keyspace: &str) -> anyhow::Result<Self>;
}

macro_rules! identity_compaction {
    ($t:ty) => {
        impl Compressible for $t {
            type Compressed = Self;
        }

        impl<Ctx> CompressibleBy<Ctx> for $t
        where
            Ctx: ?Sized,
        {
            fn compress(&self, _: &mut Ctx) -> anyhow::Result<Self> {
                Ok(*self)
            }
        }

        impl<Ctx> DecompressibleBy<Ctx> for $t
        where
            Ctx: ?Sized,
        {
            fn decompress(c: &Self::Compressed, _: &Ctx) -> anyhow::Result<Self> {
                Ok(*c)
            }
        }
    };
}

identity_compaction!(u8);
identity_compaction!(u16);
identity_compaction!(u32);
identity_compaction!(u64);
identity_compaction!(u128);

impl<T> Compressible for Option<T>
where
    T: Compressible + Clone,
{
    type Compressed = Option<T::Compressed>;
}

impl<T, Ctx> CompressibleBy<Ctx> for Option<T>
where
    T: CompressibleBy<Ctx> + Clone,
{
    fn compress(&self, ctx: &mut Ctx) -> anyhow::Result<Self::Compressed> {
        self.as_ref().map(|item| item.compress(ctx)).transpose()
    }
}

impl<T, Ctx> DecompressibleBy<Ctx> for Option<T>
where
    T: DecompressibleBy<Ctx> + Clone,
{
    fn decompress(c: &Self::Compressed, ctx: &Ctx) -> anyhow::Result<Self> {
        c.as_ref().map(|item| T::decompress(item, ctx)).transpose()
    }
}

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

impl<const S: usize, T> Compressible for [T; S]
where
    T: Compressible + Clone,
{
    type Compressed = ArrayWrapper<S, T::Compressed>;
}

impl<const S: usize, T, Ctx> CompressibleBy<Ctx> for [T; S]
where
    T: CompressibleBy<Ctx> + Clone,
{
    fn compress(&self, ctx: &mut Ctx) -> anyhow::Result<Self::Compressed> {
        Ok(ArrayWrapper(try_map_array(self.clone(), |v: T| {
            v.compress(ctx)
        })?))
    }
}

impl<const S: usize, T, Ctx> DecompressibleBy<Ctx> for [T; S]
where
    T: DecompressibleBy<Ctx> + Clone,
{
    fn decompress(c: &Self::Compressed, ctx: &Ctx) -> anyhow::Result<Self> {
        try_map_array(c.0.clone(), |v: T::Compressed| T::decompress(&v, ctx))
    }
}

impl<T> Compressible for Vec<T>
where
    T: Compressible + Clone,
{
    type Compressed = Vec<T::Compressed>;
}

impl<T, Ctx> CompressibleBy<Ctx> for Vec<T>
where
    T: CompressibleBy<Ctx> + Clone,
{
    fn compress(&self, ctx: &mut Ctx) -> anyhow::Result<Self::Compressed> {
        self.iter().map(|item| item.compress(ctx)).collect()
    }
}

impl<T, Ctx> DecompressibleBy<Ctx> for Vec<T>
where
    T: DecompressibleBy<Ctx> + Clone,
{
    fn decompress(c: &Self::Compressed, ctx: &Ctx) -> anyhow::Result<Self> {
        c.iter().map(|item| T::decompress(item, ctx)).collect()
    }
}

impl<T> Compressible for PhantomData<T> {
    type Compressed = ();
}

impl<T, Ctx> CompressibleBy<Ctx> for PhantomData<T> {
    fn compress(&self, _: &mut Ctx) -> anyhow::Result<Self::Compressed> {
        Ok(())
    }
}

impl<T, Ctx> DecompressibleBy<Ctx> for PhantomData<T> {
    fn decompress(_: &Self::Compressed, _: &Ctx) -> anyhow::Result<Self> {
        Ok(PhantomData)
    }
}

#[cfg(feature = "never")]
// #[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::RawKey;

    use super::{
        Compressible,
        CompressibleBy,
        CompressionContext,
        DecompressibleBy,
        DecompressionContext,
    };
    use fuel_derive::Compressed;
    use fuel_types::{
        Address,
        AssetId,
    };
    use serde::{
        Deserialize,
        Serialize,
    };

    pub struct TestCompressionContext {
        pub assets: HashMap<AssetId, RawKey>,
    }

    impl Compressible for AssetId {
        type Compressed = RawKey;
    }

    impl<Ctx> CompressibleBy<Ctx> for AssetId
    where
        Ctx: CompressionContext<Self>,
    {
        fn compress(&self, compressor: &mut Ctx) -> anyhow::Result<Self::Compressed> {
            compressor.compress(self)
        }
    }

    impl CompressionContext<AssetId> for TestCompressionContext {
        fn compress(
            &mut self,
            type_to_compact: &AssetId,
        ) -> anyhow::Result<<AssetId as Compressible>::Compressed> {
            let size = self.assets.len();
            let entry = self
                .assets
                .entry(*type_to_compact)
                .or_insert_with(|| RawKey::try_from(size as u32).unwrap());
            Ok(*entry)
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    struct ManualExample {
        a: AssetId,
        b: AssetId,
        c: u64,
        d: [u8; 32],
    }

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct ManualExampleCompressed {
        a: <AssetId as Compressible>::Compressed,
        b: <AssetId as Compressible>::Compressed,
        c: <u64 as Compressible>::Compressed,
        c: <[u8; 32] as Compressible>::Compressed,
    }

    impl Compressible for ManualExample {
        type Compressed = ManualExampleCompressed;
    }

    impl<Ctx> CompressibleBy<Ctx> for ManualExample {
        fn compress(&self, ctx: &mut Ctx) -> anyhow::Result<Self::Compressed>
        where
            AssetId: CompressibleBy<Ctx>,
            u64: CompressibleBy<Ctx>,
            [u8; 32]: CompressibleBy<Ctx>,
        {
            Ok(ManualExampleCompressed {
                a: self.a.compress(ctx)?,
                b: self.b.compress(ctx)?,
                c: self.c.compress(ctx)?,
                d: self.d.compress(ctx)?,
            })
        }
    }

    impl<Ctx> DecompressibleBy<Ctx> for ManualExample {
        fn decompress(c: &Self::Compressed, ctx: &Ctx) -> anyhow::Result<Self>
        where
            u64: DecompressibleBy<Ctx>,
        {
            Ok(ManualExample {
                a: <AssetId as DecompressibleBy<Ctx>>::decompress(c.a, ctx)?,
                b: <AssetId as DecompressibleBy<Ctx>>::decompress(c.b, ctx)?,
                c: <u64 as DecompressibleBy<Ctx>>::decompress(c.c, ctx)?,
                d: <[u8; 32] as DedompressibleBy<Ctx>>::decompress(c.d, ctx)?,
            })
        }
    }

    // #[derive(Debug, Clone, PartialEq, Compressed)]
    // struct AutomaticExample {
    //     #[da_compress(registry)]
    //     a: AssetId,
    //     #[da_compress(registry)]
    //     b: AssetId,
    //     c: u32,
    // }

    #[test]
    fn test_compaction_properties() {
        let _a = ManualExample {
            a: AssetId::from([1u8; 32]),
            b: AssetId::from([2u8; 32]),
            c: 3,
        };

        // let b = AutomaticExample {
        //     a: AssetId::from([1u8; 32]),
        //     b: AssetId::from([2u8; 32]),
        //     c: 3,
        // };
        // assert_eq!(b.count().Address, 0);
        // assert_eq!(b.count().AssetId, 2);
    }

    // #[test]
    // fn test_compaction_roundtrip_manual() {
    //     let target = ManualExample {
    //         a: Address::from([1u8; 32]),
    //         b: Address::from([2u8; 32]),
    //         c: 3,
    //     };
    //     let mut registry = DummyRegistry::default();
    //     let (compacted, _) = registry.compact(target.clone()).unwrap();
    //     let decompacted = ManualExample::decompact(compacted, &registry).unwrap();
    //     assert_eq!(target, decompacted);
    // }

    // #[test]
    // fn test_compaction_roundtrip_derive() {
    //     let target = AutomaticExample {
    //         a: AssetId::from([1u8; 32]),
    //         b: AssetId::from([2u8; 32]),
    //         c: 3,
    //     };
    //     let mut registry = DummyRegistry::default();
    //     let (compacted, _) = registry.compact(target.clone()).unwrap();
    //     let decompacted = AutomaticExample::decompact(compacted, &registry).unwrap();
    //     assert_eq!(target, decompacted);
    // }
}

#[cfg(test)]
mod playground {
    use fuel_types::AssetId;

    use crate::RawKey;

    use super::{
        ArrayWrapper,
        Compressible,
        CompressibleBy,
        CompressionContext,
        DecompressibleBy,
        DecompressionContext,
    };

    impl Compressible for AssetId {
        type Compressed = RawKey;
    }

    impl<C> CompressibleBy<C> for AssetId
    where
        C: CompressionContext<Self>,
    {
        fn compress(&self, compressor: &mut C) -> anyhow::Result<Self::Compressed> {
            compressor.compress(self)
        }
    }

    impl<C> DecompressibleBy<C> for AssetId
    where
        C: DecompressionContext<Self>,
    {
        fn decompress(c: &Self::Compressed, compressor: &C) -> anyhow::Result<Self> {
            compressor.decompress(c)
        }
    }

    // #[derive(CompressibleBy)]
    #[derive(Clone, Debug, PartialEq, Default)]
    pub struct ComplexStruct {
        asset_id: AssetId,
        array: [u8; 32],
        // #[compressible_by(skip)]
        some_field: u64,
    }

    // Generated code from `#[derive(CompressibleBy)]`

    #[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    pub struct CompressedComplexStruct {
        asset_id: <AssetId as Compressible>::Compressed,
        array: ArrayWrapper<32, u8>,
    }

    impl Compressible for ComplexStruct {
        type Compressed = CompressedComplexStruct;
    }

    impl<C> CompressibleBy<C> for ComplexStruct
    where
        AssetId: CompressibleBy<C>,
        [u8; 32]: DecompressibleBy<C>,
    {
        fn compress(&self, register: &mut C) -> anyhow::Result<Self::Compressed> {
            let asset_id = self.asset_id.compress(register)?;
            let array = self.array.compress(register)?;
            Ok(CompressedComplexStruct { asset_id, array })
        }
    }

    impl<C> DecompressibleBy<C> for ComplexStruct
    where
        AssetId: DecompressibleBy<C>,
        [u8; 32]: DecompressibleBy<C>,
    {
        fn decompress(c: &Self::Compressed, register: &C) -> anyhow::Result<Self> {
            let asset_id = AssetId::decompress(&c.asset_id, register)?;
            let array =
                <[u8; 32] as DecompressibleBy<C>>::decompress(&c.array, register)?;
            Ok(ComplexStruct {
                asset_id,
                array,
                ..Default::default()
            })
        }
    }

    mod tests {
        use bimap::BiMap;

        use super::*;

        #[derive(Default)]
        struct MapRegister {
            assets: BiMap<RawKey, AssetId>,
        }

        impl CompressionContext<AssetId> for MapRegister {
            fn compress(&mut self, type_to_compact: &AssetId) -> anyhow::Result<RawKey> {
                if let Some(key) = self.assets.get_by_right(type_to_compact) {
                    return Ok(*key);
                }
                let size = self.assets.len();
                let key = RawKey::try_from(size as u32).unwrap();
                self.assets.insert(key, *type_to_compact);
                Ok(key)
            }
        }

        impl DecompressionContext<AssetId> for MapRegister {
            fn decompress(&self, c: &RawKey) -> anyhow::Result<AssetId> {
                self.assets
                    .get_by_left(c)
                    .copied()
                    .ok_or_else(|| anyhow::anyhow!("Asset not found"))
            }
        }

        #[test]
        fn can_register() {
            let mut register = MapRegister::default();
            let complex_struct = ComplexStruct {
                asset_id: [1; 32].into(),
                array: [2; 32],
                some_field: Default::default(),
            };
            let compressed_complex = complex_struct.compress(&mut register).unwrap();
            let restored = <ComplexStruct as DecompressibleBy<_>>::decompress(
                &compressed_complex,
                &register,
            )
            .unwrap();
            assert_eq!(complex_struct, restored);

            let compressed_again_complex =
                complex_struct.compress(&mut register).unwrap();

            let new_complex_struct = ComplexStruct {
                asset_id: [2; 32].into(),
                array: [2; 32],
                some_field: 3,
            };
            let compressed_new_complex =
                new_complex_struct.compress(&mut register).unwrap();
        }
    }
}
