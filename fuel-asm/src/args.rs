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
    #[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
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
    #[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
    pub enum GTFArgs {
        /// Set `$rA` to `tx.type`
        Type = 0x001,

        /// Set `$rA` to `tx.gasPrice`
        ScriptGasPrice = 0x002,

        /// Set `$rA` to `tx.gasLimit`
        ScriptGasLimit = 0x003,

        /// Set `$rA` to `tx.maturity`
        ScriptMaturity = 0x004,

        /// Set `$rA` to `tx.scriptLength`
        ScriptLength = 0x005,

        /// Set `$rA` to `tx.scriptDataLength`
        ScriptDataLength = 0x006,

        /// Set `$rA` to `tx.inputsCount`
        ScriptInputsCount = 0x007,

        /// Set `$rA` to `tx.outputsCount`
        ScriptOutputsCount = 0x008,

        /// Set `$rA` to `tx.witnessesCount`
        ScriptWitnessesCound = 0x009,

        /// Set `$rA` to `Memory address of tx.receiptsRoot`
        ScriptReceiptsRoot = 0x00A,

        /// Set `$rA` to `Memory address of tx.script`
        Script = 0x00B,

        /// Set `$rA` to `Memory address of tx.scriptData`
        ScriptData = 0x00C,

        /// Set `$rA` to `Memory address of tx.inputs[$rB]`
        ScriptInputAtIndex = 0x00D,

        /// Set `$rA` to `Memory address of t.outputs[$rB]`
        ScriptOutputAtIndex = 0x00E,

        /// Set `$rA` to `Memory address of tx.witnesses[$rB]`
        ScriptWitnessAtIndex = 0x00F,

        /// Set `$rA` to `tx.gasPrice`
        CreateGasPrice = 0x010,

        /// Set `$rA` to `tx.gasLimit`
        CreateGasLimit = 0x011,

        /// Set `$rA` to `tx.maturity`
        CreateMaturity = 0x012,

        /// Set `$rA` to `tx.bytecodeLength`
        CreateBytecodeLength = 0x013,

        /// Set `$rA` to `tx.bytecodeWitnessIndex`
        CreateBytecodeWitnessIndex = 0x014,

        /// Set `$rA` to `tx.storageSlotsCount`
        CreateStorageSlotsCount = 0x015,

        /// Set `$rA` to `tx.inputsCount`
        CreateInputsCount = 0x016,

        /// Set `$rA` to `tx.outputsCount`
        CreateOutputsCount = 0x017,

        /// Set `$rA` to `tx.witnessesCount`
        CreateWitnessesCount = 0x018,

        /// Set `$rA` to `Memory address of tx.salt`
        CreateSalt = 0x019,

        /// Set `$rA` to `Memory address of tx.storageSlots[$rB]`
        CreateStorageSlotAtIndex = 0x01A,

        /// Set `$rA` to `Memory address of tx.inputs[$rB]`
        CreateInputAtIndex = 0x01B,

        /// Set `$rA` to `Memory address of t.outputs[$rB]`
        CreateOutputAtIndex = 0x01C,

        /// Set `$rA` to `Memory address of tx.witnesses[$rB]`
        CreateWitnessAtIndex = 0x01D,

        /// Set `$rA` to `tx.inputs[$rB].type`
        InputType = 0x101,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].txID`
        InputCoinTxId = 0x102,

        /// Set `$rA` to `tx.inputs[$rB].outputIndex`
        InputCoinOutputIndex = 0x103,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].owner`
        InputCoinOwner = 0x104,

        /// Set `$rA` to `tx.inputs[$rB].amount`
        InputCoinAmount = 0x105,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].asset_id`
        InputCoinAssetId = 0x106,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].txPointer`
        InputCoinTxPointer = 0x107,

        /// Set `$rA` to `tx.inputs[$rB].witnessIndex`
        InputCoinWitnessIndex = 0x108,

        /// Set `$rA` to `tx.inputs[$rB].maturity`
        InputCoinMaturity = 0x109,

        /// Set `$rA` to `tx.inputs[$rB].predicateLength`
        InputCoinPredicateLength = 0x10A,

        /// Set `$rA` to `tx.inputs[$rB].predicateDataLength`
        InputCoinPredicateDataLength = 0x10B,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].predicate`
        InputCoinPredicate = 0x10C,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].predicateData`
        InputCoinPredicateData = 0x10D,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].predicateGasUsed`
        InputCoinPredicateGasUsed = 0x10E,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].txID`
        InputContractTxId = 0x10F,

        /// Set `$rA` to `tx.inputs[$rB].outputIndex`
        InputContractOutputIndex = 0x110,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].balanceRoot`
        InputContractBalanceRoot = 0x111,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].stateRoot`
        InputContractStateRoot = 0x112,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].txPointer`
        InputContractTxPointer = 0x113,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].contractID`
        InputContractId = 0x114,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].sender`
        InputMessageSender = 0x115,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].recipient`
        InputMessageRecipient = 0x116,

        /// Set `$rA` to `tx.inputs[$rB].amount`
        InputMessageAmount = 0x117,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].nonce`
        InputMessageNonce = 0x118,

        /// Set `$rA` to `tx.inputs[$rB].witnessIndex`
        InputMessageWitnessIndex = 0x119,

        /// Set `$rA` to `tx.inputs[$rB].dataLength`
        InputMessageDataLength = 0x11A,

        /// Set `$rA` to `tx.inputs[$rB].predicateLength`
        InputMessagePredicateLength = 0x11B,

        /// Set `$rA` to `tx.inputs[$rB].predicateDataLength`
        InputMessagePredicateDataLength = 0x11C,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].data`
        InputMessageData = 0x11D,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].predicate`
        InputMessagePredicate = 0x11E,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].predicateData`
        InputMessagePredicateData = 0x11F,

        /// Set `$rA` to `Memory address of tx.inputs[$rB].predicateGasUsed`
        InputMessagePredicateGasUsed = 0x120,

        /// Set `$rA` to `tx.outputs[$rB].type`
        OutputType = 0x201,

        /// Set `$rA` to `Memory address of tx.outputs[$rB].to`
        OutputCoinTo = 0x202,

        /// Set `$rA` to `tx.outputs[$rB].amount`
        OutputCoinAmount = 0x203,

        /// Set `$rA` to `Memory address of tx.outputs[$rB].asset_id`
        OutputCoinAssetId = 0x204,

        /// Set `$rA` to `tx.outputs[$rB].inputIndex`
        OutputContractInputIndex = 0x205,

        /// Set `$rA` to `Memory address of tx.outputs[$rB].balanceRoot`
        OutputContractBalanceRoot = 0x206,

        /// Set `$rA` to `Memory address of tx.outputs[$rB].stateRoot`
        OutputContractStateRoot = 0x207,

        /// Set `$rA` to `Memory address of tx.outputs[$rB].contractID`
        OutputContractCreatedContractId = 0x208,

        /// Set `$rA` to `Memory address of tx.outputs[$rB].stateRoot`
        OutputContractCreatedStateRoot = 0x209,

        /// Set `$rA` to `tx.witnesses[$rB].dataLength`
        WitnessDataLength = 0x301,

        /// Set `$rA` to `Memory address of tx.witnesses[$rB].data`
        WitnessData = 0x302,
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
        GTFArgs::ScriptGasPrice,
        GTFArgs::ScriptGasLimit,
        GTFArgs::ScriptMaturity,
        GTFArgs::ScriptLength,
        GTFArgs::ScriptDataLength,
        GTFArgs::ScriptInputsCount,
        GTFArgs::ScriptOutputsCount,
        GTFArgs::ScriptWitnessesCound,
        GTFArgs::ScriptReceiptsRoot,
        GTFArgs::Script,
        GTFArgs::ScriptData,
        GTFArgs::ScriptInputAtIndex,
        GTFArgs::ScriptOutputAtIndex,
        GTFArgs::ScriptWitnessAtIndex,
        GTFArgs::CreateGasPrice,
        GTFArgs::CreateGasLimit,
        GTFArgs::CreateMaturity,
        GTFArgs::CreateBytecodeLength,
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
        GTFArgs::InputCoinMaturity,
        GTFArgs::InputCoinPredicateLength,
        GTFArgs::InputCoinPredicateDataLength,
        GTFArgs::InputCoinPredicate,
        GTFArgs::InputCoinPredicateData,
        GTFArgs::InputCoinPredicateGasUsed,
        GTFArgs::InputContractTxId,
        GTFArgs::InputContractOutputIndex,
        GTFArgs::InputContractBalanceRoot,
        GTFArgs::InputContractStateRoot,
        GTFArgs::InputContractTxPointer,
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
        GTFArgs::OutputContractBalanceRoot,
        GTFArgs::OutputContractStateRoot,
        GTFArgs::OutputContractCreatedContractId,
        GTFArgs::OutputContractCreatedStateRoot,
        GTFArgs::WitnessDataLength,
        GTFArgs::WitnessData,
    ];

    args.into_iter().for_each(|a| {
        let imm = a as Immediate12;
        let a_p = GTFArgs::try_from(imm).expect("failed to convert GTFArgs");

        assert_eq!(a, a_p);
    });
}
