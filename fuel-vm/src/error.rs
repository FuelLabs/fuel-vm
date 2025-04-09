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
    string::{
        String,
        ToString,
    },
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
    /// Encountered a bug
    #[display(fmt = "Bug: {_0}")]
    Bug(Bug),
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
            Self::Bug(e) => InterpreterError::Bug(e.clone()),
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
            RuntimeError::Bug(e) => Self::Bug(e),
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

impl<StorageError> From<Bug> for InterpreterError<StorageError> {
    fn from(bug: Bug) -> Self {
        Self::Bug(bug)
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
    /// Invalid interpreter state reached unexpectedly, this is a bug
    Bug(Bug),
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
            (RuntimeError::Bug(a), RuntimeError::Bug(b)) => a == b,
            (RuntimeError::Storage(a), RuntimeError::Storage(b)) => a == b,
            _ => false,
        }
    }
}

impl<StorageError: core::fmt::Debug> fmt::Display for RuntimeError<StorageError> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Recoverable(reason) => write!(f, "Recoverable error: {}", reason),
            Self::Bug(err) => write!(f, "Bug: {}", err),
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

impl<StorageError> From<Bug> for RuntimeError<StorageError> {
    fn from(bug: Bug) -> Self {
        Self::Bug(bug)
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
    /// Invalid interpreter state reached unexpectedly, this is a bug
    #[display(fmt = "Invalid interpreter state reached unexpectedly")]
    Bug(Bug),
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

impl From<InterpreterError<predicate::PredicateStorageError>>
    for PredicateVerificationFailed
{
    fn from(error: InterpreterError<predicate::PredicateStorageError>) -> Self {
        match error {
            error if error.panic_reason() == Some(PanicReason::OutOfGas) => {
                PredicateVerificationFailed::OutOfGas
            }
            InterpreterError::Panic(reason) => PredicateVerificationFailed::Panic(reason),
            InterpreterError::PanicInstruction(result) => {
                PredicateVerificationFailed::PanicInstruction(result)
            }
            InterpreterError::Bug(bug) => PredicateVerificationFailed::Bug(bug),
            InterpreterError::Storage(_) => PredicateVerificationFailed::Storage,
            _ => PredicateVerificationFailed::False,
        }
    }
}

impl From<Bug> for PredicateVerificationFailed {
    fn from(bug: Bug) -> Self {
        Self::Bug(bug)
    }
}

impl From<PanicReason> for PredicateVerificationFailed {
    fn from(reason: PanicReason) -> Self {
        Self::Panic(reason)
    }
}

impl From<PanicOrBug> for PredicateVerificationFailed {
    fn from(err: PanicOrBug) -> Self {
        match err {
            PanicOrBug::Panic(reason) => Self::from(reason),
            PanicOrBug::Bug(bug) => Self::Bug(bug),
        }
    }
}

/// Traceable bug variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumMessage)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BugVariant {
    /// Context gas increase has overflow
    #[strum(
        message = "The context gas cannot overflow since it was created by a valid transaction and the total gas does not increase - hence, it always fits a word."
    )]
    ContextGasOverflow,

    /// Context gas increase has underflow
    #[strum(
        message = "The context gas cannot underflow since any script should halt upon gas exhaustion."
    )]
    ContextGasUnderflow,

    /// Global gas subtraction has underflow
    #[strum(
        message = "The gas consumption cannot exceed the gas context since it is capped by the transaction gas limit."
    )]
    GlobalGasUnderflow,

    /// The global gas is less than the context gas.
    #[strum(message = "The global gas cannot ever be less than the context gas. ")]
    GlobalGasLessThanContext,

    /// The stack point has overflow
    #[strum(message = "The stack pointer cannot overflow under checked operations.")]
    StackPointerOverflow,

    /// Code size of a contract doesn't fit into a Word. This is prevented by tx size
    /// limit.
    #[strum(message = "Contract size doesn't fit into a word.")]
    CodeSizeOverflow,

    /// Refund cannot be computed in the current vm state.
    #[strum(message = "Refund cannot be computed in the current vm state.")]
    UncomputableRefund,

    /// Receipts context is full, but there's an attempt to add more receipts.
    #[strum(message = "Receipts context is full, cannot add new receipts.")]
    ReceiptsCtxFull,

    /// Witness index is out of bounds.
    #[strum(message = "Witness index is out of bounds.")]
    WitnessIndexOutOfBounds,

    /// The witness subsection index is higher than the total number of parts.
    #[strum(
        message = "The witness subsection index is higher than the total number of parts."
    )]
    NextSubsectionIndexIsHigherThanTotalNumberOfParts,

    /// Input index more than u16::MAX was used internally.
    #[strum(message = "Input index more than u16::MAX was used internally.")]
    InputIndexMoreThanU16Max,
}

impl fmt::Display for BugVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use strum::EnumMessage;
        if let Some(msg) = self.get_message() {
            write!(f, "{}", msg)
        } else {
            write!(f, "{:?}", self)
        }
    }
}

/// VM encountered unexpected state. This is a bug.
/// The execution must terminate since the VM is in an invalid state.
///
/// The bug it self is identified by the caller location.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[must_use]
pub struct Bug {
    /// Source code location of the bug, in `path/to/file.rs:line:column` notation
    location: String,

    /// Type of bug
    variant: BugVariant,

    /// Additional error message for the bug, if it's caused by a runtime error
    inner_message: Option<String>,

    /// Optionally include a backtrace for the instruction triggering this bug.
    /// This is only available when the `backtrace` feature is enabled.
    #[cfg(feature = "backtrace")]
    bt: backtrace::Backtrace,
}

impl Bug {
    /// Construct a new bug with the specified variant, using caller location for
    /// idenitfying the bug.
    #[track_caller]
    pub fn new(variant: BugVariant) -> Self {
        let caller = core::panic::Location::caller();
        let location = format!("{}:{}:{}", caller.file(), caller.line(), caller.column());
        Self {
            location,
            variant,
            inner_message: None,
            #[cfg(feature = "backtrace")]
            bt: backtrace::Backtrace::new(),
        }
    }

    /// Set an additional error message.
    pub fn with_message<E: ToString>(mut self, error: E) -> Self {
        self.inner_message = Some(error.to_string());
        self
    }
}

impl PartialEq for Bug {
    fn eq(&self, other: &Self) -> bool {
        self.location == other.location
    }
}

#[cfg(feature = "backtrace")]
mod bt {
    use super::*;
    use backtrace::Backtrace;

    impl Bug {
        /// Backtrace data
        pub const fn bt(&self) -> &Backtrace {
            &self.bt
        }
    }
}

impl fmt::Display for Bug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use percent_encoding::{
            utf8_percent_encode,
            NON_ALPHANUMERIC,
        };

        let issue_title = format!("Bug report: {:?} in {}", self.variant, self.location);

        let issue_body = format!(
            "Error: {:?} {}\nLocation: {}\nVersion: {} {}\n",
            self.variant,
            self.inner_message
                .as_ref()
                .map(|msg| format!("({msg})"))
                .unwrap_or_default(),
            self.location,
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        );

        write!(
            f,
            concat!(
                "Encountered a bug! Please report this using the following link: ",
                "https://github.com/FuelLabs/fuel-vm/issues/new",
                "?title={}",
                "&body={}",
                "\n\n",
                "{:?} error in {}: {} {}\n",
            ),
            utf8_percent_encode(&issue_title, NON_ALPHANUMERIC),
            utf8_percent_encode(&issue_body, NON_ALPHANUMERIC),
            self.variant,
            self.location,
            self.variant,
            self.inner_message
                .as_ref()
                .map(|msg| format!("({msg})"))
                .unwrap_or_default(),
        )?;

        #[cfg(feature = "backtrace")]
        {
            write!(f, "\nBacktrace:\n{:?}\n", self.bt)?;
        }

        Ok(())
    }
}

/// Runtime error description that should either be specified in the protocol or
/// halt the execution.
#[derive(Debug, Clone, PartialEq)]
#[must_use]
pub enum PanicOrBug {
    /// VM panic
    Panic(PanicReason),
    /// Invalid interpreter state reached unexpectedly, this is a bug
    Bug(Bug),
}

impl From<PanicReason> for PanicOrBug {
    fn from(panic: PanicReason) -> Self {
        Self::Panic(panic)
    }
}

impl From<Bug> for PanicOrBug {
    fn from(bug: Bug) -> Self {
        Self::Bug(bug)
    }
}

impl<StorageError> From<PanicOrBug> for RuntimeError<StorageError> {
    fn from(value: PanicOrBug) -> Self {
        match value {
            PanicOrBug::Panic(reason) => Self::Recoverable(reason),
            PanicOrBug::Bug(bug) => Self::Bug(bug),
        }
    }
}

impl<StorageError> From<PanicOrBug> for InterpreterError<StorageError> {
    fn from(value: PanicOrBug) -> Self {
        match value {
            PanicOrBug::Panic(reason) => Self::Panic(reason),
            PanicOrBug::Bug(bug) => Self::Bug(bug),
        }
    }
}

/// Result of a operation that doesn't access storage
pub type SimpleResult<T> = Result<T, PanicOrBug>;

/// Result of a operation that accesses storage
pub type IoResult<T, S> = Result<T, RuntimeError<S>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bug_report_message() {
        let bug = Bug::new(BugVariant::ContextGasOverflow).with_message("Test message");
        let text = format!("{}", bug);
        assert!(text.contains(file!()));
        assert!(text.contains("https://github.com/FuelLabs/fuel-vm/issues/new"));
        assert!(text.contains("ContextGasOverflow"));
        assert!(text.contains("Test message"));
    }
}
