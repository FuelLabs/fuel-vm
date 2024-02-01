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
