// `pub(crate)` so the JIT (`interpreter::jit`) can name the `Execute` trait to call op
// impls directly from its specialized thunks.
pub(crate) mod instruction;
mod main;
mod predicate;

mod debug;
mod opcodes_impl;

pub use main::predicates;
