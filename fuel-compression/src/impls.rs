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

        impl<Ctx> CompressibleBy<Ctx> for $t
        where
            Ctx: ContextError,
        {
            async fn compress_with(&self, _: &mut Ctx) -> Result<Self, Ctx::Error> {
                Ok(*self)
            }
        }

        impl<Ctx> DecompressibleBy<Ctx> for $t
        where
            Ctx: ContextError,
        {
            async fn decompress_with(
                c: &Self::Compressed,
                _: &Ctx,
            ) -> Result<Self, Ctx::Error> {
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

impl Compressible for Address {
    type Compressed = RegistryKey;
}

impl Compressible for ContractId {
    type Compressed = RegistryKey;
}

impl Compressible for AssetId {
    type Compressed = RegistryKey;
}

impl<const S: usize, T> Compressible for [T; S]
where
    T: Compressible,
{
    type Compressed = [T::Compressed; S];
}

impl<const S: usize, T, Ctx> CompressibleBy<Ctx> for [T; S]
where
    T: CompressibleBy<Ctx>,
    Ctx: ContextError,
{
    #[allow(unsafe_code)]
    async fn compress_with(&self, ctx: &mut Ctx) -> Result<Self::Compressed, Ctx::Error> {
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

impl<const S: usize, T, Ctx> DecompressibleBy<Ctx> for [T; S]
where
    T: DecompressibleBy<Ctx>,
    Ctx: ContextError,
{
    #[allow(unsafe_code)]
    async fn decompress_with(
        c: &Self::Compressed,
        ctx: &Ctx,
    ) -> Result<Self, Ctx::Error> {
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
    T: Compressible,
{
    type Compressed = Vec<T::Compressed>;
}

impl<T, Ctx> CompressibleBy<Ctx> for Vec<T>
where
    T: CompressibleBy<Ctx>,
    Ctx: ContextError,
{
    async fn compress_with(&self, ctx: &mut Ctx) -> Result<Self::Compressed, Ctx::Error> {
        let mut result = Vec::with_capacity(self.len());
        for item in self {
            result.push(item.compress_with(ctx).await?);
        }
        Ok(result)
    }
}

impl<T, Ctx> DecompressibleBy<Ctx> for Vec<T>
where
    T: DecompressibleBy<Ctx>,
    Ctx: ContextError,
{
    async fn decompress_with(
        c: &Self::Compressed,
        ctx: &Ctx,
    ) -> Result<Self, Ctx::Error> {
        let mut result = Vec::with_capacity(c.len());
        for item in c {
            result.push(T::decompress_with(item, ctx).await?);
        }
        Ok(result)
    }
}
