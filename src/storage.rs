//! Storage backend implementations.

mod interpreter;
mod memory;
mod predicate;

pub use interpreter::InterpreterStorage;
pub use memory::MemoryStorage;
pub use predicate::PredicateStorage;
