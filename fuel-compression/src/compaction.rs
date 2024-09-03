use std::{
    marker::PhantomData,
    mem::MaybeUninit,
};

use serde::{
    Deserialize,
    Serialize,
};

use crate::RawKey;

/// This type can be compressed to a more compact form and back using
/// `CompressibleBy` and `DecompressibleBy` traits.
pub trait Compressible {
    /// The compressed type.
    type Compressed: Clone + Serialize + for<'a> Deserialize<'a>;
}

/// This type can be compressed to a more compact form and back using
/// `CompressionContext`.
pub trait CompressibleBy<Ctx>: Compressible
where
    Ctx: ?Sized,
{
    /// Perform compression, returning the compressed data and possibly modifying the
    /// context. The context is mutable to allow for stateful compression.
    /// For instance, it can be used to extract original data when replacing it with
    /// references.
    fn compress(&self, ctx: &mut Ctx) -> anyhow::Result<Self::Compressed>;
}

/// This type can be decompressed using `CompressionContext`.
pub trait DecompressibleBy<Ctx>: Compressible
where
    Ctx: ?Sized,
    Self: Sized,
{
    /// Perform decompression, returning the original data.
    /// The context can be used to resolve references.
    fn decompress(c: &Self::Compressed, ctx: &Ctx) -> anyhow::Result<Self>;
}

/// A context that can be used to compress a type.
pub trait CompressionContext<Type>
where
    Type: Compressible,
{
    /// Perform compression, returning the compressed data and possibly modifying the
    /// context. The context is mutable to allow for stateful compression.
    /// For instance, it can be used to extract original data when replacing it with
    /// references.
    fn compress(&mut self, value: &Type) -> anyhow::Result<Type::Compressed>;
}

/// A context that can be used to decompress a type.
pub trait DecompressionContext<Type>
where
    Type: Compressible,
{
    /// Perform decompression, returning the original data.
    /// The context can be used to resolve references.
    fn decompress(&self, value: &Type::Compressed) -> anyhow::Result<Type>;
}

/// Uses a compression context to substitute a type with a reference.
/// This is used instead of `CompressibleBy` when the type is substitutable by
/// a reference. Used with `da_compress(registry = "keyspace")` attribute from
/// `fuel-derive::Compressed`.
pub trait RegistrySubstitutableBy<Ctx>: Compressible
where
    Ctx: ?Sized,
{
    /// Perform substitution, returning the reference and possibly modifying the context.
    /// Typically the original value is stored into the context.
    fn substitute(&self, keyspace: &str, ctx: &mut Ctx) -> anyhow::Result<RawKey>;
}

/// Uses a decompression context
/// This is used instead of `DecompressibleBy` when the type is desubstitutable from
/// a reference. Used with `da_compress(registry = "keyspace")` attribute from
/// `fuel-derive::Compressed`.
pub trait RegistryDesubstitutableBy<Ctx>: Compressible
where
    Ctx: ?Sized,
    Self: Sized,
{
    /// Perform desubstitution, returning the original value.
    /// The context is typically used to resolve the reference.
    fn desubstitute(c: &RawKey, keyspace: &str, ctx: &Ctx) -> anyhow::Result<Self>;
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
