use core::{convert, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[repr(u8)]
#[non_exhaustive]
/// Panic reason representation for the interpreter.
pub enum PanicReason {
    /// 0 is reserved for success, while any non-zero value indicates a failure.
    Success = 0x00,
    /// Found `RVRT` instruction.
    Revert = 0x01,
    /// Execution ran out of gas.
    OutOfGas = 0x02,
    /// The transaction validity is violated.
    TransactionValidity = 0x03,
    /// Attempt to write outside interpreter memory boundaries.
    MemoryOverflow = 0x04,
    /// Overflow while executing arithmetic operation.
    ArithmeticOverflow = 0x05,
    /// Designed contract was not found in the storage.
    ContractNotFound = 0x06,
    /// Memory ownership rules are violated.
    MemoryOwnership = 0x07,
    /// The asset ID balance isn't enough for the instruction.
    NotEnoughBalance = 0x08,
    /// The interpreter is expected to be in internal context.
    ExpectedInternalContext = 0x09,
    /// The queried asset ID was not found in the state.
    AssetIdNotFound = 0x0a,
    /// The provided input is not found in the transaction.
    InputNotFound = 0x0b,
    /// The provided output is not found in the transaction.
    OutputNotFound = 0x0c,
    /// The provided witness is not found in the transaction.
    WitnessNotFound = 0x0d,
    /// The transaction maturity is not valid for this request.
    TransactionMaturity = 0x0e,
    /// The metadata identifier is invalid.
    InvalidMetadataIdentifier = 0x0f,
    /// The call structure is not valid.
    MalformedCallStructure = 0x10,
    /// The provided register does not allow write operations.
    ReservedRegisterNotWritable = 0x11,
    /// The execution resulted in an erroneous state of the interpreter.
    ErrorFlag = 0x12,
    /// The provided immediate value is not valid for this instruction.
    InvalidImmediateValue = 0x13,
    /// The provided transaction input is not of type `Coin`.
    ExpectedCoinInput = 0x14,
    /// The requested memory access exceeds the limits of the interpreter.
    MaxMemoryAccess = 0x15,
    /// Two segments of the interpreter memory should not intersect for write operations.
    MemoryWriteOverlap = 0x16,
    /// The requested contract is not listed in the transaction inputs.
    ContractNotInInputs = 0x17,
    /// The internal asset ID balance overflowed with the provided instruction.
    InternalBalanceOverflow = 0x18,
    /// The maximum allowed contract size is violated.
    ContractMaxSize = 0x19,
    /// This instruction expects the stack area to be unallocated for this call.
    ExpectedUnallocatedStack = 0x1a,
    /// The maximum allowed number of static contracts was reached for this transaction.
    MaxStaticContractsReached = 0x1b,
    /// The requested transfer amount cannot be zero.
    TransferAmountCannotBeZero = 0x1c,
    /// The provided transaction output should be of type `Variable`.
    ExpectedOutputVariable = 0x1d,
    /// The expected context of the stack parent is internal.
    ExpectedParentInternalContext = 0x1e,
    /// The jump instruction cannot move backwards in predicate verification.
    IllegalJump = 0x1f,
    /// The contract ID is already deployed and can't be overwritten.
    ContractIdAlreadyDeployed = 0x20,
    /// The loaded contract mismatch expectations.
    ContractMismatch = 0x21,
    /// No more nested calls are allowed.
    NestedCallLimitReached = 0x22,
    /// The byte can't be mapped to any known `PanicReason`.
    UnknownPanicReason = 0x23,
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

// TODO: Remove this - `Infallible` has nothing to do with `PanicReason`.
impl From<convert::Infallible> for PanicReason {
    fn from(_i: convert::Infallible) -> Self {
        unreachable!()
    }
}

impl From<u8> for PanicReason {
    fn from(b: u8) -> Self {
        use PanicReason::*;
        match b {
            0x00 => Success,
            0x01 => Revert,
            0x02 => OutOfGas,
            0x03 => TransactionValidity,
            0x04 => MemoryOverflow,
            0x05 => ArithmeticOverflow,
            0x06 => ContractNotFound,
            0x07 => MemoryOwnership,
            0x08 => NotEnoughBalance,
            0x09 => ExpectedInternalContext,
            0x0a => AssetIdNotFound,
            0x0b => InputNotFound,
            0x0c => OutputNotFound,
            0x0d => WitnessNotFound,
            0x0e => TransactionMaturity,
            0x0f => InvalidMetadataIdentifier,
            0x10 => MalformedCallStructure,
            0x11 => ReservedRegisterNotWritable,
            0x12 => ErrorFlag,
            0x13 => InvalidImmediateValue,
            0x14 => ExpectedCoinInput,
            0x15 => MaxMemoryAccess,
            0x16 => MemoryWriteOverlap,
            0x17 => ContractNotInInputs,
            0x18 => InternalBalanceOverflow,
            0x19 => ContractMaxSize,
            0x1a => ExpectedUnallocatedStack,
            0x1b => MaxStaticContractsReached,
            0x1c => TransferAmountCannotBeZero,
            0x1d => ExpectedOutputVariable,
            0x1e => ExpectedParentInternalContext,
            0x1f => IllegalJump,
            0x20 => ContractIdAlreadyDeployed,
            0x21 => ContractMismatch,
            0x22 => NestedCallLimitReached,
            _ => UnknownPanicReason,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u8_panic_reason_round_trip() {
        const LAST_PANIC_REASON: u8 = 0x23;
        for i in 0..LAST_PANIC_REASON {
            let reason = PanicReason::from(i);
            let i2 = reason as u8;
            assert_eq!(i, i2);
        }
        for i in LAST_PANIC_REASON..=255 {
            let reason = PanicReason::from(i);
            let i2 = reason as u8;
            assert_eq!(PanicReason::UnknownPanicReason as u8, i2);
        }
    }
}
