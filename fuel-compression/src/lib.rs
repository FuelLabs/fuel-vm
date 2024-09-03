//! Compression and decompression of fuel-types for the DA layer

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![warn(missing_docs)]
#![deny(unsafe_code)]
#![deny(unused_crate_dependencies)]
#![deny(clippy::cast_possible_truncation)]

mod compaction;
mod key;

pub use compaction::{
    Compressible,
    CompressibleBy,
    CompressionContext,
    DecompressibleBy,
    DecompressionContext,
    RegistryDesubstitutableBy,
    RegistrySubstitutableBy,
};
pub use key::RawKey;

pub use fuel_derive::Compressed;
