//! Runtime interpreter error implementation

use fuel_asm::{
    PanicInstruction,
    PanicReason,
    RawInstruction,
    Word,
};
use fuel_tx::ValidityError;

use crate::checked_transaction::CheckError;
use alloc::{
    format,
    string::String,
};
use core::{
    convert::Infallible,
    fmt,
};

use crate::storage::predicate;

/// Interpreter runtime error variants.
#[derive(Debug, derive_more::Display)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InterpreterError<StorageError> {
    /// The instructions execution resulted in a well-formed panic, caused by an
    /// explicit instruction.
    #[display(fmt = "Execution error: {_0:?}")]
    PanicInstruction(PanicInstruction),
    /// The VM execution resulted in a well-formed panic. This panic wasn't
    /// caused by an instruction contained in the transaction or a called
    /// contract.
    #[display(fmt = "Execution error: {_0:?}")]
    Panic(PanicReason),
    /// Failed while checking the transaction.
    #[display(fmt = "Failed to check the transaction: {_0:?}")]
    CheckError(CheckError),
    /// No transaction was initialized in the interpreter. It cannot provide
    /// state transitions.
    #[display(fmt = "Execution error")]
    NoTransactionInitialized,
    #[display(fmt = "Execution error")]
    /// The debug state is not initialized; debug routines can't be called.
    DebugStateNotInitialized,
    /// Storage I/O error
    #[display(fmt = "Storage error: {}", _0)]
    Storage(StorageError),
    /// The `Ready` transaction provided to `Interpreter` doesn't have expected gas price
    #[display(
        fmt = "The transaction's gas price is wrong: expected {expected}, got {actual}"
    )]
    ReadyTransactionWrongGasPrice {
        /// Expected gas price
        expected: Word,
        /// Actual gas price
        actual: Word,
    },
}

impl<StorageError> InterpreterError<StorageError> {
    /// Describe the error as recoverable or halt.
    pub fn from_runtime(
        error: RuntimeError<StorageError>,
        instruction: RawInstruction,
    ) -> Self {
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

impl<StorageError> InterpreterError<StorageError>
where
    StorageError: fmt::Debug,
{
    /// Make non-generic by converting the storage error to a string.
    pub fn erase_generics(&self) -> InterpreterError<String> {
        match self {
            Self::Storage(e) => InterpreterError::Storage(format!("{e:?}")),
            Self::PanicInstruction(e) => InterpreterError::PanicInstruction(*e),
            Self::Panic(e) => InterpreterError::Panic(*e),
            Self::NoTransactionInitialized => InterpreterError::NoTransactionInitialized,
            Self::DebugStateNotInitialized => InterpreterError::DebugStateNotInitialized,
            Self::CheckError(e) => InterpreterError::CheckError(e.clone()),
            InterpreterError::ReadyTransactionWrongGasPrice { expected, actual } => {
                InterpreterError::ReadyTransactionWrongGasPrice {
                    expected: *expected,
                    actual: *actual,
                }
            }
        }
    }
}

impl<StorageError> From<RuntimeError<StorageError>> for InterpreterError<StorageError> {
    fn from(error: RuntimeError<StorageError>) -> Self {
        match error {
            RuntimeError::Recoverable(e) => Self::Panic(e),
            RuntimeError::Storage(e) => Self::Storage(e),
        }
    }
}

impl<StorageError> PartialEq for InterpreterError<StorageError>
where
    StorageError: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::PanicInstruction(s), Self::PanicInstruction(o)) => s == o,
            (Self::Panic(s), Self::Panic(o)) => s == o,
            (Self::NoTransactionInitialized, Self::NoTransactionInitialized) => true,
            (Self::Storage(a), Self::Storage(b)) => a == b,
            (Self::DebugStateNotInitialized, Self::DebugStateNotInitialized) => true,

            _ => false,
        }
    }
}

impl<StorageError> From<PanicReason> for InterpreterError<StorageError> {
    fn from(reason: PanicReason) -> Self {
        Self::Panic(reason)
    }
}

impl<StorageError> From<Infallible> for InterpreterError<StorageError> {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

impl<StorageError> From<ValidityError> for InterpreterError<StorageError> {
    fn from(err: ValidityError) -> Self {
        Self::CheckError(CheckError::Validity(err))
    }
}

/// Runtime error description that should either be specified in the protocol or
/// halt the execution.
#[derive(Debug)]
#[must_use]
pub enum RuntimeError<StorageError> {
    /// Specified error with well-formed fallback strategy, i.e. vm panics.
    Recoverable(PanicReason),
    /// Storage io error
    Storage(StorageError),
}

impl<StorageError> RuntimeError<StorageError> {
    /// Flag whether the error is recoverable.
    pub const fn is_recoverable(&self) -> bool {
        matches!(self, Self::Recoverable(_))
    }

    /// Flag whether the error must halt the execution.
    pub const fn must_halt(&self) -> bool {
        !self.is_recoverable()
    }
}

impl<StorageError: PartialEq> PartialEq for RuntimeError<StorageError> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (RuntimeError::Recoverable(a), RuntimeError::Recoverable(b)) => a == b,
            (RuntimeError::Storage(a), RuntimeError::Storage(b)) => a == b,
            _ => false,
        }
    }
}

impl<StorageError: core::fmt::Debug> fmt::Display for RuntimeError<StorageError> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Recoverable(reason) => write!(f, "Recoverable error: {}", reason),
            Self::Storage(err) => write!(f, "Unrecoverable storage error: {:?}", err),
        }
    }
}

impl<StorageError> From<PanicReason> for RuntimeError<StorageError> {
    fn from(value: PanicReason) -> Self {
        Self::Recoverable(value)
    }
}

impl<StorageError> From<core::array::TryFromSliceError> for RuntimeError<StorageError> {
    fn from(value: core::array::TryFromSliceError) -> Self {
        Self::Recoverable(value.into())
    }
}

impl<StorageError> From<Infallible> for RuntimeError<StorageError> {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

/// Predicates checking failed
#[derive(Debug, Clone, PartialEq, derive_more::Display)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PredicateVerificationFailed {
    /// The predicate did not use the amount of gas provided
    #[display(fmt = "Predicate used less than the required amount of gas")]
    GasMismatch,
    /// The transaction doesn't contain enough gas to evaluate the predicate
    #[display(fmt = "Insufficient gas available for single predicate")]
    OutOfGas,
    /// The predicate owner does not correspond to the predicate code
    #[display(fmt = "Predicate owner invalid, doesn't match code root")]
    InvalidOwner,
    /// The predicate wasn't successfully evaluated to true
    #[display(fmt = "Predicate failed to evaluate")]
    False,
    /// The predicate gas used was not specified before execution
    #[display(fmt = "Predicate failed to evaluate")]
    GasNotSpecified,
    /// The transaction's `max_gas` is greater than the global gas limit.
    #[display(fmt = "Transaction exceeds total gas allowance {_0:?}")]
    TransactionExceedsTotalGasAllowance(Word),
    /// The cumulative gas overflowed the u64 accumulator
    #[display(fmt = "Cumulative gas computation overflowed the u64 accumulator")]
    GasOverflow,
    /// The VM execution resulted in a well-formed panic, caused by an instruction.
    #[display(fmt = "Execution error: {_0:?}")]
    PanicInstruction(PanicInstruction),
    /// The VM execution resulted in a well-formed panic not caused by an instruction.
    #[display(fmt = "Execution error: {_0:?}")]
    Panic(PanicReason),
    /// Predicate verification failed since it attempted to access storage
    #[display(
        fmt = "Predicate verification failed since it attempted to access storage"
    )]
    Storage,
}

impl From<InterpreterError<predicate::StorageUnavailable>>
    for PredicateVerificationFailed
{
    fn from(error: InterpreterError<predicate::StorageUnavailable>) -> Self {
        match error {
            error if error.panic_reason() == Some(PanicReason::OutOfGas) => {
                PredicateVerificationFailed::OutOfGas
            }
            InterpreterError::Panic(reason) => PredicateVerificationFailed::Panic(reason),
            InterpreterError::PanicInstruction(result) => {
                PredicateVerificationFailed::PanicInstruction(result)
            }
            InterpreterError::Storage(_) => PredicateVerificationFailed::Storage,
            _ => PredicateVerificationFailed::False,
        }
    }
}

impl From<PanicReason> for PredicateVerificationFailed {
    fn from(reason: PanicReason) -> Self {
        Self::Panic(reason)
    }
}

/// Result of a operation that doesn't access storage
pub type SimpleResult<T> = Result<T, PanicReason>;

/// Result of a operation that accesses storage
pub type IoResult<T, S> = Result<T, RuntimeError<S>>;
