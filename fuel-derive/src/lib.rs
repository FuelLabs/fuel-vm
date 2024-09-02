//! Derive macros for canonical type serialization and deserialization.

#![deny(unused_must_use, missing_docs)]
#![deny(
    clippy::arithmetic_side_effects,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::string_slice
)]

extern crate proc_macro;
mod canonical_attribute;
mod compact;
mod deserialize;
mod serialize;

use self::{
    compact::compact_derive,
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
    [Compressed, attributes(da_compress)] =>
    /// Derives `Compressed` trait for the given `struct` or `enum`.
    compact_derive
);
