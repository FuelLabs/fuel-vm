use core::fmt;

macro_rules! enum_from {
    (
        $(#[$meta:meta])* $vis:vis enum $name:ident {
            $($(#[$vmeta:meta])* $vname:ident $(= $val:expr)?,)*
        }
    ) => {
        $(#[$meta])*
        $vis enum $name {
            $($(#[$vmeta])* $vname $(= $val)?,)*
        }

        impl From<u8> for $name {
            fn from(v: u8) -> Self {
                match v {
                    $(x if x == $name::$vname as u8 => $name::$vname,)*
                    _ => $name::UnknownPanicReason,
                }
            }
        }
    }
}

enum_from! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumIter)]
    #[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(fuel_types::canonical::Serialize, fuel_types::canonical::Deserialize)]
    #[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
    #[repr(u8)]
    #[non_exhaustive]
    /// Panic reason representation for the interpreter.
    pub enum PanicReason {
        /// The byte can't be mapped to any known `PanicReason`.
        UnknownPanicReason = 0x00,
        /// Found `RVRT` instruction.
        Revert = 0x01,
        /// Execution ran out of gas.
        OutOfGas = 0x02,
        /// The transaction validity is violated.
        TransactionValidity = 0x03,
        /// Attempt to write outside interpreter memory boundaries.
        MemoryOverflow = 0x04,
        /// Overflow while executing arithmetic operation.
        /// These errors are ignored using the WRAPPING flag.
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
        InvalidFlags = 0x12,
        /// The provided immediate value is not valid for this instruction.
        InvalidImmediateValue = 0x13,
        /// The provided transaction input is not of type `Coin`.
        ExpectedCoinInput = 0x14,
        /// `ECAL` instruction failed.
        EcalError = 0x15,
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
        /// Attempting to send message data longer than `MAX_MESSAGE_DATA_LENGTH`
        MessageDataTooLong = 0x22,
        /// Mathimatically invalid arguments where given to an arithmetic instruction.
        /// For instance, division by zero produces this.
        /// These errors are ignored using the UNSAFEMATH flag.
        ArithmeticError = 0x23,
        /// The contract instruction is not allowed in predicates.
        ContractInstructionNotAllowed = 0x24,
        /// Transfer of zero coins is not allowed.
        TransferZeroCoins = 0x25,
        /// Attempted to execute an invalid instruction
        InvalidInstruction = 0x26,
        /// Memory outside $is..$ssp range is not executable
        MemoryNotExecutable = 0x27,
        /// The policy is not set.
        PolicyIsNotSet = 0x28,
        /// The policy is not found across policies.
        PolicyNotFound = 0x29,
        /// Receipt context is full
        TooManyReceipts = 0x2a,
    }
}

impl fmt::Display for PanicReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
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
    use strum::IntoEnumIterator;

    #[test]
    fn test_u8_panic_reason_round_trip() {
        let last_known_panic_reason: u8 = PanicReason::iter().last().unwrap() as u8 + 1;
        let reason = PanicReason::from(0);
        assert_eq!(reason, PanicReason::UnknownPanicReason);

        for i in 1..last_known_panic_reason {
            let reason = PanicReason::try_from(i).unwrap();
            let i2 = reason as u8;
            assert_eq!(i, i2);
        }
        for i in last_known_panic_reason..=255 {
            let reason = PanicReason::from(i);
            let i2 = reason as u8;
            assert_eq!(PanicReason::UnknownPanicReason as u8, i2);
        }
    }
}
