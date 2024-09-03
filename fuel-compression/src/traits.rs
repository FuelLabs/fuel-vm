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

/// A context that can be used to compress a type.
pub trait CompressionContext<Type>
where
    Type: Compressible,
{
    /// Error when compressing. Note that the compression itself is not faillible,
    /// but the context may still do fallible operations.
    type Error;

    /// Perform compression, returning the compressed data and possibly modifying the
    /// context. The context is mutable to allow for stateful compression.
    /// For instance, it can be used to extract original data when replacing it with
    /// references.
    fn compress(&mut self, value: &Type) -> Result<Type::Compressed, Self::Error>;
}

/// A context that can be used to decompress a type.
pub trait DecompressionContext<Type>
where
    Type: Compressible,
{
    /// Error when compressing. Note that the compression itself is not faillible,
    /// but the context may still do fallible operations.
    type Error;

    /// Perform decompression, returning the original data.
    /// The context can be used to resolve references.
    fn decompress(&self, value: &Type::Compressed) -> Result<Type, Self::Error>;
}

/// Error type for context errors.
pub trait CtxError {
    /// Context error type
    type Error;
}

/// This type can be compressed to a more compact form and back using
/// `CompressionContext`.
pub trait CompressibleBy<Ctx, E>: Compressible
where
    Ctx: ?Sized,
{
    /// Perform compression, returning the compressed data and possibly modifying the
    /// context. The context is mutable to allow for stateful compression.
    /// For instance, it can be used to extract original data when replacing it with
    /// references.
    fn compress(&self, ctx: &mut Ctx) -> Result<Self::Compressed, E>;
}

/// This type can be decompressed using `CompressionContext`.
pub trait DecompressibleBy<Ctx, E>: Compressible
where
    Ctx: ?Sized,
    Self: Sized,
{
    /// Perform decompression, returning the original data.
    /// The context can be used to resolve references.
    fn decompress(c: &Self::Compressed, ctx: &Ctx) -> Result<Self, E>;
}

/// Uses a compression context to substitute a type with a reference.
/// This is used instead of `CompressibleBy` when the type is substitutable by
/// a reference. Used with `da_compress(registry)` attribute from
/// `fuel-derive::Compressed`.
#[diagnostic::on_unimplemented(
    message = "`fuel_compression::RegistrySubstitutableBy<_,_>` is not implemented for `{Self}`",
    label = "When trying to compress this parent type",
    note = "#[da_compress(registry)] was likely used on field with type {Self}"
)]
pub trait RegistrySubstitutableBy<Ctx, E>
where
    Ctx: ?Sized,
{
    /// Perform substitution, returning the reference and possibly modifying the context.
    /// Typically the original value is stored into the context.
    fn substitute(&self, ctx: &mut Ctx) -> Result<RawKey, E>;
}

/// Uses a decompression context to desubstitute a type from a reference.
/// This is used instead of `DecompressibleBy` when the type is desubstitutable from
/// a reference. Used with `da_compress(registry)` attribute from
/// `fuel-derive::Compressed`.
#[diagnostic::on_unimplemented(
    message = "`fuel_compression::RegistrySubstitutableBy<_,_>` is not implemented for `{Self}`",
    label = "When trying to decompress this parent type",
    note = "#[da_compress(registry)] was likely used on field with type {Self}"
)]
pub trait RegistryDesubstitutableBy<Ctx, E>
where
    Ctx: ?Sized,
    Self: Sized,
{
    /// Perform desubstitution, returning the original value.
    /// The context is typically used to resolve the reference.
    fn desubstitute(c: &RawKey, ctx: &Ctx) -> Result<Self, E>;
}
