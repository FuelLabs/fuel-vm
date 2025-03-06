#![allow(deprecated)]

pub mod narrowint;
pub mod wideint;

/// 12-bits immediate value type
type Immediate12 = u16;

/// 18-bits immediate value type
type Immediate18 = u32;

crate::enum_try_from! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumIter)]
    #[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
    #[repr(u8)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    /// Argument list for GM (get metadata) instruction
    /// The VM is the only who should match this struct, and it *MUST* always perform
    /// exhaustive match so all offered variants are covered.
    pub enum GMArgs {
        /// Get if caller is external.
        IsCallerExternal = 0x01,

        /// Get caller's contract ID.
        GetCaller = 0x02,

        /// Get index of current predicate.
        GetVerifyingPredicate = 0x03,

        /// Get the Chain ID this VM is operating within
        GetChainId = 0x04,

        /// Get memory address where the transaction is located
        TxStart = 0x05,

        /// Get memory address of base asset ID
        BaseAssetId = 0x06,
    },
    Immediate18
}

impl From<GMArgs> for Immediate18 {
    fn from(args: GMArgs) -> Self {
        args as Immediate18
    }
}

crate::enum_try_from! {
    /// Argument list for GTF (get tx fields) instruction
    /// The VM is the only who should match this struct, and it *MUST* always perform
    /// exhaustive match so all offered variants are covered.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumIter)]
    #[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
    #[repr(u16)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub enum GTFArgs {
        /// Set `$rA` to `tx.type`
        Type = 0x001,

        /// Set `$rA` to `tx.scriptGasLimit`
        ScriptGasLimit = 0x002,

        /// Set `$rA` to `tx.scriptLength`
        ScriptLength = 0x003,

        /// Set `$rA` to `tx.scriptDataLength`
        ScriptDataLength = 0x004,

        /// Set `$rA` to `tx.inputsCount`
        #[deprecated(since = "0.60.0", note = "Use the generic `TxInputsCount` instead")]
        ScriptInputsCount = 0x005,

        /// Set `$rA` to `tx.outputsCount`
        #[deprecated(since = "0.60.0", note = "Use the generic `TxOutputsCount` instead")]
        ScriptOutputsCount = 0x006,

        /// Set `$rA` to `tx.witnessesCount`
        #[deprecated(since = "0.60.0", note = "Use the generic `TxWitnessesCount` instead")]
        ScriptWitnessesCount = 0x007,

        /// Set `$rA` to `Memory address of tx.script`
        Script = 0x009,

        /// Set `$rA` to `Memory address of tx.scriptData`
        ScriptData = 0x00A,

        /// Set `$rA` to `Memory address of tx.inputs[$rB]`
        #[deprecated(since = "0.60.0", note = "Use the generic `TxInputAtIndex` instead")]
        ScriptInputAtIndex = 0x00B,

        /// Set `$rA` to `Memory address of t.outputs[$rB]`
        #[deprecated(since = "0.60.0", note = "Use the generic `TxOutputAtIndex` instead")]
        ScriptOutputAtIndex = 0x00C,

        /// Set `$rA` to `Memory address of tx.witnesses[$rB]`
        #[deprecated(since = "0.60.0", note = "Use the generic `TxWitnessAtIndex` instead")]
        ScriptWitnessAtIndex = 0x00D,

        /// Set `$rA` to size of the transaction in memory, in bytes
        TxLength = 0x00E,

        /// Set `$rA` to `tx.bytecodeWitnessIndex`
        CreateBytecodeWitnessIndex = 0x101,

        /// Set `$rA` to `tx.storageSlotsCount`
        CreateStorageSlotsCount = 0x102,

        /// Set `$rA` to `tx.inputsCount`
        #[deprecated(since = "0.60.0", note = "Use the generic `TxInputsCount` instead")]
        CreateInputsCount = 0x103,

        /// Set `$rA` to `tx.outputsCount`
        #[deprecated(since = "0.60.0", note = "Use the generic `TxOutputsCount` instead")]
        CreateOutputsCount = 0x104,

        /// Set `$rA` to `tx.witnessesCount`
        #[deprecated(since = "0.60.0", note = "Use the generic `TxWitnessesCount` instead")]
        CreateWitnessesCount = 0x105,

        /// Set `$rA` to `Memory address of tx.salt`
        CreateSalt = 0x106,

        /// Set `$rA` to `Memory address of tx.storageSlots[$rB]`
        CreateStorageSlotAtIndex = 0x107,

        /// Set `$rA` to `Memory address of tx.inputs[$rB]`
        #[deprecated(since = "0.60.0", note = "Use the generic `TxInputAtIndex` instead")]
        CreateInputAtIndex = 0x108,

        /// Set `$rA` to `Memory address of t.outputs[$rB]`
        #[deprecated(since = "0.60.0", note = "Use the generic `TxOutputAtIndex` instead")]
        CreateOutputAtIndex = 0x109,

        /// Set `$rA` to `Memory address of tx.witnesses[$rB]`
        #[deprecated(since = "0.60.0", note = "Use the generic `TxWitnessAtIndex` instead")]
        CreateWitnessAtIndex = 0x10A,

        /// Set `$rA` to `tx.inputs[$rB].type`
        InputType = 0x200,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].txID`
        InputCoinTxId = 0x201,

        /// Set `$rA` to `tx.inputs[$rB].outputIndex`
        InputCoinOutputIndex = 0x202,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].owner`
        InputCoinOwner = 0x203,

        /// Set `$rA` to `tx.inputs[$rB].amount`
        InputCoinAmount = 0x204,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].asset_id`
        InputCoinAssetId = 0x205,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].txPointer`
        InputCoinTxPointer = 0x206,

        /// Set `$rA` to `tx.inputs[$rB].witnessIndex`
        InputCoinWitnessIndex = 0x207,

        /// Set `$rA` to `tx.inputs[$rB].predicateLength`
        InputCoinPredicateLength = 0x209,

        /// Set `$rA` to `tx.inputs[$rB].predicateDataLength`
        InputCoinPredicateDataLength = 0x20A,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].predicate`
        InputCoinPredicate = 0x20B,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].predicateData`
        InputCoinPredicateData = 0x20C,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].predicateGasUsed`
        InputCoinPredicateGasUsed = 0x20D,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].txID`
        InputContractTxId = 0x220,

        /// Set `$rA` to `tx.inputs[$rB].outputIndex`
        InputContractOutputIndex = 0x221,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].contractID`
        InputContractId = 0x225,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].sender`
        InputMessageSender = 0x240,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].recipient`
        InputMessageRecipient = 0x241,

        /// Set `$rA` to `tx.inputs[$rB].amount`
        InputMessageAmount = 0x242,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].nonce`
        InputMessageNonce = 0x243,

        /// Set `$rA` to `tx.inputs[$rB].witnessIndex`
        InputMessageWitnessIndex = 0x244,

        /// Set `$rA` to `tx.inputs[$rB].dataLength`
        InputMessageDataLength = 0x245,

        /// Set `$rA` to `tx.inputs[$rB].predicateLength`
        InputMessagePredicateLength = 0x246,

        /// Set `$rA` to `tx.inputs[$rB].predicateDataLength`
        InputMessagePredicateDataLength = 0x247,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].data`
        InputMessageData = 0x248,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].predicate`
        InputMessagePredicate = 0x249,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].predicateData`
        InputMessagePredicateData = 0x24A,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].predicateGasUsed`
        InputMessagePredicateGasUsed = 0x24B,

        /// Set `$rA` to `tx.outputs[$rB].type`
        OutputType = 0x300,

        /// Set `$rA` to `Memory address of tx.outputs[$rB].to`
        OutputCoinTo = 0x301,

        /// Set `$rA` to `tx.outputs[$rB].amount`
        OutputCoinAmount = 0x302,

        /// Set `$rA` to `Memory address of tx.outputs[$rB].asset_id`
        OutputCoinAssetId = 0x303,

        /// Set `$rA` to `tx.outputs[$rB].inputIndex`
        OutputContractInputIndex = 0x304,

        /// Set `$rA` to `Memory address of tx.outputs[$rB].contractID`
        OutputContractCreatedContractId = 0x307,

        /// Set `$rA` to `Memory address of tx.outputs[$rB].stateRoot`
        OutputContractCreatedStateRoot = 0x308,

        /// Set `$rA` to `tx.witnesses[$rB].dataLength`
        WitnessDataLength = 0x400,

        /// Set `$rA` to `Memory address of tx.witnesses[$rB].data`
        WitnessData = 0x401,

        /// Set `$rA` to `tx.policyTypes`
        PolicyTypes = 0x500,

        /// Set `$rA` to `tx.policies[0x00].gasPrice`
        PolicyTip = 0x501,

        /// Set `$rA` to `tx.policies[count_ones(0b11 & tx.policyTypes) - 1].witnessLimit`
        PolicyWitnessLimit = 0x502,

        /// Set `$rA` to `tx.policies[count_ones(0b111 & tx.policyTypes) - 1].maturity`
        PolicyMaturity = 0x503,

        /// Set `$rA` to `tx.policies[count_ones(0b1111 & tx.policyTypes) - 1].maxFee`
        PolicyMaxFee = 0x504,

        /// Set `$rA` to `tx.policies[count_ones(0b11111 & tx.policyTypes) - 1].expiration`
        PolicyExpiration = 0x505,

        /// Set `$rA` to `Memory address of tx.root`
        UploadRoot = 0x600,

        /// Set `$rA` to `tx.witnessIndex`
        UploadWitnessIndex = 0x601,

        /// Set `$rA` to `tx.subsectionIndex`
        UploadSubsectionIndex = 0x602,

        /// Set `$rA` to `tx.subsectionsNumber`
        UploadSubsectionsCount = 0x603,

        /// Set `$rA` to `tx.proofSetCount`
        UploadProofSetCount = 0x604,

        /// Set `$rA` to `Memory address of tx.proofSet[$rB]`
        UploadProofSetAtIndex = 0x605,

        /// Set `$rA` to `Memory address of tx.id`
        BlobId = 0x700,

        /// Set `$rA` to `tx.witnessIndex`
        BlobWitnessIndex = 0x701,

        /// Set `$rA` to `Memory address of tx.purpose`
        UpgradePurpose = 0x800,

        /// Set `$rA` to `tx.inputsCount`
        TxInputsCount = 0x900,

        /// Set `$rA` to `tx.outputsCount`
        TxOutputsCount = 0x901,

        /// Set `$rA` to `tx.witnessesCount`
        TxWitnessesCount = 0x902,

        /// Set `$rA` to `Memory address of tx.inputs[$rB]`
        TxInputAtIndex = 0x903,

        /// Set `$rA` to `Memory address of t.outputs[$rB]`
        TxOutputAtIndex = 0x904,

        /// Set `$rA` to `Memory address of tx.witnesses[$rB]`
        TxWitnessAtIndex = 0x905,
    },
    Immediate12
}

impl From<GTFArgs> for Immediate12 {
    fn from(args: GTFArgs) -> Self {
        args as Immediate12
    }
}

#[test]
#[cfg(feature = "std")]
fn encode_gm_args() {
    let args = vec![
        GMArgs::IsCallerExternal,
        GMArgs::GetCaller,
        GMArgs::GetVerifyingPredicate,
        GMArgs::GetChainId,
        GMArgs::TxStart,
        GMArgs::BaseAssetId,
    ];

    args.into_iter().for_each(|a| {
        let imm = a as Immediate18;
        let a_p = GMArgs::try_from(imm).expect("failed to convert GMArgs");

        assert_eq!(a, a_p);
    });
}

#[test]
#[cfg(feature = "std")]
fn encode_gtf_args() {
    let args = vec![
        GTFArgs::Type,
        GTFArgs::ScriptGasLimit,
        GTFArgs::ScriptLength,
        GTFArgs::ScriptDataLength,
        GTFArgs::ScriptInputsCount,
        GTFArgs::ScriptOutputsCount,
        GTFArgs::ScriptWitnessesCount,
        GTFArgs::Script,
        GTFArgs::ScriptData,
        GTFArgs::ScriptInputAtIndex,
        GTFArgs::ScriptOutputAtIndex,
        GTFArgs::ScriptWitnessAtIndex,
        GTFArgs::CreateBytecodeWitnessIndex,
        GTFArgs::CreateStorageSlotsCount,
        GTFArgs::CreateInputsCount,
        GTFArgs::CreateOutputsCount,
        GTFArgs::CreateWitnessesCount,
        GTFArgs::CreateSalt,
        GTFArgs::CreateStorageSlotAtIndex,
        GTFArgs::CreateInputAtIndex,
        GTFArgs::CreateOutputAtIndex,
        GTFArgs::CreateWitnessAtIndex,
        GTFArgs::InputType,
        GTFArgs::InputCoinTxId,
        GTFArgs::InputCoinOutputIndex,
        GTFArgs::InputCoinOwner,
        GTFArgs::InputCoinAmount,
        GTFArgs::InputCoinAssetId,
        GTFArgs::InputCoinTxPointer,
        GTFArgs::InputCoinWitnessIndex,
        GTFArgs::InputCoinPredicateLength,
        GTFArgs::InputCoinPredicateDataLength,
        GTFArgs::InputCoinPredicate,
        GTFArgs::InputCoinPredicateData,
        GTFArgs::InputCoinPredicateGasUsed,
        GTFArgs::InputContractId,
        GTFArgs::InputMessageSender,
        GTFArgs::InputMessageRecipient,
        GTFArgs::InputMessageAmount,
        GTFArgs::InputMessageNonce,
        GTFArgs::InputMessageWitnessIndex,
        GTFArgs::InputMessageDataLength,
        GTFArgs::InputMessagePredicateLength,
        GTFArgs::InputMessagePredicateDataLength,
        GTFArgs::InputMessageData,
        GTFArgs::InputMessagePredicate,
        GTFArgs::InputMessagePredicateData,
        GTFArgs::InputMessagePredicateGasUsed,
        GTFArgs::OutputType,
        GTFArgs::OutputCoinTo,
        GTFArgs::OutputCoinAmount,
        GTFArgs::OutputCoinAssetId,
        GTFArgs::OutputContractInputIndex,
        GTFArgs::OutputContractCreatedContractId,
        GTFArgs::OutputContractCreatedStateRoot,
        GTFArgs::WitnessDataLength,
        GTFArgs::WitnessData,
        GTFArgs::PolicyTypes,
        GTFArgs::PolicyTip,
        GTFArgs::PolicyWitnessLimit,
        GTFArgs::PolicyMaturity,
        GTFArgs::PolicyExpiration,
        GTFArgs::PolicyMaxFee,
        GTFArgs::UploadRoot,
        GTFArgs::UploadWitnessIndex,
        GTFArgs::UploadSubsectionIndex,
        GTFArgs::UploadSubsectionsCount,
        GTFArgs::UploadProofSetCount,
        GTFArgs::UploadProofSetAtIndex,
        GTFArgs::BlobId,
        GTFArgs::BlobWitnessIndex,
        GTFArgs::UpgradePurpose,
        GTFArgs::TxInputsCount,
        GTFArgs::TxOutputsCount,
        GTFArgs::TxWitnessesCount,
        GTFArgs::TxInputAtIndex,
        GTFArgs::TxOutputAtIndex,
        GTFArgs::TxWitnessAtIndex,
    ];

    args.into_iter().for_each(|a| {
        let imm = a as Immediate12;
        let a_p = GTFArgs::try_from(imm).expect("failed to convert GTFArgs");

        assert_eq!(a, a_p);
    });
}
