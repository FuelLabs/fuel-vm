extern crate proc_macro;

mod deserialize;
mod serialize;

use self::{
    deserialize::deserialize_derive,
    serialize::serialize_derive,
};
synstructure::decl_derive!(
    [Deserialize] =>
    /// Derives `Deserialize` trait for the given `struct` or `enum`.
    deserialize_derive
);
synstructure::decl_derive!(
    [Serialize] =>
    /// Derives `Serialize` trait for the given `struct` or `enum`.
    serialize_derive
);
