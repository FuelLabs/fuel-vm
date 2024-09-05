//! Compression and decompression of fuel-types for the DA layer

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![warn(missing_docs)]
#![deny(unsafe_code)]
#![deny(unused_crate_dependencies)]
#![deny(clippy::cast_possible_truncation)]

mod impls;
mod key;
mod traits;

pub use key::RegistryKey;
pub use traits::*;

pub use fuel_derive::{
    Compress,
    Decompress,
};
