//! Runtime interpreter error implementation

use fuel_asm::{Instruction, InstructionResult, PanicReason};
use fuel_tx::ValidationError;

use std::convert::Infallible as StdInfallible;
use std::error::Error as StdError;
use std::{fmt, io};

#[derive(Debug)]
/// Interpreter runtime error variants.
pub enum InterpreterError {
    /// The instructions execution resulted in a well-formed panic, caused by an
    /// explicit instruction.
    PanicInstruction(InstructionResult),
    /// The VM execution resulted in a well-formed panic. This panic wasn't
    /// caused by an instruction contained in the transaction or a called
    /// contract.
    Panic(PanicReason),
    /// The provided transaction isn't valid.
    ValidationError(ValidationError),
    /// The predicate verification failed.
    PredicateFailure,
    /// No transaction was initialized in the interpreter. It cannot provide
    /// state transitions.
    NoTransactionInitialized,
    /// I/O and OS related errors.
    Io(io::Error),

    #[cfg(feature = "debug")]
    /// The debug state is not initialized; debug routines can't be called.
    DebugStateNotInitialized,
}

impl InterpreterError {
    /// Describe the error as recoverable or halt.
    pub fn from_runtime(error: RuntimeError, instruction: Instruction) -> Self {
        match error {
            RuntimeError::Recoverable(reason) => Self::PanicInstruction(InstructionResult::error(reason, instruction)),
            RuntimeError::Halt(e) => Self::Io(e),
        }
    }

    /// Return the specified panic reason that caused this error, if applicable.
    pub const fn panic_reason(&self) -> Option<PanicReason> {
        match self {
            Self::PanicInstruction(result) => Some(*result.reason()),
            Self::Panic(reason) => Some(*reason),
            _ => None,
        }
    }

    /// Return the instruction that caused this error, if applicable.
    pub const fn instruction(&self) -> Option<&Instruction> {
        match self {
            Self::PanicInstruction(result) => Some(result.instruction()),
            _ => None,
        }
    }

    /// Return the underlying `InstructionResult` if this instance is
    /// `PanicInstruction`; returns `None` otherwise.
    pub fn instruction_result(&self) -> Option<&InstructionResult> {
        match self {
            Self::PanicInstruction(r) => Some(r),
            _ => None,
        }
    }

    /// Produces a `halt` error from `io`.
    pub fn from_io<E>(e: E) -> Self
    where
        E: Into<io::Error>,
    {
        Self::Io(e.into())
    }
}

impl From<io::Error> for InterpreterError {
    fn from(e: io::Error) -> Self {
        InterpreterError::Io(e)
    }
}

impl fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValidationError(e) => {
                write!(f, "Failed to validate the transaction: {}", e)
            }

            Self::Io(e) => {
                write!(f, "Unrecoverable error: {}", e)
            }

            _ => write!(f, "Execution error: {:?}", self),
        }
    }
}

impl StdError for InterpreterError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::ValidationError(e) => Some(e),
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<ValidationError> for InterpreterError {
    fn from(e: ValidationError) -> Self {
        Self::ValidationError(e)
    }
}

impl From<InstructionResult> for InterpreterError {
    fn from(r: InstructionResult) -> InterpreterError {
        Self::PanicInstruction(r)
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde-types-minimal", derive(serde::Serialize, serde::Deserialize))]
/// Runtime error description that should either be specified in the protocol or
/// halt the execution.
pub enum RuntimeError {
    /// Specified error with well-formed fallback strategy.
    Recoverable(PanicReason),
    /// Unspecified error that should halt the execution.
    Halt(io::Error),
}

impl RuntimeError {
    /// Flag whether the error is recoverable.
    pub const fn is_recoverable(&self) -> bool {
        matches!(self, Self::Recoverable(_))
    }

    /// Flag whether the error must halt the execution.
    pub const fn must_halt(&self) -> bool {
        matches!(self, Self::Halt(_))
    }

    /// Produces a `halt` error from `io`.
    pub fn from_io<E>(e: E) -> Self
    where
        E: Into<io::Error>,
    {
        Self::Halt(e.into())
    }
}

impl From<PanicReason> for RuntimeError {
    fn from(r: PanicReason) -> Self {
        RuntimeError::Recoverable(r)
    }
}

impl From<io::Error> for RuntimeError {
    fn from(e: io::Error) -> Self {
        RuntimeError::Halt(e)
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Recoverable(e) => e.fmt(f),
            Self::Halt(e) => e.fmt(f),
        }
    }
}

impl StdError for RuntimeError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Recoverable(e) => Some(e),
            Self::Halt(e) => Some(e),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Infallible implementation that converts into [`io::Error`].
pub struct Infallible(StdInfallible);

impl fmt::Display for Infallible {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl StdError for Infallible {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.0)
    }
}

impl<E> From<E> for Infallible
where
    E: Into<StdInfallible>,
{
    fn from(e: E) -> Infallible {
        Self(e.into())
    }
}

impl From<Infallible> for InterpreterError {
    fn from(_e: Infallible) -> InterpreterError {
        unreachable!()
    }
}

impl From<Infallible> for RuntimeError {
    fn from(_e: Infallible) -> RuntimeError {
        unreachable!()
    }
}

impl From<Infallible> for PanicReason {
    fn from(_e: Infallible) -> PanicReason {
        unreachable!()
    }
}

impl Into<io::Error> for Infallible {
    fn into(self) -> io::Error {
        unreachable!()
    }
}
