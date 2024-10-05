#![allow(async_fn_in_trait)] // We control the implementation so this is fine

/// Defines the error type for the context used in compression and decompression.
pub trait ContextError {
    /// The error type returned by the context.
    type Error;
}

/// This type can be compressed to a more compact form and back using
/// `CompressibleBy` and `DecompressibleBy` traits.
pub trait Compressible {
    /// The compressed type.
    type Compressed: Sized;
}

/// This type can be compressed to a more compact form and back using
/// `CompressionContext`.
pub trait CompressibleBy<Ctx>: Compressible
where
    Ctx: ContextError,
{
    /// Perform compression, returning the compressed data and possibly modifying the
    /// context. The context is mutable to allow for stateful compression.
    /// For instance, it can be used to extract original data when replacing it with
    /// references.
    async fn compress_with(&self, ctx: &mut Ctx) -> Result<Self::Compressed, Ctx::Error>;
}

/// This type can be decompressed using `CompressionContext`.
pub trait DecompressibleBy<Ctx>: Compressible
where
    Ctx: ContextError,
    Self: Sized,
{
    /// Perform decompression, returning the original data.
    /// The context can be used to resolve references.
    async fn decompress_with(c: Self::Compressed, ctx: &Ctx) -> Result<Self, Ctx::Error>;
}

/// The trait allows for decompression of a compressed type.
/// This trait is syntax sugar for `DecompressibleBy` with the compressed type as the
/// receiver.
pub trait Decompress<Decompressed, Ctx>
where
    Ctx: ContextError,
{
    /// Perform decompression, returning the original data.
    async fn decompress(self, ctx: &Ctx) -> Result<Decompressed, Ctx::Error>;
}

impl<T, Ctx, Decompressed> Decompress<Decompressed, Ctx> for T
where
    Ctx: ContextError,
    Decompressed: DecompressibleBy<Ctx, Compressed = Self>,
{
    async fn decompress(self, ctx: &Ctx) -> Result<Decompressed, Ctx::Error> {
        Decompressed::decompress_with(self, ctx).await
    }
}
