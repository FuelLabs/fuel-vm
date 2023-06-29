use crate::PanicReason;

use fuel_types::{
    Immediate12,
    Immediate18,
};

pub mod wideint;

const GM_IS_CALLER_EXTERNAL: u8 = 0x01;
const GM_GET_CALLER: u8 = 0x02;
const GM_GET_VERIFYING_PREDICATE: u8 = 0x03;
const GM_GET_CHAIN_ID: u8 = 0x04;

/// Argument list for GM (get metadata) instruction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumIter)]
#[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
#[repr(u8)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
// The VM is the only who should match this struct, and it *MUST* always perform
// exhaustive match so all offered variants are covered.
pub enum GMArgs {
    /// Get if caller is external.
    IsCallerExternal = 0x01,

    /// Get caller's contract ID.
    GetCaller = 0x02,

    /// Get index of current predicate.
    GetVerifyingPredicate = 0x03,

    /// Get the Chain ID this VM is operating within
    GetChainId = 0x04,
}

impl TryFrom<Immediate18> for GMArgs {
    type Error = PanicReason;

    fn try_from(value: Immediate18) -> Result<Self, Self::Error> {
        match value as u8 {
            GM_IS_CALLER_EXTERNAL => Ok(Self::IsCallerExternal),
            GM_GET_CALLER => Ok(Self::GetCaller),
            GM_GET_VERIFYING_PREDICATE => Ok(Self::GetVerifyingPredicate),
            GM_GET_CHAIN_ID => Ok(Self::GetChainId),
            _ => Err(PanicReason::InvalidMetadataIdentifier),
        }
    }
}

impl From<GMArgs> for Immediate18 {
    fn from(args: GMArgs) -> Self {
        args as Immediate18
    }
}

const GTF_TYPE: u16 = 0x001;
const GTF_SCRIPT_GAS_PRICE: u16 = 0x002;
const GTF_SCRIPT_GAS_LIMIT: u16 = 0x003;
const GTF_SCRIPT_MATURITY: u16 = 0x004;
const GTF_SCRIPT_LENGTH: u16 = 0x005;
const GTF_SCRIPT_DATA_LENGTH: u16 = 0x006;
const GTF_SCRIPT_INPUTS_COUNT: u16 = 0x007;
const GTF_SCRIPT_OUTPUTS_COUNT: u16 = 0x008;
const GTF_SCRIPT_WITNESSES_COUNT: u16 = 0x009;
const GTF_SCRIPT_RECEIPTS_ROOT: u16 = 0x00A;
const GTF_SCRIPT: u16 = 0x00B;
const GTF_SCRIPT_DATA: u16 = 0x00C;
const GTF_SCRIPT_INPUT_AT_INDEX: u16 = 0x00D;
const GTF_SCRIPT_OUTPUT_AT_INDEX: u16 = 0x00E;
const GTF_SCRIPT_WITNESS_AT_INDEX: u16 = 0x00F;
const GTF_CREATE_GAS_PRICE: u16 = 0x010;
const GTF_CREATE_GAS_LIMIT: u16 = 0x011;
const GTF_CREATE_MATURITY: u16 = 0x012;
const GTF_CREATE_BYTECODE_LENGTH: u16 = 0x013;
const GTF_CREATE_BYTECODE_WITNESS_INDEX: u16 = 0x014;
const GTF_CREATE_STORAGE_SLOTS_COUNT: u16 = 0x015;
const GTF_CREATE_INPUTS_COUNT: u16 = 0x016;
const GTF_CREATE_OUTPUTS_COUNT: u16 = 0x017;
const GTF_CREATE_WITNESSES_COUNT: u16 = 0x018;
const GTF_CREATE_SALT: u16 = 0x019;
const GTF_CREATE_STORAGE_SLOT_AT_INDEX: u16 = 0x01A;
const GTF_CREATE_INPUT_AT_INDEX: u16 = 0x01B;
const GTF_CREATE_OUTPUT_AT_INDEX: u16 = 0x01C;
const GTF_CREATE_WITNESS_AT_INDEX: u16 = 0x01D;
const GTF_INPUT_TYPE: u16 = 0x101;
const GTF_INPUT_COIN_TX_ID: u16 = 0x102;
const GTF_INPUT_COIN_OUTPUT_INDEX: u16 = 0x103;
const GTF_INPUT_COIN_OWNER: u16 = 0x104;
const GTF_INPUT_COIN_AMOUNT: u16 = 0x105;
const GTF_INPUT_COIN_ASSET_ID: u16 = 0x106;
const GTF_INPUT_COIN_TX_POINTER: u16 = 0x107;
const GTF_INPUT_COIN_WITNESS_INDEX: u16 = 0x108;
const GTF_INPUT_COIN_MATURITY: u16 = 0x109;
const GTF_INPUT_COIN_PREDICATE_LENGTH: u16 = 0x10A;
const GTF_INPUT_COIN_PREDICATE_DATA_LENGTH: u16 = 0x10B;
const GTF_INPUT_COIN_PREDICATE: u16 = 0x10C;
const GTF_INPUT_COIN_PREDICATE_DATA: u16 = 0x10D;
const GTF_INPUT_COIN_PREDICATE_GAS_USED: u16 = 0x10E;
const GTF_INPUT_CONTRACT_TX_ID: u16 = 0x10F;
const GTF_INPUT_CONTRACT_OUTPUT_INDEX: u16 = 0x110;
const GTF_INPUT_CONTRACT_BALANCE_ROOT: u16 = 0x111;
const GTF_INPUT_CONTRACT_STATE_ROOT: u16 = 0x112;
const GTF_INPUT_CONTRACT_TX_POINTER: u16 = 0x113;
const GTF_INPUT_CONTRACT_ID: u16 = 0x114;
const GTF_INPUT_MESSAGE_SENDER: u16 = 0x115;
const GTF_INPUT_MESSAGE_RECIPIENT: u16 = 0x116;
const GTF_INPUT_MESSAGE_AMOUNT: u16 = 0x117;
const GTF_INPUT_MESSAGE_NONCE: u16 = 0x118;
const GTF_INPUT_MESSAGE_WITNESS_INDEX: u16 = 0x119;
const GTF_INPUT_MESSAGE_DATA_LENGTH: u16 = 0x11A;
const GTF_INPUT_MESSAGE_PREDICATE_LENGTH: u16 = 0x11B;
const GTF_INPUT_MESSAGE_PREDICATE_DATA_LENGTH: u16 = 0x11C;
const GTF_INPUT_MESSAGE_DATA: u16 = 0x11D;
const GTF_INPUT_MESSAGE_PREDICATE: u16 = 0x11E;
const GTF_INPUT_MESSAGE_PREDICATE_DATA: u16 = 0x11F;
const GTF_INPUT_MESSAGE_PREDICATE_GAS_USED: u16 = 0x120;
const GTF_OUTPUT_TYPE: u16 = 0x201;
const GTF_OUTPUT_COIN_TO: u16 = 0x202;
const GTF_OUTPUT_COIN_AMOUNT: u16 = 0x203;
const GTF_OUTPUT_COIN_ASSET_ID: u16 = 0x204;
const GTF_OUTPUT_CONTRACT_INPUT_INDEX: u16 = 0x205;
const GTF_OUTPUT_CONTRACT_BALANCE_ROOT: u16 = 0x206;
const GTF_OUTPUT_CONTRACT_STATE_ROOT: u16 = 0x207;
const GTF_OUTPUT_CONTRACT_CREATED_CONTRACT_ID: u16 = 0x208;
const GTF_OUTPUT_CONTRACT_CREATED_STATE_ROOT: u16 = 0x209;
const GTF_WITNESS_DATA_LENGTH: u16 = 0x301;
const GTF_WITNESS_DATA: u16 = 0x302;

/// Argument list for GTF (get tx fields) instruction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumIter)]
#[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
#[repr(u16)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
// The VM is the only who should match this struct, and it *MUST* always perform
// exhaustive match so all offered variants are covered.
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
}

impl TryFrom<Immediate12> for GTFArgs {
    type Error = PanicReason;

    fn try_from(value: Immediate12) -> Result<Self, Self::Error> {
        match value {
            GTF_TYPE => Ok(Self::Type),
            GTF_SCRIPT_GAS_PRICE => Ok(Self::ScriptGasPrice),
            GTF_SCRIPT_GAS_LIMIT => Ok(Self::ScriptGasLimit),
            GTF_SCRIPT_MATURITY => Ok(Self::ScriptMaturity),
            GTF_SCRIPT_LENGTH => Ok(Self::ScriptLength),
            GTF_SCRIPT_DATA_LENGTH => Ok(Self::ScriptDataLength),
            GTF_SCRIPT_INPUTS_COUNT => Ok(Self::ScriptInputsCount),
            GTF_SCRIPT_OUTPUTS_COUNT => Ok(Self::ScriptOutputsCount),
            GTF_SCRIPT_WITNESSES_COUNT => Ok(Self::ScriptWitnessesCound),
            GTF_SCRIPT_RECEIPTS_ROOT => Ok(Self::ScriptReceiptsRoot),
            GTF_SCRIPT => Ok(Self::Script),
            GTF_SCRIPT_DATA => Ok(Self::ScriptData),
            GTF_SCRIPT_INPUT_AT_INDEX => Ok(Self::ScriptInputAtIndex),
            GTF_SCRIPT_OUTPUT_AT_INDEX => Ok(Self::ScriptOutputAtIndex),
            GTF_SCRIPT_WITNESS_AT_INDEX => Ok(Self::ScriptWitnessAtIndex),
            GTF_CREATE_GAS_PRICE => Ok(Self::CreateGasPrice),
            GTF_CREATE_GAS_LIMIT => Ok(Self::CreateGasLimit),
            GTF_CREATE_MATURITY => Ok(Self::CreateMaturity),
            GTF_CREATE_BYTECODE_LENGTH => Ok(Self::CreateBytecodeLength),
            GTF_CREATE_BYTECODE_WITNESS_INDEX => Ok(Self::CreateBytecodeWitnessIndex),
            GTF_CREATE_STORAGE_SLOTS_COUNT => Ok(Self::CreateStorageSlotsCount),
            GTF_CREATE_INPUTS_COUNT => Ok(Self::CreateInputsCount),
            GTF_CREATE_OUTPUTS_COUNT => Ok(Self::CreateOutputsCount),
            GTF_CREATE_WITNESSES_COUNT => Ok(Self::CreateWitnessesCount),
            GTF_CREATE_SALT => Ok(Self::CreateSalt),
            GTF_CREATE_STORAGE_SLOT_AT_INDEX => Ok(Self::CreateStorageSlotAtIndex),
            GTF_CREATE_INPUT_AT_INDEX => Ok(Self::CreateInputAtIndex),
            GTF_CREATE_OUTPUT_AT_INDEX => Ok(Self::CreateOutputAtIndex),
            GTF_CREATE_WITNESS_AT_INDEX => Ok(Self::CreateWitnessAtIndex),
            GTF_INPUT_TYPE => Ok(Self::InputType),
            GTF_INPUT_COIN_TX_ID => Ok(Self::InputCoinTxId),
            GTF_INPUT_COIN_OUTPUT_INDEX => Ok(Self::InputCoinOutputIndex),
            GTF_INPUT_COIN_OWNER => Ok(Self::InputCoinOwner),
            GTF_INPUT_COIN_AMOUNT => Ok(Self::InputCoinAmount),
            GTF_INPUT_COIN_ASSET_ID => Ok(Self::InputCoinAssetId),
            GTF_INPUT_COIN_TX_POINTER => Ok(Self::InputCoinTxPointer),
            GTF_INPUT_COIN_WITNESS_INDEX => Ok(Self::InputCoinWitnessIndex),
            GTF_INPUT_COIN_MATURITY => Ok(Self::InputCoinMaturity),
            GTF_INPUT_COIN_PREDICATE_LENGTH => Ok(Self::InputCoinPredicateLength),
            GTF_INPUT_COIN_PREDICATE_DATA_LENGTH => {
                Ok(Self::InputCoinPredicateDataLength)
            }
            GTF_INPUT_COIN_PREDICATE => Ok(Self::InputCoinPredicate),
            GTF_INPUT_COIN_PREDICATE_DATA => Ok(Self::InputCoinPredicateData),
            GTF_INPUT_COIN_PREDICATE_GAS_USED => Ok(Self::InputCoinPredicateGasUsed),
            GTF_INPUT_CONTRACT_TX_ID => Ok(Self::InputContractTxId),
            GTF_INPUT_CONTRACT_OUTPUT_INDEX => Ok(Self::InputContractOutputIndex),
            GTF_INPUT_CONTRACT_BALANCE_ROOT => Ok(Self::InputContractBalanceRoot),
            GTF_INPUT_CONTRACT_STATE_ROOT => Ok(Self::InputContractStateRoot),
            GTF_INPUT_CONTRACT_TX_POINTER => Ok(Self::InputContractTxPointer),
            GTF_INPUT_CONTRACT_ID => Ok(Self::InputContractId),
            GTF_INPUT_MESSAGE_SENDER => Ok(Self::InputMessageSender),
            GTF_INPUT_MESSAGE_RECIPIENT => Ok(Self::InputMessageRecipient),
            GTF_INPUT_MESSAGE_AMOUNT => Ok(Self::InputMessageAmount),
            GTF_INPUT_MESSAGE_NONCE => Ok(Self::InputMessageNonce),
            GTF_INPUT_MESSAGE_WITNESS_INDEX => Ok(Self::InputMessageWitnessIndex),
            GTF_INPUT_MESSAGE_DATA_LENGTH => Ok(Self::InputMessageDataLength),
            GTF_INPUT_MESSAGE_PREDICATE_LENGTH => Ok(Self::InputMessagePredicateLength),
            GTF_INPUT_MESSAGE_PREDICATE_DATA_LENGTH => {
                Ok(Self::InputMessagePredicateDataLength)
            }
            GTF_INPUT_MESSAGE_DATA => Ok(Self::InputMessageData),
            GTF_INPUT_MESSAGE_PREDICATE => Ok(Self::InputMessagePredicate),
            GTF_INPUT_MESSAGE_PREDICATE_DATA => Ok(Self::InputMessagePredicateData),
            GTF_INPUT_MESSAGE_PREDICATE_GAS_USED => {
                Ok(Self::InputMessagePredicateGasUsed)
            }
            GTF_OUTPUT_TYPE => Ok(Self::OutputType),
            GTF_OUTPUT_COIN_TO => Ok(Self::OutputCoinTo),
            GTF_OUTPUT_COIN_AMOUNT => Ok(Self::OutputCoinAmount),
            GTF_OUTPUT_COIN_ASSET_ID => Ok(Self::OutputCoinAssetId),
            GTF_OUTPUT_CONTRACT_INPUT_INDEX => Ok(Self::OutputContractInputIndex),
            GTF_OUTPUT_CONTRACT_BALANCE_ROOT => Ok(Self::OutputContractBalanceRoot),
            GTF_OUTPUT_CONTRACT_STATE_ROOT => Ok(Self::OutputContractStateRoot),
            GTF_OUTPUT_CONTRACT_CREATED_CONTRACT_ID => {
                Ok(Self::OutputContractCreatedContractId)
            }
            GTF_OUTPUT_CONTRACT_CREATED_STATE_ROOT => {
                Ok(Self::OutputContractCreatedStateRoot)
            }
            GTF_WITNESS_DATA_LENGTH => Ok(Self::WitnessDataLength),
            GTF_WITNESS_DATA => Ok(Self::WitnessData),
            _ => Err(PanicReason::InvalidMetadataIdentifier),
        }
    }
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
