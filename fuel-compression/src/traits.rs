#![allow(async_fn_in_trait)] // We control the implementation so this is fine

/// This type can be compressed to a more compact form and back using
/// `CompressibleBy` and `DecompressibleBy` traits.
pub trait Compressible {
    /// The compressed type.
    type Compressed: Sized;
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
    async fn compress_with(&self, ctx: &mut Ctx) -> Result<Self::Compressed, E>;
}

/// This type can be decompressed using `CompressionContext`.
pub trait DecompressibleBy<Ctx, E>: Compressible
where
    Ctx: ?Sized,
    Self: Sized,
{
    /// Perform decompression, returning the original data.
    /// The context can be used to resolve references.
    async fn decompress_with(c: &Self::Compressed, ctx: &Ctx) -> Result<Self, E>;
}
