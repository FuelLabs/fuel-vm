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

extern crate proc_macro;
mod canonical_attribute;
mod compressed;
mod deserialize;
mod serialize;

use self::{
    compressed::compressible_by,
    deserialize::deserialize_derive,
    serialize::serialize_derive,
};

synstructure::decl_derive!(
    [Deserialize, attributes(canonical)] =>
    /// Derives `Deserialize` trait for the given `struct` or `enum`.
    deserialize_derive
);
synstructure::decl_derive!(
    [Serialize, attributes(canonical)] =>
    /// Derives `Serialize` trait for the given `struct` or `enum`.
    serialize_derive
);
synstructure::decl_derive!(
    [CompressibleBy, attributes(compressible_by)] =>
    /// Derives `Compressible` and `CompressibleBy` trait for the given `struct` or `enum`.
    compressible_by
);
