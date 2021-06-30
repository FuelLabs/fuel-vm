#[cfg(feature = "debug")]
mod debugger;

#[cfg(feature = "debug")]
pub use debugger::{Breakpoint, DebugEval, Debugger};

#[derive(Debug, Default, Clone)]
#[cfg(not(feature = "debug"))]
pub struct Debugger {}
