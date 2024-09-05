//! Trait impls for Rust types

use super::traits::*;
use crate::RegistryKey;
use core::mem::MaybeUninit;
use fuel_types::{
    Address,
    AssetId,
    BlobId,
    BlockHeight,
    Bytes32,
    ContractId,
    Nonce,
    Salt,
};

macro_rules! identity_compression {
    ($t:ty) => {
        impl Compressible for $t {
            type Compressed = Self;
        }

        impl<Ctx, E> CompressibleBy<Ctx, E> for $t
        where
            Ctx: ?Sized,
        {
            async fn compress_with(&self, _: &mut Ctx) -> Result<Self, E> {
                Ok(*self)
            }
        }

        impl<Ctx, E> DecompressibleBy<Ctx, E> for $t
        where
            Ctx: ?Sized,
        {
            async fn decompress_with(c: &Self::Compressed, _: &Ctx) -> Result<Self, E> {
                Ok(*c)
            }
        }
    };
}

identity_compression!(u8);
identity_compression!(u16);
identity_compression!(u32);
identity_compression!(u64);
identity_compression!(u128);

identity_compression!(BlockHeight);
identity_compression!(BlobId);
identity_compression!(Bytes32);
identity_compression!(Salt);
identity_compression!(Nonce);

macro_rules! array_types_compression {
    ($t:ty, $compressed_t:ty) => {
        impl Compressible for $t {
            type Compressed = $compressed_t;
        }

        impl<Ctx, E> CompressibleBy<Ctx, E> for $t
        where
            Ctx: CompressionContext<$t, Error = E>,
            Ctx: ?Sized,
        {
            async fn compress_with(&self, ctx: &mut Ctx) -> Result<$compressed_t, E> {
                ctx.compress(self).await
            }
        }

        impl<Ctx, E> DecompressibleBy<Ctx, E> for $t
        where
            Ctx: DecompressionContext<$t, Error = E>,
            Ctx: ?Sized,
        {
            async fn decompress_with(
                value: &Self::Compressed,
                ctx: &Ctx,
            ) -> Result<$t, E> {
                ctx.decompress(value).await
            }
        }
    };
}

array_types_compression!(Address, RegistryKey);
array_types_compression!(ContractId, RegistryKey);
array_types_compression!(AssetId, RegistryKey);

impl<const S: usize, T> Compressible for [T; S]
where
    T: Compressible,
{
    type Compressed = [T::Compressed; S];
}

impl<const S: usize, T, Ctx, E> CompressibleBy<Ctx, E> for [T; S]
where
    T: CompressibleBy<Ctx, E>,
{
    #[allow(unsafe_code)]
    async fn compress_with(&self, ctx: &mut Ctx) -> Result<Self::Compressed, E> {
        // SAFETY: we are claiming to have initialized an array of `MaybeUninit`s,
        // which do not require initialization.
        let mut tmp: [MaybeUninit<T::Compressed>; S] =
            unsafe { MaybeUninit::uninit().assume_init() };

        // Dropping a `MaybeUninit` does nothing, so we can just overwrite the array.
        // TODO: Handle the case of the error. Currently it will cause a memory leak.
        //  https://github.com/FuelLabs/fuel-vm/issues/811
        for (v, empty) in self.iter().zip(tmp.iter_mut()) {
            unsafe {
                core::ptr::write(empty.as_mut_ptr(), v.compress_with(ctx).await?);
            }
        }

        // SAFETY: Every element is initialized.
        let result = tmp.map(|v| unsafe { v.assume_init() });
        Ok(result)
    }
}

impl<const S: usize, T, Ctx, E> DecompressibleBy<Ctx, E> for [T; S]
where
    T: DecompressibleBy<Ctx, E> + Clone,
{
    #[allow(unsafe_code)]
    async fn decompress_with(c: &Self::Compressed, ctx: &Ctx) -> Result<Self, E> {
        // SAFETY: we are claiming to have initialized an array of `MaybeUninit`s,
        // which do not require initialization.
        let mut tmp: [MaybeUninit<T>; S] = unsafe { MaybeUninit::uninit().assume_init() };

        // Dropping a `MaybeUninit` does nothing, so we can just overwrite the array.
        // TODO: Handle the case of the error. Currently it will cause a memory leak.
        //  https://github.com/FuelLabs/fuel-vm/issues/811
        for (v, empty) in c.iter().zip(tmp.iter_mut()) {
            unsafe {
                core::ptr::write(empty.as_mut_ptr(), T::decompress_with(v, ctx).await?);
            }
        }

        // SAFETY: Every element is initialized.
        let result = tmp.map(|v| unsafe { v.assume_init() });
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
    async fn compress_with(&self, ctx: &mut Ctx) -> Result<Self::Compressed, E> {
        let mut result = Vec::with_capacity(self.len());
        for item in self {
            result.push(item.compress_with(ctx).await?);
        }
        Ok(result)
    }
}

impl<T, Ctx, E> DecompressibleBy<Ctx, E> for Vec<T>
where
    T: DecompressibleBy<Ctx, E> + Clone,
{
    async fn decompress_with(c: &Self::Compressed, ctx: &Ctx) -> Result<Self, E> {
        let mut result = Vec::with_capacity(c.len());
        for item in c {
            result.push(T::decompress_with(item, ctx).await?);
        }
        Ok(result)
    }
}
