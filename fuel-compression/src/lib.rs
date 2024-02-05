//! Compression and decompression of fuel-types for the DA layer

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![warn(missing_docs)]
#![deny(unsafe_code)]
#![deny(unused_crate_dependencies)]
#![deny(clippy::cast_possible_truncation)]

mod compaction;
mod registry;

pub use compaction::{
    Compactable,
    CompactionContext,
};
pub use registry::{
    tables,
    ChangesPerTable,
    CountPerTable,
    Key,
    RegistryDb,
    Table,
};

#[cfg(feature = "test-helpers")]
pub use registry::in_memory::InMemoryRegistry;

pub use fuel_derive::Compact;
