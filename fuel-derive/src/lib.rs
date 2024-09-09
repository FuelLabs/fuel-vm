//! Derive macros for canonical type serialization and deserialization.

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![deny(unused_must_use, unsafe_code, unused_crate_dependencies, missing_docs)]
#![deny(
    clippy::arithmetic_side_effects,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::string_slice
)]

mod helpers;

extern crate proc_macro;

mod canonical {
    mod attribute;
    pub mod deserialize;
    pub mod serialize;
}

synstructure::decl_derive!(
    [Serialize, attributes(canonical)] =>
    /// Derives `Serialize` trait for the given `struct` or `enum`.
    canonical::serialize::derive
);
synstructure::decl_derive!(
    [Deserialize, attributes(canonical)] =>
    /// Derives `Deserialize` trait for the given `struct` or `enum`.
    canonical::deserialize::derive
);

mod compression {
    mod attribute;
    pub mod compress;
    pub mod decompress;
}

synstructure::decl_derive!(
    [Compress, attributes(compress)] =>
    /// Derives `Compressible` and `CompressibleBy` traits for the given `struct` or `enum`.
    compression::compress::derive
);
synstructure::decl_derive!(
    [Decompress, attributes(compress)] =>
    /// Derives `DecompressibleBy` trait for the given `struct` or `enum`.
    compression::decompress::derive
);
