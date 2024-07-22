//! Compression and decompression of fuel-types for the DA layer

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![warn(missing_docs)]
#![deny(unsafe_code)]
#![deny(unused_crate_dependencies)]
#![deny(clippy::cast_possible_truncation)]

mod block_section;
mod compaction;
mod key;
mod table;

pub use compaction::Compactable;
pub use table::{
    tables,
    ChangesPerTable,
    CompactionContext,
    CountPerTable,
    DecompactionContext,
    Table,
};

pub use key::Key;

pub use fuel_derive::Compact;
