use fuel_asm::{Instruction, Opcode};
use fuel_types::Word;

use std::mem;

const WORD_SIZE: usize = mem::size_of::<Word>();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-types", derive(serde::Serialize, serde::Deserialize))]
pub enum PanicReason {
    Success = 0x00,
    Revert = 0x01,
    OutOfGas = 0x02,
    TransactionValidity = 0x03,
    MemoryOverflow = 0x04,
    ArithmeticOverflow = 0x05,
    ContractNotFound = 0x06,
    MemoryOwnership = 0x07,
    NotEnoughBalance = 0x08,
    ExpectedInternalContext = 0x09,
    ColorNotFound = 0x0a,
    InputNotFound = 0x0b,
    OutputNotFound = 0x0c,
    WitnessNotFound = 0x0d,
    TransactionMaturity = 0x0e,
    InvalidMetadataIdentifier = 0x0f,
    MalformedCallStructure = 0x10,
    ReservedRegisterNotWritable = 0x11,
    ErrorFlag = 0x12,
    InvalidImmediateValue = 0x13,
    ExpectedCoinInput = 0x14,
    MaxMemoryAccess = 0x15,
    MemoryWriteOverlap = 0x16,
    ContractNotInInputs = 0x17,
    InternalBalanceOverflow = 0x18,
    ContractMaxSize = 0x19,
    ExpectedUnallocatedStack = 0x1a,
    MaxStaticContractsReached = 0x1b,
    TransferAmountCannotBeZero = 0x1c,
    ExpectedOutputVariable = 0x1d,
    ExpectedParentInternalContext = 0x1e,
    InvalidRepresentation = 0xff,
}

impl From<PanicReason> for Word {
    fn from(r: PanicReason) -> Word {
        r as Word
    }
}

impl From<Word> for PanicReason {
    fn from(b: Word) -> Self {
        match b {
            0x00 => Self::Success,
            0x01 => Self::Revert,
            0x02 => Self::OutOfGas,
            0x03 => Self::TransactionValidity,
            0x04 => Self::MemoryOverflow,
            0x05 => Self::ArithmeticOverflow,
            0x06 => Self::ContractNotFound,
            0x07 => Self::MemoryOwnership,
            0x08 => Self::NotEnoughBalance,
            0x09 => Self::ExpectedInternalContext,
            0x0a => Self::ColorNotFound,
            0x0b => Self::InputNotFound,
            0x0c => Self::OutputNotFound,
            0x0d => Self::WitnessNotFound,
            0x0e => Self::TransactionMaturity,
            0x0f => Self::InvalidMetadataIdentifier,
            0x10 => Self::MalformedCallStructure,
            0x11 => Self::ReservedRegisterNotWritable,
            0x12 => Self::ErrorFlag,
            0x13 => Self::InvalidImmediateValue,
            0x14 => Self::ExpectedCoinInput,
            0x15 => Self::MaxMemoryAccess,
            0x16 => Self::MemoryWriteOverlap,
            0x17 => Self::ContractNotInInputs,
            0x18 => Self::InternalBalanceOverflow,
            0x19 => Self::ContractMaxSize,
            0x1a => Self::ExpectedUnallocatedStack,
            0x1b => Self::MaxStaticContractsReached,
            0x1c => Self::TransferAmountCannotBeZero,
            0x1d => Self::ExpectedOutputVariable,
            0x1e => Self::ExpectedParentInternalContext,
            _ => Self::InvalidRepresentation,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-types", derive(serde::Serialize, serde::Deserialize))]
pub struct ScriptResult {
    result: PanicReason,
    instruction: Instruction,
}

impl ScriptResult {
    pub const fn new(result: PanicReason, instruction: Instruction) -> Self {
        Self {
            result,
            instruction,
        }
    }

    pub const fn result(&self) -> &PanicReason {
        &self.result
    }

    pub const fn instruction(&self) -> &Instruction {
        &self.instruction
    }
}

const RESULT_OFFSET: Word = (WORD_SIZE * 8 - 8) as Word;
const INSTR_OFFSET: Word = ((WORD_SIZE - mem::size_of::<u32>()) * 8 - 8) as Word;

impl From<ScriptResult> for Word {
    fn from(r: ScriptResult) -> Word {
        let result = Word::from(r.result);
        let instruction = u32::from(r.instruction) as Word;

        (result << RESULT_OFFSET) | (instruction << INSTR_OFFSET)
    }
}

impl From<Word> for ScriptResult {
    fn from(val: Word) -> Self {
        let result = PanicReason::from(val >> RESULT_OFFSET);
        let instruction = Instruction::from((val >> INSTR_OFFSET) as u32);

        Self::new(result, instruction)
    }
}

impl From<ScriptResult> for Instruction {
    fn from(r: ScriptResult) -> Self {
        r.instruction
    }
}

impl From<ScriptResult> for Opcode {
    fn from(r: ScriptResult) -> Self {
        r.instruction.into()
    }
}

impl From<ScriptResult> for PanicReason {
    fn from(r: ScriptResult) -> Self {
        r.result
    }
}
