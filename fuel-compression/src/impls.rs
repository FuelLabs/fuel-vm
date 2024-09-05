//! Trait impls for Rust types

use super::traits::*;
use core::{
    marker::PhantomData,
    mem::MaybeUninit,
};
use serde::{
    Deserialize,
    Serialize,
};

macro_rules! identity_compaction {
    ($t:ty) => {
        impl Compressible for $t {
            type Compressed = Self;
        }

        impl<Ctx, E> CompressibleBy<Ctx, E> for $t
        where
            Ctx: ?Sized,
        {
            async fn compress(&self, _: &mut Ctx) -> Result<Self, E> {
                Ok(*self)
            }
        }

        impl<Ctx, E> DecompressibleBy<Ctx, E> for $t
        where
            Ctx: ?Sized,
        {
            async fn decompress(c: &Self::Compressed, _: &Ctx) -> Result<Self, E> {
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

impl<T, Ctx, E> CompressibleBy<Ctx, E> for Option<T>
where
    T: CompressibleBy<Ctx, E> + Clone,
{
    async fn compress(&self, ctx: &mut Ctx) -> Result<Self::Compressed, E> {
        if let Some(item) = self {
            Ok(Some(item.compress(ctx).await?))
        } else {
            Ok(None)
        }
    }
}

impl<T, Ctx, E> DecompressibleBy<Ctx, E> for Option<T>
where
    T: DecompressibleBy<Ctx, E> + Clone,
{
    async fn decompress(c: &Self::Compressed, ctx: &Ctx) -> Result<Self, E> {
        if let Some(item) = c {
            Ok(Some(T::decompress(item, ctx).await?))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArrayWrapper<const S: usize, T: Serialize + for<'a> Deserialize<'a>>(
    #[serde(with = "serde_big_array::BigArray")] pub [T; S],
);

impl<const S: usize, T> Compressible for [T; S]
where
    T: Compressible + Clone,
{
    type Compressed = ArrayWrapper<S, T::Compressed>;
}

impl<const S: usize, T, Ctx, E> CompressibleBy<Ctx, E> for [T; S]
where
    T: CompressibleBy<Ctx, E> + Clone,
{
    #[allow(unsafe_code)]
    async fn compress(&self, ctx: &mut Ctx) -> Result<Self::Compressed, E> {
        // SAFETY: we are claiming to have initialized an array of `MaybeUninit`s,
        // which do not require initialization.
        let mut tmp: [MaybeUninit<T::Compressed>; S] =
            unsafe { MaybeUninit::uninit().assume_init() };

        // Dropping a `MaybeUninit` does nothing, so we can just overwrite the array.
        for (i, v) in self.iter().enumerate() {
            tmp[i] = MaybeUninit::new(v.compress(ctx).await?);
        }

        // SAFETY: Every element is initialized.
        let result = tmp.map(|v| unsafe { v.assume_init() });
        Ok(ArrayWrapper(result))
    }
}

impl<const S: usize, T, Ctx, E> DecompressibleBy<Ctx, E> for [T; S]
where
    T: DecompressibleBy<Ctx, E> + Clone,
{
    #[allow(unsafe_code)]
    async fn decompress(c: &Self::Compressed, ctx: &Ctx) -> Result<Self, E> {
        // SAFETY: we are claiming to have initialized an array of `MaybeUninit`s,
        // which do not require initialization.
        let mut tmp: [MaybeUninit<T>; S] = unsafe { MaybeUninit::uninit().assume_init() };

        // Dropping a `MaybeUninit` does nothing, so we can just overwrite the array.
        for (i, v) in c.0.iter().enumerate() {
            tmp[i] = MaybeUninit::new(T::decompress(v, ctx).await?);
        }

        // SAFETY: Every element is initialized.
        let result: [T; S] = tmp.map(|v| unsafe { v.assume_init() });
        Ok(result)
    }
}

impl<T> Compressible for Vec<T>
where
    T: Compressible + Clone,
{
    type Compressed = Vec<T::Compressed>;
}

impl<T, Ctx, E> CompressibleBy<Ctx, E> for Vec<T>
where
    T: CompressibleBy<Ctx, E> + Clone,
{
    async fn compress(&self, ctx: &mut Ctx) -> Result<Self::Compressed, E> {
        let mut result = Vec::with_capacity(self.len());
        for item in self {
            result.push(item.compress(ctx).await?);
        }
        Ok(result)
    }
}

impl<T, Ctx, E> DecompressibleBy<Ctx, E> for Vec<T>
where
    T: DecompressibleBy<Ctx, E> + Clone,
{
    async fn decompress(c: &Self::Compressed, ctx: &Ctx) -> Result<Self, E> {
        let mut result = Vec::with_capacity(c.len());
        for item in c {
            result.push(T::decompress(item, ctx).await?);
        }
        Ok(result)
    }
}

impl<T> Compressible for PhantomData<T> {
    type Compressed = ();
}

impl<T, Ctx, E> CompressibleBy<Ctx, E> for PhantomData<T> {
    async fn compress(&self, _: &mut Ctx) -> Result<Self::Compressed, E> {
        Ok(())
    }
}

impl<T, Ctx, E> DecompressibleBy<Ctx, E> for PhantomData<T> {
    async fn decompress(_: &Self::Compressed, _: &Ctx) -> Result<Self, E> {
        Ok(PhantomData)
    }
}
