use core::fmt;
use num_enum::{FromPrimitive, IntoPrimitive};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, IntoPrimitive, FromPrimitive, strum::EnumIter)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[repr(u8)]
#[non_exhaustive]
/// Panic reason representation for the interpreter.
pub enum PanicReason {
    /// The byte can't be mapped to any known `PanicReason`.
    #[num_enum(default)]
    UnknownPanicReason,
    /// Found `RVRT` instruction.
    Revert,
    /// Execution ran out of gas.
    OutOfGas,
    /// The transaction validity is violated.
    TransactionValidity,
    /// Attempt to write outside interpreter memory boundaries.
    MemoryOverflow,
    /// The requested memory access exceeds the limits of the interpreter.
    MaxMemoryAccess,
    /// Two segments of the interpreter memory should not intersect for write operations.
    MemoryWriteOverlap,
    /// Memory ownership rules are violated.
    MemoryOwnership,
    /// Designed contract was not found in the storage.
    ContractNotFound,
    /// The asset ID balance isn't enough for the instruction.
    NotEnoughBalance,
    /// The interpreter is expected to be in internal context.
    ExpectedInternalContext,
    /// The queried asset ID was not found in the state.
    AssetIdNotFound,
    /// The provided input is not found in the transaction.
    InputNotFound,
    /// The provided output is not found in the transaction.
    OutputNotFound,
    /// The provided witness is not found in the transaction.
    WitnessNotFound,
    /// The transaction maturity is not valid for this request.
    TransactionMaturity,
    /// The metadata identifier is invalid.
    InvalidMetadataIdentifier,
    /// The call structure is not valid.
    MalformedCallStructure,
    /// The provided register does not allow write operations.
    ReservedRegisterNotWritable,
    /// The execution resulted in an erroneous state of the interpreter.
    ErrorFlag,
    /// The provided immediate value is not valid for this instruction.
    InvalidImmediateValue,
    /// The requested contract is not listed in the transaction inputs.
    ContractNotInInputs,
    /// The internal asset ID balance overflowed with the provided instruction.
    InternalBalanceOverflow,
    /// The maximum allowed contract size is violated.
    ContractMaxSize,
    /// This instruction expects the stack area to be unallocated for this call.
    ExpectedUnallocatedStack,
    /// The provided transaction output should be of type `Variable`.
    ExpectedOutputVariable,
    /// The contract ID is already deployed and can't be overwritten.
    ContractIdAlreadyDeployed,
    /// The loaded contract mismatch expectations.
    ContractMismatch,
    /// Attempting to send message data longer than `MAX_MESSAGE_DATA_LENGTH`
    MessageDataTooLong,
    /// Overflow while executing arithmetic operation.
    /// These errors are ignored using the WRAPPING flag.
    ArithmeticOverflow,
    /// Mathimatically invalid arguments where given to an arithmetic instruction.
    /// For instance, division by zero produces this.
    /// These errors are ignored using the UNSAFEMATH flag.
    ArithmeticError,
    /// The contract instruction is not allowed in predicates.
    ContractInstructionNotAllowed,
}

impl fmt::Display for PanicReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for PanicReason {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

#[cfg(feature = "std")]
impl From<PanicReason> for std::io::Error {
    fn from(reason: PanicReason) -> Self {
        use std::io;

        io::Error::new(io::ErrorKind::Other, reason)
    }
}

impl From<core::array::TryFromSliceError> for PanicReason {
    fn from(_: core::array::TryFromSliceError) -> Self {
        Self::MemoryOverflow
    }
}
