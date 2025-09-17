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
                c: Self::Compressed,
                _: &Ctx,
            ) -> Result<Self, Ctx::Error> {
                Ok(c)
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

        let mut i = 0;
        while i < self.len() {
            match self[i].compress_with(ctx).await {
                Ok(value) => {
                    // SAFETY: MaybeUninit can be safely overwritten.
                    tmp[i].write(value);
                }
                Err(e) => {
                    // Drop the already initialized elements, so we don't leak the memory
                    for initialized_item in tmp.iter_mut().take(i) {
                        // Safety: First i elements have been initialized successfully.
                        unsafe {
                            initialized_item.assume_init_drop();
                        }
                    }
                    return Err(e);
                }
            }
            i += 1;
        }

        // SAFETY: Every element is initialized. In case of error, we have returned
        // instead.
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
    async fn decompress_with(c: Self::Compressed, ctx: &Ctx) -> Result<Self, Ctx::Error> {
        // SAFETY: we are claiming to have initialized an array of `MaybeUninit`s,
        // which do not require initialization.
        let mut tmp: [MaybeUninit<T>; S] = unsafe { MaybeUninit::uninit().assume_init() };

        for (i, c) in c.into_iter().enumerate() {
            match T::decompress_with(c, ctx).await {
                Ok(value) => {
                    // SAFETY: MaybeUninit can be safely overwritten.
                    tmp[i].write(value);
                }
                Err(e) => {
                    // Drop the already initialized elements, so we don't leak the memory
                    for initialized_item in tmp.iter_mut().take(i) {
                        // Safety: First i elements have been initialized successfully.
                        unsafe {
                            initialized_item.assume_init_drop();
                        }
                    }
                    return Err(e);
                }
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
    async fn decompress_with(c: Self::Compressed, ctx: &Ctx) -> Result<Self, Ctx::Error> {
        let mut result = Vec::with_capacity(c.len());
        for item in c {
            result.push(T::decompress_with(item, ctx).await?);
        }
        Ok(result)
    }
}

#[cfg(feature = "alloc")]
impl Compressible for fuel_types::bytes::Bytes {
    type Compressed = Self;
}

#[cfg(feature = "alloc")]
impl<Ctx> CompressibleBy<Ctx> for fuel_types::bytes::Bytes
where
    Ctx: ContextError,
{
    async fn compress_with(&self, _: &mut Ctx) -> Result<Self::Compressed, Ctx::Error> {
        Ok(self.clone())
    }
}

#[cfg(feature = "alloc")]
impl<Ctx> DecompressibleBy<Ctx> for fuel_types::bytes::Bytes
where
    Ctx: ContextError,
{
    async fn decompress_with(c: Self::Compressed, _: &Ctx) -> Result<Self, Ctx::Error> {
        Ok(c)
    }
}
