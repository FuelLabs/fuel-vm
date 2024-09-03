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
