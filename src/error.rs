//! Runtime interpreter error implementation

use fuel_asm::{Instruction, InstructionResult, PanicReason};
use fuel_tx::ValidationError;

use std::convert::Infallible as StdInfallible;
use std::error::Error as StdError;
use std::{fmt, io};
use thiserror::Error;

/// Interpreter runtime error variants.
#[derive(Debug, Error)]
pub enum InterpreterError {
    /// The instructions execution resulted in a well-formed panic, caused by an
    /// explicit instruction.
    #[error("Execution error: {0:?}")]
    PanicInstruction(InstructionResult),
    /// The VM execution resulted in a well-formed panic. This panic wasn't
    /// caused by an instruction contained in the transaction or a called
    /// contract.
    #[error("Execution error: {0:?}")]
    Panic(PanicReason),
    /// The provided transaction isn't valid.
    #[error("Failed to validate the transaction: {0}")]
    ValidationError(#[from] ValidationError),
    /// The predicate verification failed.
    #[error("Execution error")]
    PredicateFailure,
    /// No transaction was initialized in the interpreter. It cannot provide
    /// state transitions.
    #[error("Execution error")]
    NoTransactionInitialized,
    /// I/O and OS related errors.
    #[error("Unrecoverable error: {0}")]
    Io(#[from] io::Error),

    #[cfg(feature = "debug")]
    #[error("Execution error")]
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

impl From<InstructionResult> for InterpreterError {
    fn from(r: InstructionResult) -> InterpreterError {
        Self::PanicInstruction(r)
    }
}

impl From<RuntimeError> for InterpreterError {
    fn from(error: RuntimeError) -> Self {
        match error {
            RuntimeError::Recoverable(e) => Self::Panic(e),
            RuntimeError::Halt(e) => Self::Io(e),
        }
    }
}

impl From<InterpreterError> for io::Error {
    fn from(e: InterpreterError) -> Self {
        io::Error::new(io::ErrorKind::Other, e)
    }
}

impl PartialEq for InterpreterError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::PanicInstruction(s), Self::PanicInstruction(o)) => s == o,
            (Self::Panic(s), Self::Panic(o)) => s == o,
            (Self::ValidationError(s), Self::ValidationError(o)) => s == o,
            (Self::PredicateFailure, Self::PredicateFailure) => true,
            (Self::NoTransactionInitialized, Self::NoTransactionInitialized) => true,
            (Self::Io(s), Self::Io(o)) => s.kind() == o.kind(),

            #[cfg(feature = "debug")]
            (Self::DebugStateNotInitialized, Self::DebugStateNotInitialized) => true,

            _ => false,
        }
    }
}

#[derive(Debug, Error)]
/// Runtime error description that should either be specified in the protocol or
/// halt the execution.
pub enum RuntimeError {
    /// Specified error with well-formed fallback strategy.
    #[error(transparent)]
    Recoverable(#[from] PanicReason),
    /// Unspecified error that should halt the execution.
    #[error(transparent)]
    Halt(#[from] io::Error), // TODO: a more generic error type
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

impl PartialEq for RuntimeError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (RuntimeError::Recoverable(s), RuntimeError::Recoverable(o)) => s == o,
            (RuntimeError::Halt(s), RuntimeError::Halt(o)) => s.kind() == o.kind(),

            _ => false,
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

/// Unique bug identifier
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "strum", derive(strum::EnumVariantNames))]
pub enum BugId {
    ID001,
    ID002,
    ID003,
    ID004,
    ID005,
    ID006,
}

/// Traceable bug variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bug {
    /// Context gas increase has overflow
    ContextGasOverflow(BugId),

    /// Context gas increase has underflow
    ContextGasUnderflow(BugId),

    /// Global gas subtraction has underflow
    GlobalGasUnderflow(BugId),
}

impl Bug {
    /// Return the unique bug identifier per location
    pub const fn id(&self) -> BugId {
        match self {
            Bug::ContextGasOverflow(id) | Bug::ContextGasUnderflow(id) | Bug::GlobalGasUnderflow(id) => *id,
        }
    }
}

impl fmt::Display for Bug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "This is a bug [{:?}]! Please, report this incident as an issue in fuel-vm repository\n\n",
            self.id()
        )?;

        match self {
            Bug::ContextGasOverflow(_id) => write!(
                f,
                r#"The context gas cannot overflow since it was created by a valid transaction and the total gas does not increase - hence, it always fits a word.

                This overflow means the registers are corrupted."#
            ),

            Bug::ContextGasUnderflow(_id) => write!(
                f,
                r#"The context gas cannot underflow since any script should halt upon gas exhaustion.

                This underflow means the registers are corrupted."#
            ),

            Bug::GlobalGasUnderflow(_id) => write!(
                f,
                r#"The gas consumption cannot exceed the gas context since it is capped by the transaction gas limit.

                This underflow means the registers are corrupted."#
            ),
        }
    }
}

impl StdError for Bug {}

impl From<Bug> for RuntimeError {
    fn from(bug: Bug) -> Self {
        Self::Halt(io::Error::new(io::ErrorKind::Other, bug))
    }
}

impl From<Bug> for InterpreterError {
    fn from(bug: Bug) -> Self {
        RuntimeError::from(bug).into()
    }
}
