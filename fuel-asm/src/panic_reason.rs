use crate::{Instruction, Opcode, Word};

use core::{convert, fmt, mem};

const WORD_SIZE: usize = mem::size_of::<Word>();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde-types-minimal",
    derive(serde::Serialize, serde::Deserialize)
)]
#[repr(u8)]
/// Panic reason representation for the interpreter.
pub enum PanicReason {
    /// Representation reserved per protocol.
    RESERV00 = 0x00,
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
    /// The color balance isn't enough for the instruction.
    NotEnoughBalance = 0x08,
    /// The interpreter is expected to be in internal context.
    ExpectedInternalContext = 0x09,
    /// The queried color was not found in the state.
    ColorNotFound = 0x0a,
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
    /// The internal color balance overflowed with the provided instruction.
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
    /// RESERV1F
    RESERV1F = 0x1f,
    /// RESERV20
    RESERV20 = 0x20,
    /// RESERV21
    RESERV21 = 0x21,
    /// RESERV22
    RESERV22 = 0x22,
    /// RESERV23
    RESERV23 = 0x23,
    /// RESERV24
    RESERV24 = 0x24,
    /// RESERV25
    RESERV25 = 0x25,
    /// RESERV26
    RESERV26 = 0x26,
    /// RESERV27
    RESERV27 = 0x27,
    /// RESERV28
    RESERV28 = 0x28,
    /// RESERV29
    RESERV29 = 0x29,
    /// RESERV2A
    RESERV2A = 0x2a,
    /// RESERV2B
    RESERV2B = 0x2b,
    /// RESERV2C
    RESERV2C = 0x2c,
    /// RESERV2D
    RESERV2D = 0x2d,
    /// RESERV2E
    RESERV2E = 0x2e,
    /// RESERV2F
    RESERV2F = 0x2f,
    /// RESERV30
    RESERV30 = 0x30,
    /// RESERV31
    RESERV31 = 0x31,
    /// RESERV32
    RESERV32 = 0x32,
    /// RESERV33
    RESERV33 = 0x33,
    /// RESERV34
    RESERV34 = 0x34,
    /// RESERV35
    RESERV35 = 0x35,
    /// RESERV36
    RESERV36 = 0x36,
    /// RESERV37
    RESERV37 = 0x37,
    /// RESERV38
    RESERV38 = 0x38,
    /// RESERV39
    RESERV39 = 0x39,
    /// RESERV3A
    RESERV3A = 0x3a,
    /// RESERV3B
    RESERV3B = 0x3b,
    /// RESERV3C
    RESERV3C = 0x3c,
    /// RESERV3D
    RESERV3D = 0x3d,
    /// RESERV3E
    RESERV3E = 0x3e,
    /// RESERV3F
    RESERV3F = 0x3f,
    /// RESERV40
    RESERV40 = 0x40,
    /// RESERV41
    RESERV41 = 0x41,
    /// RESERV42
    RESERV42 = 0x42,
    /// RESERV43
    RESERV43 = 0x43,
    /// RESERV44
    RESERV44 = 0x44,
    /// RESERV45
    RESERV45 = 0x45,
    /// RESERV46
    RESERV46 = 0x46,
    /// RESERV47
    RESERV47 = 0x47,
    /// RESERV48
    RESERV48 = 0x48,
    /// RESERV49
    RESERV49 = 0x49,
    /// RESERV4A
    RESERV4A = 0x4a,
    /// RESERV4B
    RESERV4B = 0x4b,
    /// RESERV4C
    RESERV4C = 0x4c,
    /// RESERV4D
    RESERV4D = 0x4d,
    /// RESERV4E
    RESERV4E = 0x4e,
    /// RESERV4F
    RESERV4F = 0x4f,
    /// RESERV50
    RESERV50 = 0x50,
    /// RESERV51
    RESERV51 = 0x51,
    /// RESERV52
    RESERV52 = 0x52,
    /// RESERV53
    RESERV53 = 0x53,
    /// RESERV54
    RESERV54 = 0x54,
    /// RESERV55
    RESERV55 = 0x55,
    /// RESERV56
    RESERV56 = 0x56,
    /// RESERV57
    RESERV57 = 0x57,
    /// RESERV58
    RESERV58 = 0x58,
    /// RESERV59
    RESERV59 = 0x59,
    /// RESERV5A
    RESERV5A = 0x5a,
    /// RESERV5B
    RESERV5B = 0x5b,
    /// RESERV5C
    RESERV5C = 0x5c,
    /// RESERV5D
    RESERV5D = 0x5d,
    /// RESERV5E
    RESERV5E = 0x5e,
    /// RESERV5F
    RESERV5F = 0x5f,
    /// RESERV60
    RESERV60 = 0x60,
    /// RESERV61
    RESERV61 = 0x61,
    /// RESERV62
    RESERV62 = 0x62,
    /// RESERV63
    RESERV63 = 0x63,
    /// RESERV64
    RESERV64 = 0x64,
    /// RESERV65
    RESERV65 = 0x65,
    /// RESERV66
    RESERV66 = 0x66,
    /// RESERV67
    RESERV67 = 0x67,
    /// RESERV68
    RESERV68 = 0x68,
    /// RESERV69
    RESERV69 = 0x69,
    /// RESERV6A
    RESERV6A = 0x6a,
    /// RESERV6B
    RESERV6B = 0x6b,
    /// RESERV6C
    RESERV6C = 0x6c,
    /// RESERV6D
    RESERV6D = 0x6d,
    /// RESERV6E
    RESERV6E = 0x6e,
    /// RESERV6F
    RESERV6F = 0x6f,
    /// RESERV70
    RESERV70 = 0x70,
    /// RESERV71
    RESERV71 = 0x71,
    /// RESERV72
    RESERV72 = 0x72,
    /// RESERV73
    RESERV73 = 0x73,
    /// RESERV74
    RESERV74 = 0x74,
    /// RESERV75
    RESERV75 = 0x75,
    /// RESERV76
    RESERV76 = 0x76,
    /// RESERV77
    RESERV77 = 0x77,
    /// RESERV78
    RESERV78 = 0x78,
    /// RESERV79
    RESERV79 = 0x79,
    /// RESERV7A
    RESERV7A = 0x7a,
    /// RESERV7B
    RESERV7B = 0x7b,
    /// RESERV7C
    RESERV7C = 0x7c,
    /// RESERV7D
    RESERV7D = 0x7d,
    /// RESERV7E
    RESERV7E = 0x7e,
    /// RESERV7F
    RESERV7F = 0x7f,
    /// RESERV80
    RESERV80 = 0x80,
    /// RESERV81
    RESERV81 = 0x81,
    /// RESERV82
    RESERV82 = 0x82,
    /// RESERV83
    RESERV83 = 0x83,
    /// RESERV84
    RESERV84 = 0x84,
    /// RESERV85
    RESERV85 = 0x85,
    /// RESERV86
    RESERV86 = 0x86,
    /// RESERV87
    RESERV87 = 0x87,
    /// RESERV88
    RESERV88 = 0x88,
    /// RESERV89
    RESERV89 = 0x89,
    /// RESERV8A
    RESERV8A = 0x8a,
    /// RESERV8B
    RESERV8B = 0x8b,
    /// RESERV8C
    RESERV8C = 0x8c,
    /// RESERV8D
    RESERV8D = 0x8d,
    /// RESERV8E
    RESERV8E = 0x8e,
    /// RESERV8F
    RESERV8F = 0x8f,
    /// RESERV90
    RESERV90 = 0x90,
    /// RESERV91
    RESERV91 = 0x91,
    /// RESERV92
    RESERV92 = 0x92,
    /// RESERV93
    RESERV93 = 0x93,
    /// RESERV94
    RESERV94 = 0x94,
    /// RESERV95
    RESERV95 = 0x95,
    /// RESERV96
    RESERV96 = 0x96,
    /// RESERV97
    RESERV97 = 0x97,
    /// RESERV98
    RESERV98 = 0x98,
    /// RESERV99
    RESERV99 = 0x99,
    /// RESERV9A
    RESERV9A = 0x9a,
    /// RESERV9B
    RESERV9B = 0x9b,
    /// RESERV9C
    RESERV9C = 0x9c,
    /// RESERV9D
    RESERV9D = 0x9d,
    /// RESERV9E
    RESERV9E = 0x9e,
    /// RESERV9F
    RESERV9F = 0x9f,
    /// RESERVA0
    RESERVA0 = 0xa0,
    /// RESERVA1
    RESERVA1 = 0xa1,
    /// RESERVA2
    RESERVA2 = 0xa2,
    /// RESERVA3
    RESERVA3 = 0xa3,
    /// RESERVA4
    RESERVA4 = 0xa4,
    /// RESERVA5
    RESERVA5 = 0xa5,
    /// RESERVA6
    RESERVA6 = 0xa6,
    /// RESERVA7
    RESERVA7 = 0xa7,
    /// RESERVA8
    RESERVA8 = 0xa8,
    /// RESERVA9
    RESERVA9 = 0xa9,
    /// RESERVAA
    RESERVAA = 0xaa,
    /// RESERVAB
    RESERVAB = 0xab,
    /// RESERVAC
    RESERVAC = 0xac,
    /// RESERVAD
    RESERVAD = 0xad,
    /// RESERVAE
    RESERVAE = 0xae,
    /// RESERVAF
    RESERVAF = 0xaf,
    /// RESERVB0
    RESERVB0 = 0xb0,
    /// RESERVB1
    RESERVB1 = 0xb1,
    /// RESERVB2
    RESERVB2 = 0xb2,
    /// RESERVB3
    RESERVB3 = 0xb3,
    /// RESERVB4
    RESERVB4 = 0xb4,
    /// RESERVB5
    RESERVB5 = 0xb5,
    /// RESERVB6
    RESERVB6 = 0xb6,
    /// RESERVB7
    RESERVB7 = 0xb7,
    /// RESERVB8
    RESERVB8 = 0xb8,
    /// RESERVB9
    RESERVB9 = 0xb9,
    /// RESERVBA
    RESERVBA = 0xba,
    /// RESERVBB
    RESERVBB = 0xbb,
    /// RESERVBC
    RESERVBC = 0xbc,
    /// RESERVBD
    RESERVBD = 0xbd,
    /// RESERVBE
    RESERVBE = 0xbe,
    /// RESERVBF
    RESERVBF = 0xbf,
    /// RESERVC0
    RESERVC0 = 0xc0,
    /// RESERVC1
    RESERVC1 = 0xc1,
    /// RESERVC2
    RESERVC2 = 0xc2,
    /// RESERVC3
    RESERVC3 = 0xc3,
    /// RESERVC4
    RESERVC4 = 0xc4,
    /// RESERVC5
    RESERVC5 = 0xc5,
    /// RESERVC6
    RESERVC6 = 0xc6,
    /// RESERVC7
    RESERVC7 = 0xc7,
    /// RESERVC8
    RESERVC8 = 0xc8,
    /// RESERVC9
    RESERVC9 = 0xc9,
    /// RESERVCA
    RESERVCA = 0xca,
    /// RESERVCB
    RESERVCB = 0xcb,
    /// RESERVCC
    RESERVCC = 0xcc,
    /// RESERVCD
    RESERVCD = 0xcd,
    /// RESERVCE
    RESERVCE = 0xce,
    /// RESERVCF
    RESERVCF = 0xcf,
    /// RESERVD0
    RESERVD0 = 0xd0,
    /// RESERVD1
    RESERVD1 = 0xd1,
    /// RESERVD2
    RESERVD2 = 0xd2,
    /// RESERVD3
    RESERVD3 = 0xd3,
    /// RESERVD4
    RESERVD4 = 0xd4,
    /// RESERVD5
    RESERVD5 = 0xd5,
    /// RESERVD6
    RESERVD6 = 0xd6,
    /// RESERVD7
    RESERVD7 = 0xd7,
    /// RESERVD8
    RESERVD8 = 0xd8,
    /// RESERVD9
    RESERVD9 = 0xd9,
    /// RESERVDA
    RESERVDA = 0xda,
    /// RESERVDB
    RESERVDB = 0xdb,
    /// RESERVDC
    RESERVDC = 0xdc,
    /// RESERVDD
    RESERVDD = 0xdd,
    /// RESERVDE
    RESERVDE = 0xde,
    /// RESERVDF
    RESERVDF = 0xdf,
    /// RESERVE0
    RESERVE0 = 0xe0,
    /// RESERVE1
    RESERVE1 = 0xe1,
    /// RESERVE2
    RESERVE2 = 0xe2,
    /// RESERVE3
    RESERVE3 = 0xe3,
    /// RESERVE4
    RESERVE4 = 0xe4,
    /// RESERVE5
    RESERVE5 = 0xe5,
    /// RESERVE6
    RESERVE6 = 0xe6,
    /// RESERVE7
    RESERVE7 = 0xe7,
    /// RESERVE8
    RESERVE8 = 0xe8,
    /// RESERVE9
    RESERVE9 = 0xe9,
    /// RESERVEA
    RESERVEA = 0xea,
    /// RESERVEB
    RESERVEB = 0xeb,
    /// RESERVEC
    RESERVEC = 0xec,
    /// RESERVED
    RESERVED = 0xed,
    /// RESERVEE
    RESERVEE = 0xee,
    /// RESERVEF
    RESERVEF = 0xef,
    /// RESERVF0
    RESERVF0 = 0xf0,
    /// RESERVF1
    RESERVF1 = 0xf1,
    /// RESERVF2
    RESERVF2 = 0xf2,
    /// RESERVF3
    RESERVF3 = 0xf3,
    /// RESERVF4
    RESERVF4 = 0xf4,
    /// RESERVF5
    RESERVF5 = 0xf5,
    /// RESERVF6
    RESERVF6 = 0xf6,
    /// RESERVF7
    RESERVF7 = 0xf7,
    /// RESERVF8
    RESERVF8 = 0xf8,
    /// RESERVF9
    RESERVF9 = 0xf9,
    /// RESERVFA
    RESERVFA = 0xfa,
    /// RESERVFB
    RESERVFB = 0xfb,
    /// RESERVFC
    RESERVFC = 0xfc,
    /// RESERVFD
    RESERVFD = 0xfd,
    /// RESERVFE
    RESERVFE = 0xfe,
    /// RESERVFF
    RESERVFF = 0xff,
}

impl fmt::Display for PanicReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for PanicReason {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl From<convert::Infallible> for PanicReason {
    fn from(_i: convert::Infallible) -> Self {
        unreachable!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde-types-minimal",
    derive(serde::Serialize, serde::Deserialize)
)]
/// Describe a panic reason with the instruction that generated it
pub struct InstructionResult {
    reason: PanicReason,
    instruction: Instruction,
}

impl InstructionResult {
    /// Represents success
    pub const fn success() -> Self {
        let reason = PanicReason::from_u8(0);
        let instruction = Instruction::new(0);

        Self {
            reason,
            instruction,
        }
    }

    /// Represents an error described by a reason and an instruction.
    pub const fn error(reason: PanicReason, instruction: Instruction) -> Self {
        Self {
            reason,
            instruction,
        }
    }

    /// Underlying panic reason
    pub const fn reason(&self) -> &PanicReason {
        &self.reason
    }

    /// Underlying instruction
    pub const fn instruction(&self) -> &Instruction {
        &self.instruction
    }

    /// This result represents success?
    pub const fn is_success(&self) -> bool {
        (self.reason as u8) == 0u8
    }

    /// This result represents error?
    pub const fn is_error(&self) -> bool {
        !self.is_success()
    }
}

const REASON_OFFSET: Word = (WORD_SIZE * 8 - 8) as Word;
const INSTR_OFFSET: Word = ((WORD_SIZE - mem::size_of::<u32>()) * 8 - 8) as Word;

impl From<InstructionResult> for Word {
    fn from(r: InstructionResult) -> Word {
        let reason = Word::from(r.reason);
        let instruction = (reason != 0) as Word * u32::from(r.instruction) as Word;

        (reason << REASON_OFFSET) | (instruction << INSTR_OFFSET)
    }
}

impl From<Word> for InstructionResult {
    fn from(val: Word) -> Self {
        let reason = val >> REASON_OFFSET;
        let instruction = val >> INSTR_OFFSET;
        let instruction = (reason != 0) as Word * instruction;

        let reason = PanicReason::from(reason);
        let instruction = Instruction::from(instruction as u32);

        Self {
            reason,
            instruction,
        }
    }
}

impl From<InstructionResult> for Instruction {
    fn from(r: InstructionResult) -> Self {
        r.instruction
    }
}

impl From<InstructionResult> for Opcode {
    fn from(r: InstructionResult) -> Self {
        r.instruction.into()
    }
}

impl From<InstructionResult> for PanicReason {
    fn from(r: InstructionResult) -> Self {
        r.reason
    }
}
