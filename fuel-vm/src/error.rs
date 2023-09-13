//! Runtime interpreter error implementation

use fuel_asm::{
    PanicInstruction,
    PanicReason,
    RawInstruction,
};
use fuel_tx::CheckError;
use thiserror::Error;

use core::{
    convert::Infallible as StdInfallible,
    fmt,
};

/// Interpreter runtime error variants.
#[derive(Debug, Error)]
pub enum InterpreterError {
    /// The instructions execution resulted in a well-formed panic, caused by an
    /// explicit instruction.
    #[error("Execution error: {0:?}")]
    PanicInstruction(PanicInstruction),
    /// The VM execution resulted in a well-formed panic. This panic wasn't
    /// caused by an instruction contained in the transaction or a called
    /// contract.
    #[error("Execution error: {0:?}")]
    Panic(PanicReason),
    /// The provided transaction isn't valid.
    #[error("Failed to check the transaction: {0}")]
    CheckError(#[from] CheckError),
    /// The predicate verification failed.
    #[error("Execution error")]
    PredicateFailure,
    /// No transaction was initialized in the interpreter. It cannot provide
    /// state transitions.
    #[error("Execution error")]
    NoTransactionInitialized,
    // /// I/O and OS related errors.
    // #[error("Unrecoverable error: {0}")]
    // Io(#[from] io::Error),
    #[error("Execution error")]
    /// The debug state is not initialized; debug routines can't be called.
    DebugStateNotInitialized,
    /// I/O and OS related errors.
    #[error("Runtime error: {0}")]
    RuntimeError(RuntimeError),
}

impl InterpreterError {
    /// Describe the error as recoverable or halt.
    pub fn from_runtime(error: RuntimeError, instruction: RawInstruction) -> Self {
        match error {
            RuntimeError::Recoverable(reason) => {
                Self::PanicInstruction(PanicInstruction::error(reason, instruction))
            }
            _ => Self::from(error),
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
    pub const fn instruction(&self) -> Option<&RawInstruction> {
        match self {
            Self::PanicInstruction(result) => Some(result.instruction()),
            _ => None,
        }
    }

    /// Return the underlying `InstructionResult` if this instance is
    /// `PanicInstruction`; returns `None` otherwise.
    pub fn instruction_result(&self) -> Option<PanicInstruction> {
        match self {
            Self::PanicInstruction(r) => Some(*r),
            _ => None,
        }
    }
}

impl From<RuntimeError> for InterpreterError {
    fn from(error: RuntimeError) -> Self {
        match error {
            RuntimeError::Recoverable(e) => Self::Panic(e),
            RuntimeError::Halt => todo!(), // TODO
        }
    }
}

impl PartialEq for InterpreterError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::PanicInstruction(s), Self::PanicInstruction(o)) => s == o,
            (Self::Panic(s), Self::Panic(o)) => s == o,
            (Self::CheckError(s), Self::CheckError(o)) => s == o,
            (Self::PredicateFailure, Self::PredicateFailure) => true,
            (Self::NoTransactionInitialized, Self::NoTransactionInitialized) => true,
            (Self::RuntimeError(s), Self::RuntimeError(o)) => todo!(), // TODO

            (Self::DebugStateNotInitialized, Self::DebugStateNotInitialized) => true,

            _ => false,
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
/// Runtime error description that should either be specified in the protocol or
/// halt the execution.
pub enum RuntimeError {
    /// Specified error with well-formed fallback strategy.
    #[error(transparent)]
    Recoverable(#[from] PanicReason),
    /// Unspecified error that should halt the execution.
    Halt, // TODO
}

impl RuntimeError {
    /// Flag whether the error is recoverable.
    pub const fn is_recoverable(&self) -> bool {
        matches!(self, Self::Recoverable(_))
    }

    /// Flag whether the error must halt the execution.
    pub const fn must_halt(&self) -> bool {
        matches!(self, Self::Halt)
    }

    /// Unexpected behavior occurred
    pub fn unexpected_behavior<E>(error: E) -> Self {
        todo!();
        Self::Halt // TODO: contents
    }
}

impl PartialEq for RuntimeError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (RuntimeError::Recoverable(s), RuntimeError::Recoverable(o)) => s == o,
            (RuntimeError::Halt, RuntimeError::Halt) => todo!(), // TODO
            _ => false,
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Recoverable(reason) => write!(f, "Recoverable error: {}", reason),
            Self::Halt => write!(f, "Unrecoverable error"),
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

impl From<core::array::TryFromSliceError> for RuntimeError {
    fn from(value: core::array::TryFromSliceError) -> Self {
        Self::Recoverable(value.into())
    }
}

/// Predicates checking failed
#[derive(Debug, Error)]
pub enum PredicateVerificationFailed {
    /// The predicate did not use the amount of gas provided
    #[error("Predicate used less than the required amount of gas")]
    GasMismatch,
    /// The transaction doesn't contain enough gas to evaluate the predicate
    #[error("Insufficient gas available for single predicate")]
    OutOfGas,
    /// The predicate owner does not correspond to the predicate code
    #[error("Predicate owner invalid, doesn't match code root")]
    InvalidOwner,
    /// The predicate wasn't successfully evaluated to true
    #[error("Predicate failed to evaluate")]
    False,
    /// The predicate gas used was not specified before execution
    #[error("Predicate failed to evaluate")]
    GasNotSpecified,
    /// The transaction doesn't contain enough gas to evaluate all predicates
    #[error("Insufficient gas available for all predicates")]
    CumulativePredicateGasExceededTxGasLimit,
    /// The cumulative gas overflowed the u64 accumulator
    #[error("Cumulative gas computation overflowed the u64 accumulator")]
    GasOverflow,
    /// TODO
    #[error("TODO")] // TODO
    RuntimeError,
}

impl From<PredicateVerificationFailed> for CheckError {
    fn from(error: PredicateVerificationFailed) -> Self {
        match error {
            PredicateVerificationFailed::OutOfGas => CheckError::PredicateExhaustedGas,
            _ => CheckError::PredicateVerificationFailed,
        }
    }
}

impl From<InterpreterError> for PredicateVerificationFailed {
    fn from(error: InterpreterError) -> Self {
        match error {
            error if error.panic_reason() == Some(PanicReason::OutOfGas) => {
                PredicateVerificationFailed::OutOfGas
            }
            InterpreterError::RuntimeError(e) => {
                PredicateVerificationFailed::RuntimeError
            }
            _ => PredicateVerificationFailed::False,
        }
    }
}

/// Unique bug identifier
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "strum", derive(strum::EnumVariantNames))]
pub enum BugId {
    // Not used
    ID001,
    ID002,
    ID003,
    ID004,
    // Not used
    ID005,
    // Not used
    ID006,
    ID007,
    ID008,
}

/// Traceable bug variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BugVariant {
    /// Context gas increase has overflow
    ContextGasOverflow,

    /// Context gas increase has underflow
    ContextGasUnderflow,

    /// Global gas subtraction has underflow
    GlobalGasUnderflow,

    /// The stack point has overflow
    StackPointerOverflow,

    /// The global gas is less than the context gas.
    GlobalGasLessThanContext,
}

impl fmt::Display for BugVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ContextGasOverflow => write!(
                f,
                r#"The context gas cannot overflow since it was created by a valid transaction and the total gas does not increase - hence, it always fits a word.

                This overflow means the registers are corrupted."#
            ),

            Self::ContextGasUnderflow => write!(
                f,
                r#"The context gas cannot underflow since any script should halt upon gas exhaustion.

                This underflow means the registers are corrupted."#
            ),

            Self::GlobalGasUnderflow => write!(
                f,
                r#"The gas consumption cannot exceed the gas context since it is capped by the transaction gas limit.

                This underflow means the registers are corrupted."#
            ),

            Self::StackPointerOverflow => write!(
                f,
                r#"The stack pointer cannot overflow under checked operations.

                This overflow means the registers are corrupted."#
            ),

            Self::GlobalGasLessThanContext => write!(
                f,
                r#"The global gas cannot ever be less than the context gas. 

                This means the registers are corrupted."#
            ),
        }
    }
}

/// Bug information with backtrace data
#[derive(Debug, Clone)]
pub struct Bug {
    id: BugId,
    variant: BugVariant,

    #[cfg(feature = "backtrace")]
    bt: backtrace::Backtrace,
}

impl Bug {
    #[cfg(not(feature = "backtrace"))]
    /// Report a bug without backtrace data
    pub const fn new(id: BugId, variant: BugVariant) -> Self {
        Self { id, variant }
    }

    /// Unique bug identifier per location
    pub const fn id(&self) -> BugId {
        self.id
    }

    /// Class variant of the bug
    pub const fn variant(&self) -> BugVariant {
        self.variant
    }
}

#[cfg(feature = "backtrace")]
mod bt {
    use super::*;
    use backtrace::Backtrace;
    use core::ops::Deref;

    impl Bug {
        /// Report a bug with backtrace data
        pub fn new(id: BugId, variant: BugVariant) -> Self {
            let bt = Backtrace::new();

            Self { id, variant, bt }
        }

        /// Backtrace data
        pub const fn bt(&self) -> &Backtrace {
            &self.bt
        }
    }

    impl Deref for Bug {
        type Target = Backtrace;

        fn deref(&self) -> &Self::Target {
            &self.bt
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

        write!(f, "{}", self.variant())?;

        Ok(())
    }
}

impl From<Bug> for InterpreterError {
    fn from(bug: Bug) -> Self {
        RuntimeError::from(bug).into()
    }
}

impl From<Bug> for PredicateVerificationFailed {
    fn from(bug: Bug) -> Self {
        let e: InterpreterError = bug.into();
        e.into()
    }
}
