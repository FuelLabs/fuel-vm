#[cfg(not(feature = "debug"))]
mod dummy;

#[cfg(feature = "debug")]
mod debugger;

#[cfg(not(feature = "debug"))]
pub use dummy::DummyDebugger as Debugger;

#[cfg(feature = "debug")]
pub use debugger::{Breakpoint, DebugEval, Debugger};
