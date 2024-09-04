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
            fn compress(&self, _: &mut Ctx) -> Result<Self, E> {
                Ok(*self)
            }
        }

        impl<Ctx, E> DecompressibleBy<Ctx, E> for $t
        where
            Ctx: ?Sized,
        {
            fn decompress(c: &Self::Compressed, _: &Ctx) -> Result<Self, E> {
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
    fn compress(&self, ctx: &mut Ctx) -> Result<Self::Compressed, E> {
        self.as_ref().map(|item| item.compress(ctx)).transpose()
    }
}

impl<T, Ctx, E> DecompressibleBy<Ctx, E> for Option<T>
where
    T: DecompressibleBy<Ctx, E> + Clone,
{
    fn decompress(c: &Self::Compressed, ctx: &Ctx) -> Result<Self, E> {
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

impl<const S: usize, T, Ctx, E> CompressibleBy<Ctx, E> for [T; S]
where
    T: CompressibleBy<Ctx, E> + Clone,
{
    fn compress(&self, ctx: &mut Ctx) -> Result<Self::Compressed, E> {
        Ok(ArrayWrapper(try_map_array(self.clone(), |v: T| {
            v.compress(ctx)
        })?))
    }
}

impl<const S: usize, T, Ctx, E> DecompressibleBy<Ctx, E> for [T; S]
where
    T: DecompressibleBy<Ctx, E> + Clone,
{
    fn decompress(c: &Self::Compressed, ctx: &Ctx) -> Result<Self, E> {
        try_map_array(c.0.clone(), |v: T::Compressed| T::decompress(&v, ctx))
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
    fn compress(&self, ctx: &mut Ctx) -> Result<Self::Compressed, E> {
        self.iter().map(|item| item.compress(ctx)).collect()
    }
}

impl<T, Ctx, E> DecompressibleBy<Ctx, E> for Vec<T>
where
    T: DecompressibleBy<Ctx, E> + Clone,
{
    fn decompress(c: &Self::Compressed, ctx: &Ctx) -> Result<Self, E> {
        c.iter().map(|item| T::decompress(item, ctx)).collect()
    }
}

impl<T> Compressible for PhantomData<T> {
    type Compressed = ();
}

impl<T, Ctx, E> CompressibleBy<Ctx, E> for PhantomData<T> {
    fn compress(&self, _: &mut Ctx) -> Result<Self::Compressed, E> {
        Ok(())
    }
}

impl<T, Ctx, E> DecompressibleBy<Ctx, E> for PhantomData<T> {
    fn decompress(_: &Self::Compressed, _: &Ctx) -> Result<Self, E> {
        Ok(PhantomData)
    }
}
