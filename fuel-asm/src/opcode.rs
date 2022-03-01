use fuel_types::{bytes, Immediate12, Immediate18, Immediate24, RegisterId, Word};

#[cfg(feature = "std")]
use std::{io, iter};

use crate::Instruction;

mod consts;

pub use consts::OpcodeRepr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde-types-minimal",
    derive(serde::Serialize, serde::Deserialize)
)]
/// Instruction representation for the interpreter.
///
/// ## Memory Opcodes
///
/// All these opcodes advance the program counter `$pc` by `4` after performing
/// their operation. Every instruction is guaranteed to fit in `u32`
/// representation.
///
/// ## Arithmetic/Logic (ALU) Opcodes
///
/// All these opcodes advance the program counter `$pc` by `4` after performing
/// their operation.
///
/// If the [`F_UNSAFEMATH`](./main.md#flags) flag is unset, an operation that
/// would have set `$err` to `true` is instead a panic.
///
/// If the [`F_WRAPPING`](./main.md#flags) flag is unset, an operation that
/// would have set `$of` to a non-zero value is instead a panic. ## Contract
/// Opcodes
///
/// All these opcodes advance the program counter `$pc` by `4` after performing
/// their operation, except for [CALL](#call-call-contract) and
/// [REVERT](#revert-revert).
///
/// ## Cryptographic Opcodes
///
/// All these opcodes advance the program counter `$pc` by `4` after performing
/// their operation.
pub enum Opcode {
    /// Adds two registers.
    ADD(RegisterId, RegisterId, RegisterId),

    /// Adds a register and an immediate value.
    ADDI(RegisterId, RegisterId, Immediate12),

    /// Bitwise ANDs two registers.
    AND(RegisterId, RegisterId, RegisterId),

    /// Bitwise ANDs a register and an immediate value.
    ANDI(RegisterId, RegisterId, Immediate12),

    /// Divides two registers.
    DIV(RegisterId, RegisterId, RegisterId),

    /// Divides a register and an immediate value.
    DIVI(RegisterId, RegisterId, Immediate12),

    /// Compares two registers for equality.
    EQ(RegisterId, RegisterId, RegisterId),

    /// Raises one register to the power of another.
    EXP(RegisterId, RegisterId, RegisterId),

    /// Raises one register to the power of an immediate value.
    EXPI(RegisterId, RegisterId, Immediate12),

    /// Compares two registers for greater-than.
    GT(RegisterId, RegisterId, RegisterId),

    /// Compares two registers for less-than.
    LT(RegisterId, RegisterId, RegisterId),

    /// The integer logarithm of a register.
    MLOG(RegisterId, RegisterId, RegisterId),

    /// The integer root of a register.
    MROO(RegisterId, RegisterId, RegisterId),

    /// Modulo remainder of two registers.
    MOD(RegisterId, RegisterId, RegisterId),

    /// Modulo remainder of a register and an immediate value.
    MODI(RegisterId, RegisterId, Immediate12),

    /// Copy from one register to another.
    MOVE(RegisterId, RegisterId),

    /// Multiplies two registers.
    MUL(RegisterId, RegisterId, RegisterId),

    /// Multiplies a register and an immediate value.
    MULI(RegisterId, RegisterId, Immediate12),

    /// Bitwise NOT a register.
    NOT(RegisterId, RegisterId),

    /// Bitwise ORs two registers.
    OR(RegisterId, RegisterId, RegisterId),

    /// Bitwise ORs a register and an immediate value.
    ORI(RegisterId, RegisterId, Immediate12),

    /// Left shifts a register by a register.
    SLL(RegisterId, RegisterId, RegisterId),

    /// Left shifts a register by an immediate value.
    SLLI(RegisterId, RegisterId, Immediate12),

    /// Right shifts a register by a register.
    SRL(RegisterId, RegisterId, RegisterId),

    /// Right shifts a register by an immediate value.
    SRLI(RegisterId, RegisterId, Immediate12),

    /// Subtracts two registers.
    SUB(RegisterId, RegisterId, RegisterId),

    /// Subtracts a register and an immediate value.
    SUBI(RegisterId, RegisterId, Immediate12),

    /// Bitwise XORs two registers.
    XOR(RegisterId, RegisterId, RegisterId),

    /// Bitwise XORs a register and an immediate value.
    XORI(RegisterId, RegisterId, Immediate12),

    /// Check relative timelock.
    CIMV(RegisterId, RegisterId, RegisterId),

    /// Check absolute timelock.
    CTMV(RegisterId, RegisterId),

    /// Jump.
    JI(Immediate24),

    /// Conditional jump.
    JNEI(RegisterId, RegisterId, Immediate12),

    /// Return from context.
    RET(RegisterId),

    /// Return from context with data.
    RETD(RegisterId, RegisterId),

    /// Extend the current call frame's stack by an immediate value.
    CFEI(Immediate24),

    /// Shrink the current call frame's stack by an immediate value.
    CFSI(Immediate24),

    /// A byte is loaded from the specified address offset by an immediate value.
    LB(RegisterId, RegisterId, Immediate12),

    /// A word is loaded from the specified address offset by an immediate value.
    LW(RegisterId, RegisterId, Immediate12),

    /// Allocate a number of bytes from the heap.
    ALOC(RegisterId),

    /// Clear a variable number of bytes in memory.
    MCL(RegisterId, RegisterId),

    /// Clear an immediate number of bytes in memory.
    MCLI(RegisterId, Immediate18),

    /// Copy a variable number of bytes in memory.
    MCP(RegisterId, RegisterId, RegisterId),

    /// Copy an immediate number of bytes in memory.
    MCPI(RegisterId, RegisterId, Immediate12),

    /// Compare bytes in memory.
    MEQ(RegisterId, RegisterId, RegisterId, RegisterId),

    /// Write the least significant byte of a register to memory.
    SB(RegisterId, RegisterId, Immediate12),

    /// Write a register to memory.
    SW(RegisterId, RegisterId, Immediate12),

    /// Get the balance of contract of an asset ID.
    BAL(RegisterId, RegisterId, RegisterId),

    /// Get block header hash for height.
    BHSH(RegisterId, RegisterId),

    /// Get current block height.
    BHEI(RegisterId),

    /// Burn coins of the current contract's asset ID.
    BURN(RegisterId),

    /// Call a contract.
    CALL(RegisterId, RegisterId, RegisterId, RegisterId),

    /// Copy contract code for a contract.
    CCP(RegisterId, RegisterId, RegisterId, RegisterId),

    /// Get code root of a contract.
    CROO(RegisterId, RegisterId),

    /// Get code size of a contract.
    CSIZ(RegisterId, RegisterId),

    /// Get current block proposer's address.
    CB(RegisterId),

    /// Load a contract's code as executable.
    LDC(RegisterId, RegisterId, RegisterId),

    /// Log an event.
    LOG(RegisterId, RegisterId, RegisterId, RegisterId),

    /// Log data.
    LOGD(RegisterId, RegisterId, RegisterId, RegisterId),

    /// Mint coins of the current contract's asset ID.
    MINT(RegisterId),

    /// Halt execution, reverting state changes and returning a value.
    RVRT(RegisterId),

    /// Load a static contract's code as executable.
    SLDC(RegisterId, RegisterId, RegisterId),

    /// Load a word from contract storage.
    SRW(RegisterId, RegisterId),

    /// Load 32 bytes from contract storage.
    SRWQ(RegisterId, RegisterId),

    /// Store a word in contract storage.
    SWW(RegisterId, RegisterId),

    /// Store 32 bytes in contract storage.
    SWWQ(RegisterId, RegisterId),

    /// Transfer coins to a contract unconditionally.
    TR(RegisterId, RegisterId, RegisterId),

    /// Transfer coins to a variable output.
    TRO(RegisterId, RegisterId, RegisterId, RegisterId),

    /// The 64-byte public key (x, y) recovered from 64-byte
    /// signature on 32-byte message.
    ECR(RegisterId, RegisterId, RegisterId),

    /// The keccak-256 hash of a slice.
    K256(RegisterId, RegisterId, RegisterId),

    /// The SHA-2-256 hash of a slice.
    S256(RegisterId, RegisterId, RegisterId),

    /// Get the length in bytes of an input.
    XIL(RegisterId, RegisterId),

    /// Get the memory addess of the start of an input.
    XIS(RegisterId, RegisterId),

    /// Get the length in bytes of an output.
    XOL(RegisterId, RegisterId),

    /// Get the memory addess of the start of an output.
    XOS(RegisterId, RegisterId),

    /// Get the length in bytes of a witness.
    XWL(RegisterId, RegisterId),

    /// Get the memory addess of the start of a witness.
    XWS(RegisterId, RegisterId),

    /// Performs no operation.
    NOOP,

    /// Set flag register to a register.
    FLAG(RegisterId),

    /// Get metadata from memory.
    GM(RegisterId, Immediate18),

    /// Undefined opcode, potentially from inconsistent serialization.
    Undefined,
}

impl Opcode {
    /// Size of the struct when serialized into bytes
    pub const LEN: usize = 4;

    /// Create a new [`Opcode`] given the internal attributes
    pub const fn new(instruction: Instruction) -> Self {
        let op = instruction.op();
        let ra = instruction.ra();
        let rb = instruction.rb();
        let rc = instruction.rc();
        let rd = instruction.rd();
        let imm12 = instruction.imm12();
        let imm18 = instruction.imm18();
        let imm24 = instruction.imm24();

        let repr = OpcodeRepr::from_u8(op);

        match repr {
            OpcodeRepr::ADD => Opcode::ADD(ra, rb, rc),
            OpcodeRepr::ADDI => Opcode::ADDI(ra, rb, imm12),
            OpcodeRepr::AND => Opcode::AND(ra, rb, rc),
            OpcodeRepr::ANDI => Opcode::ANDI(ra, rb, imm12),
            OpcodeRepr::DIV => Opcode::DIV(ra, rb, rc),
            OpcodeRepr::DIVI => Opcode::DIVI(ra, rb, imm12),
            OpcodeRepr::EQ => Opcode::EQ(ra, rb, rc),
            OpcodeRepr::EXP => Opcode::EXP(ra, rb, rc),
            OpcodeRepr::EXPI => Opcode::EXPI(ra, rb, imm12),
            OpcodeRepr::GT => Opcode::GT(ra, rb, rc),
            OpcodeRepr::LT => Opcode::LT(ra, rb, rc),
            OpcodeRepr::MLOG => Opcode::MLOG(ra, rb, rc),
            OpcodeRepr::MROO => Opcode::MROO(ra, rb, rc),
            OpcodeRepr::MOD => Opcode::MOD(ra, rb, rc),
            OpcodeRepr::MODI => Opcode::MODI(ra, rb, imm12),
            OpcodeRepr::MOVE => Opcode::MOVE(ra, rb),
            OpcodeRepr::MUL => Opcode::MUL(ra, rb, rc),
            OpcodeRepr::MULI => Opcode::MULI(ra, rb, imm12),
            OpcodeRepr::NOT => Opcode::NOT(ra, rb),
            OpcodeRepr::OR => Opcode::OR(ra, rb, rc),
            OpcodeRepr::ORI => Opcode::ORI(ra, rb, imm12),
            OpcodeRepr::SLL => Opcode::SLL(ra, rb, rc),
            OpcodeRepr::SLLI => Opcode::SLLI(ra, rb, imm12),
            OpcodeRepr::SRL => Opcode::SRL(ra, rb, rc),
            OpcodeRepr::SRLI => Opcode::SRLI(ra, rb, imm12),
            OpcodeRepr::SUB => Opcode::SUB(ra, rb, rc),
            OpcodeRepr::SUBI => Opcode::SUBI(ra, rb, imm12),
            OpcodeRepr::XOR => Opcode::XOR(ra, rb, rc),
            OpcodeRepr::XORI => Opcode::XORI(ra, rb, imm12),
            OpcodeRepr::CIMV => Opcode::CIMV(ra, rb, rc),
            OpcodeRepr::CTMV => Opcode::CTMV(ra, rb),
            OpcodeRepr::JI => Opcode::JI(imm24),
            OpcodeRepr::JNEI => Opcode::JNEI(ra, rb, imm12),
            OpcodeRepr::RET => Opcode::RET(ra),
            OpcodeRepr::RETD => Opcode::RETD(ra, rb),
            OpcodeRepr::CFEI => Opcode::CFEI(imm24),
            OpcodeRepr::CFSI => Opcode::CFSI(imm24),
            OpcodeRepr::LB => Opcode::LB(ra, rb, imm12),
            OpcodeRepr::LW => Opcode::LW(ra, rb, imm12),
            OpcodeRepr::ALOC => Opcode::ALOC(ra),
            OpcodeRepr::MCL => Opcode::MCL(ra, rb),
            OpcodeRepr::MCLI => Opcode::MCLI(ra, imm18),
            OpcodeRepr::MCP => Opcode::MCP(ra, rb, rc),
            OpcodeRepr::MCPI => Opcode::MCPI(ra, rb, imm12),
            OpcodeRepr::MEQ => Opcode::MEQ(ra, rb, rc, rd),
            OpcodeRepr::SB => Opcode::SB(ra, rb, imm12),
            OpcodeRepr::SW => Opcode::SW(ra, rb, imm12),
            OpcodeRepr::BAL => Opcode::BAL(ra, rb, rc),
            OpcodeRepr::BHSH => Opcode::BHSH(ra, rb),
            OpcodeRepr::BHEI => Opcode::BHEI(ra),
            OpcodeRepr::BURN => Opcode::BURN(ra),
            OpcodeRepr::CALL => Opcode::CALL(ra, rb, rc, rd),
            OpcodeRepr::CCP => Opcode::CCP(ra, rb, rc, rd),
            OpcodeRepr::CROO => Opcode::CROO(ra, rb),
            OpcodeRepr::CSIZ => Opcode::CSIZ(ra, rb),
            OpcodeRepr::CB => Opcode::CB(ra),
            OpcodeRepr::LDC => Opcode::LDC(ra, rb, rc),
            OpcodeRepr::LOG => Opcode::LOG(ra, rb, rc, rd),
            OpcodeRepr::LOGD => Opcode::LOGD(ra, rb, rc, rd),
            OpcodeRepr::MINT => Opcode::MINT(ra),
            OpcodeRepr::RVRT => Opcode::RVRT(ra),
            OpcodeRepr::SLDC => Opcode::SLDC(ra, rb, rc),
            OpcodeRepr::SRW => Opcode::SRW(ra, rb),
            OpcodeRepr::SRWQ => Opcode::SRWQ(ra, rb),
            OpcodeRepr::SWW => Opcode::SWW(ra, rb),
            OpcodeRepr::SWWQ => Opcode::SWWQ(ra, rb),
            OpcodeRepr::TR => Opcode::TR(ra, rb, rc),
            OpcodeRepr::TRO => Opcode::TRO(ra, rb, rc, rd),
            OpcodeRepr::ECR => Opcode::ECR(ra, rb, rc),
            OpcodeRepr::K256 => Opcode::K256(ra, rb, rc),
            OpcodeRepr::S256 => Opcode::S256(ra, rb, rc),
            OpcodeRepr::XIL => Opcode::XIL(ra, rb),
            OpcodeRepr::XIS => Opcode::XIS(ra, rb),
            OpcodeRepr::XOL => Opcode::XOL(ra, rb),
            OpcodeRepr::XOS => Opcode::XOS(ra, rb),
            OpcodeRepr::XWL => Opcode::XWL(ra, rb),
            OpcodeRepr::XWS => Opcode::XWS(ra, rb),
            OpcodeRepr::NOOP => Opcode::NOOP,
            OpcodeRepr::FLAG => Opcode::FLAG(ra),
            OpcodeRepr::GM => Opcode::GM(ra, imm18),
            _ => Opcode::Undefined,
        }
    }

    /// Create a `Opcode` from a slice of bytes
    ///
    /// # Safety
    ///
    /// Reflects the requirements of [`bytes::from_slice_unchecked`]
    pub unsafe fn from_bytes_unchecked(bytes: &[u8]) -> Self {
        debug_assert!(Self::LEN <= bytes.len());

        let op = bytes::from_slice_unchecked(bytes);
        let op = u32::from_be_bytes(op);

        op.into()
    }

    /// Convert the opcode to bytes representation
    pub fn to_bytes(self) -> [u8; Self::LEN] {
        u32::from(self).to_be_bytes()
    }

    /// Transform the [`Opcode`] into an optional array of 4 register
    /// identifiers
    pub const fn registers(&self) -> [Option<RegisterId>; 4] {
        match self {
            Self::ADD(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::ADDI(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::AND(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::ANDI(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::DIV(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::DIVI(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::EQ(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::EXP(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::EXPI(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::GT(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::LT(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::MLOG(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::MROO(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::MOD(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::MODI(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::MOVE(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::MUL(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::MULI(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::NOT(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::OR(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::ORI(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::SLL(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::SLLI(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::SRL(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::SRLI(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::SUB(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::SUBI(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::XOR(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::XORI(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::CIMV(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::CTMV(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::JI(_) => [None; 4],
            Self::JNEI(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::RET(ra) => [Some(*ra), None, None, None],
            Self::RETD(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::CFEI(_) => [None; 4],
            Self::CFSI(_) => [None; 4],
            Self::LB(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::LW(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::ALOC(ra) => [Some(*ra), None, None, None],
            Self::MCL(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::MCLI(ra, _) => [Some(*ra), None, None, None],
            Self::MCP(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::MCPI(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::MEQ(ra, rb, rc, rd) => [Some(*ra), Some(*rb), Some(*rc), Some(*rd)],
            Self::SB(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::SW(ra, rb, _) => [Some(*ra), Some(*rb), None, None],
            Self::BAL(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::BHSH(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::BHEI(ra) => [Some(*ra), None, None, None],
            Self::BURN(ra) => [Some(*ra), None, None, None],
            Self::CALL(ra, rb, rc, rd) => [Some(*ra), Some(*rb), Some(*rc), Some(*rd)],
            Self::CCP(ra, rb, rc, rd) => [Some(*ra), Some(*rb), Some(*rc), Some(*rd)],
            Self::CROO(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::CSIZ(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::CB(ra) => [Some(*ra), None, None, None],
            Self::LDC(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::LOG(ra, rb, rc, rd) => [Some(*ra), Some(*rb), Some(*rc), Some(*rd)],
            Self::LOGD(ra, rb, rc, rd) => [Some(*ra), Some(*rb), Some(*rc), Some(*rd)],
            Self::MINT(ra) => [Some(*ra), None, None, None],
            Self::RVRT(ra) => [Some(*ra), None, None, None],
            Self::SLDC(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::SRW(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::SRWQ(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::SWW(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::SWWQ(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::TR(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::TRO(ra, rb, rc, rd) => [Some(*ra), Some(*rb), Some(*rc), Some(*rd)],
            Self::ECR(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::K256(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::S256(ra, rb, rc) => [Some(*ra), Some(*rb), Some(*rc), None],
            Self::XIL(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::XIS(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::XOL(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::XOS(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::XWL(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::XWS(ra, rb) => [Some(*ra), Some(*rb), None, None],
            Self::NOOP => [None; 4],
            Self::FLAG(ra) => [Some(*ra), None, None, None],
            Self::GM(ra, _) => [Some(*ra), None, None, None],
            Self::Undefined => [None; 4],
        }
    }

    /// Return the underlying immediate value, if present
    pub const fn immediate(&self) -> Option<Word> {
        match self {
            Self::ADDI(_, _, imm)
            | Self::ANDI(_, _, imm)
            | Self::DIVI(_, _, imm)
            | Self::EXPI(_, _, imm)
            | Self::MODI(_, _, imm)
            | Self::MULI(_, _, imm)
            | Self::ORI(_, _, imm)
            | Self::SLLI(_, _, imm)
            | Self::SRLI(_, _, imm)
            | Self::SUBI(_, _, imm)
            | Self::XORI(_, _, imm)
            | Self::JNEI(_, _, imm)
            | Self::MCPI(_, _, imm)
            | Self::LB(_, _, imm)
            | Self::LW(_, _, imm)
            | Self::SB(_, _, imm)
            | Self::SW(_, _, imm) => Some(*imm as Word),

            Self::MCLI(_, imm) | Self::GM(_, imm) => Some(*imm as Word),

            Self::JI(imm) | Self::CFEI(imm) | Self::CFSI(imm) => Some(*imm as Word),

            Self::ADD(_, _, _)
            | Self::AND(_, _, _)
            | Self::DIV(_, _, _)
            | Self::EQ(_, _, _)
            | Self::EXP(_, _, _)
            | Self::GT(_, _, _)
            | Self::LT(_, _, _)
            | Self::MLOG(_, _, _)
            | Self::MROO(_, _, _)
            | Self::MOD(_, _, _)
            | Self::MOVE(_, _)
            | Self::MUL(_, _, _)
            | Self::NOT(_, _)
            | Self::OR(_, _, _)
            | Self::SLL(_, _, _)
            | Self::SRL(_, _, _)
            | Self::SUB(_, _, _)
            | Self::XOR(_, _, _)
            | Self::CIMV(_, _, _)
            | Self::CTMV(_, _)
            | Self::RET(_)
            | Self::RETD(_, _)
            | Self::ALOC(_)
            | Self::MCL(_, _)
            | Self::MCP(_, _, _)
            | Self::MEQ(_, _, _, _)
            | Self::BAL(_, _, _)
            | Self::BHSH(_, _)
            | Self::BHEI(_)
            | Self::BURN(_)
            | Self::CALL(_, _, _, _)
            | Self::CCP(_, _, _, _)
            | Self::CROO(_, _)
            | Self::CSIZ(_, _)
            | Self::CB(_)
            | Self::LDC(_, _, _)
            | Self::LOG(_, _, _, _)
            | Self::LOGD(_, _, _, _)
            | Self::MINT(_)
            | Self::RVRT(_)
            | Self::SLDC(_, _, _)
            | Self::SRW(_, _)
            | Self::SRWQ(_, _)
            | Self::SWW(_, _)
            | Self::SWWQ(_, _)
            | Self::TR(_, _, _)
            | Self::TRO(_, _, _, _)
            | Self::ECR(_, _, _)
            | Self::K256(_, _, _)
            | Self::S256(_, _, _)
            | Self::XIL(_, _)
            | Self::XIS(_, _)
            | Self::XOL(_, _)
            | Self::XOS(_, _)
            | Self::XWL(_, _)
            | Self::XWS(_, _)
            | Self::NOOP
            | Self::FLAG(_)
            | Self::Undefined => None,
        }
    }
}

#[cfg(feature = "std")]
impl Opcode {
    /// Create a `Opcode` from a slice of bytes
    ///
    /// This function will fail if the length of the bytes is smaller than
    /// [`Opcode::LEN`].
    pub fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() < Self::LEN {
            Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "The provided buffer is not big enough!",
            ))
        } else {
            // Safety: checked length
            Ok(unsafe { Self::from_bytes_unchecked(bytes) })
        }
    }

    /// Create a set of `Opcode` from an iterator of bytes
    ///
    /// If not padded to [`Self::LEN`], will consume the unaligned bytes but won't try to parse an
    /// opcode from them.
    pub fn from_bytes_iter<I>(bytes: I) -> Vec<Self>
    where
        I: IntoIterator<Item = u8>,
    {
        let mut bytes = bytes.into_iter();
        let mut buf = [0u8; Self::LEN];
        let mut ret = Vec::with_capacity(bytes.size_hint().0 / Self::LEN);

        loop {
            let n = bytes
                .by_ref()
                .take(Self::LEN)
                .zip(buf.as_mut().iter_mut())
                .fold(0, |n, (x, b)| {
                    *b = x;

                    n + 1
                });

            if n < Self::LEN {
                break;
            }

            ret.push(Self::from(buf));
        }

        ret
    }
}

impl From<Instruction> for Opcode {
    fn from(parsed: Instruction) -> Self {
        Self::new(parsed)
    }
}

impl From<[u8; Opcode::LEN]> for Opcode {
    fn from(b: [u8; Opcode::LEN]) -> Self {
        u32::from_be_bytes(b).into()
    }
}

impl From<u32> for Opcode {
    fn from(instruction: u32) -> Self {
        Self::new(Instruction::from(instruction))
    }
}

impl From<Opcode> for u32 {
    fn from(opcode: Opcode) -> u32 {
        match opcode {
            Opcode::ADD(ra, rb, rc) => {
                ((OpcodeRepr::ADD as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::ADDI(ra, rb, imm12) => {
                ((OpcodeRepr::ADDI as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::AND(ra, rb, rc) => {
                ((OpcodeRepr::AND as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::ANDI(ra, rb, imm12) => {
                ((OpcodeRepr::ANDI as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::DIV(ra, rb, rc) => {
                ((OpcodeRepr::DIV as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::DIVI(ra, rb, imm12) => {
                ((OpcodeRepr::DIVI as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::EQ(ra, rb, rc) => {
                ((OpcodeRepr::EQ as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::EXP(ra, rb, rc) => {
                ((OpcodeRepr::EXP as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::EXPI(ra, rb, imm12) => {
                ((OpcodeRepr::EXPI as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::GT(ra, rb, rc) => {
                ((OpcodeRepr::GT as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::LT(ra, rb, rc) => {
                ((OpcodeRepr::LT as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::MLOG(ra, rb, rc) => {
                ((OpcodeRepr::MLOG as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::MROO(ra, rb, rc) => {
                ((OpcodeRepr::MROO as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::MOD(ra, rb, rc) => {
                ((OpcodeRepr::MOD as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::MODI(ra, rb, imm12) => {
                ((OpcodeRepr::MODI as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::MOVE(ra, rb) => {
                ((OpcodeRepr::MOVE as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::MUL(ra, rb, rc) => {
                ((OpcodeRepr::MUL as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::MULI(ra, rb, imm12) => {
                ((OpcodeRepr::MULI as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::NOT(ra, rb) => {
                ((OpcodeRepr::NOT as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::OR(ra, rb, rc) => {
                ((OpcodeRepr::OR as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::ORI(ra, rb, imm12) => {
                ((OpcodeRepr::ORI as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::SLL(ra, rb, rc) => {
                ((OpcodeRepr::SLL as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::SLLI(ra, rb, imm12) => {
                ((OpcodeRepr::SLLI as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::SRL(ra, rb, rc) => {
                ((OpcodeRepr::SRL as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::SRLI(ra, rb, imm12) => {
                ((OpcodeRepr::SRLI as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::SUB(ra, rb, rc) => {
                ((OpcodeRepr::SUB as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::SUBI(ra, rb, imm12) => {
                ((OpcodeRepr::SUBI as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::XOR(ra, rb, rc) => {
                ((OpcodeRepr::XOR as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::XORI(ra, rb, imm12) => {
                ((OpcodeRepr::XORI as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::CIMV(ra, rb, rc) => {
                ((OpcodeRepr::CIMV as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::CTMV(ra, rb) => {
                ((OpcodeRepr::CTMV as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::JI(imm24) => ((OpcodeRepr::JI as u32) << 24) | (imm24 as u32),
            Opcode::JNEI(ra, rb, imm12) => {
                ((OpcodeRepr::JNEI as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::RET(ra) => ((OpcodeRepr::RET as u32) << 24) | ((ra as u32) << 18),
            Opcode::RETD(ra, rb) => {
                ((OpcodeRepr::RETD as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::CFEI(imm24) => ((OpcodeRepr::CFEI as u32) << 24) | (imm24 as u32),
            Opcode::CFSI(imm24) => ((OpcodeRepr::CFSI as u32) << 24) | (imm24 as u32),
            Opcode::LB(ra, rb, imm12) => {
                ((OpcodeRepr::LB as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::LW(ra, rb, imm12) => {
                ((OpcodeRepr::LW as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::ALOC(ra) => ((OpcodeRepr::ALOC as u32) << 24) | ((ra as u32) << 18),
            Opcode::MCL(ra, rb) => {
                ((OpcodeRepr::MCL as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::MCLI(ra, imm18) => {
                ((OpcodeRepr::MCLI as u32) << 24) | ((ra as u32) << 18) | (imm18 as u32)
            }
            Opcode::MCP(ra, rb, rc) => {
                ((OpcodeRepr::MCP as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::MCPI(ra, rb, imm12) => {
                ((OpcodeRepr::MCPI as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::MEQ(ra, rb, rc, rd) => {
                ((OpcodeRepr::MEQ as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
                    | (rd as u32)
            }
            Opcode::SB(ra, rb, imm12) => {
                ((OpcodeRepr::SB as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::SW(ra, rb, imm12) => {
                ((OpcodeRepr::SW as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | (imm12 as u32)
            }
            Opcode::BAL(ra, rb, rc) => {
                ((OpcodeRepr::BAL as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::BHSH(ra, rb) => {
                ((OpcodeRepr::BHSH as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::BHEI(ra) => ((OpcodeRepr::BHEI as u32) << 24) | ((ra as u32) << 18),
            Opcode::BURN(ra) => ((OpcodeRepr::BURN as u32) << 24) | ((ra as u32) << 18),
            Opcode::CALL(ra, rb, rc, rd) => {
                ((OpcodeRepr::CALL as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
                    | (rd as u32)
            }
            Opcode::CCP(ra, rb, rc, rd) => {
                ((OpcodeRepr::CCP as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
                    | (rd as u32)
            }
            Opcode::CROO(ra, rb) => {
                ((OpcodeRepr::CROO as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::CSIZ(ra, rb) => {
                ((OpcodeRepr::CSIZ as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::CB(ra) => ((OpcodeRepr::CB as u32) << 24) | ((ra as u32) << 18),
            Opcode::LDC(ra, rb, rc) => {
                ((OpcodeRepr::LDC as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::LOG(ra, rb, rc, rd) => {
                ((OpcodeRepr::LOG as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
                    | (rd as u32)
            }
            Opcode::LOGD(ra, rb, rc, rd) => {
                ((OpcodeRepr::LOGD as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
                    | (rd as u32)
            }
            Opcode::MINT(ra) => ((OpcodeRepr::MINT as u32) << 24) | ((ra as u32) << 18),
            Opcode::RVRT(ra) => ((OpcodeRepr::RVRT as u32) << 24) | ((ra as u32) << 18),
            Opcode::SLDC(ra, rb, rc) => {
                ((OpcodeRepr::SLDC as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::SRW(ra, rb) => {
                ((OpcodeRepr::SRW as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::SRWQ(ra, rb) => {
                ((OpcodeRepr::SRWQ as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::SWW(ra, rb) => {
                ((OpcodeRepr::SWW as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::SWWQ(ra, rb) => {
                ((OpcodeRepr::SWWQ as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::TR(ra, rb, rc) => {
                ((OpcodeRepr::TR as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::TRO(ra, rb, rc, rd) => {
                ((OpcodeRepr::TRO as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
                    | (rd as u32)
            }
            Opcode::ECR(ra, rb, rc) => {
                ((OpcodeRepr::ECR as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::K256(ra, rb, rc) => {
                ((OpcodeRepr::K256 as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::S256(ra, rb, rc) => {
                ((OpcodeRepr::S256 as u32) << 24)
                    | ((ra as u32) << 18)
                    | ((rb as u32) << 12)
                    | ((rc as u32) << 6)
            }
            Opcode::XIL(ra, rb) => {
                ((OpcodeRepr::XIL as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::XIS(ra, rb) => {
                ((OpcodeRepr::XIS as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::XOL(ra, rb) => {
                ((OpcodeRepr::XOL as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::XOS(ra, rb) => {
                ((OpcodeRepr::XOS as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::XWL(ra, rb) => {
                ((OpcodeRepr::XWL as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::XWS(ra, rb) => {
                ((OpcodeRepr::XWS as u32) << 24) | ((ra as u32) << 18) | ((rb as u32) << 12)
            }
            Opcode::NOOP => (OpcodeRepr::NOOP as u32) << 24,
            Opcode::FLAG(ra) => ((OpcodeRepr::FLAG as u32) << 24) | ((ra as u32) << 18),
            Opcode::GM(ra, imm18) => {
                ((OpcodeRepr::GM as u32) << 24) | ((ra as u32) << 18) | (imm18 as u32)
            }
            Opcode::Undefined => (0x00 << 24),
        }
    }
}

#[cfg(feature = "std")]
impl io::Read for Opcode {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        buf.chunks_exact_mut(Opcode::LEN)
            .next()
            .map(|chunk| chunk.copy_from_slice(&u32::from(*self).to_be_bytes()))
            .map(|_| Opcode::LEN)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "The provided buffer is not big enough!",
                )
            })
    }
}

#[cfg(feature = "std")]
impl io::Write for Opcode {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Safety: checked length
        buf.chunks_exact(Opcode::LEN)
            .next()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "The provided buffer is not big enough!",
                )
            })
            .map(|bytes| *self = unsafe { Self::from_bytes_unchecked(bytes) })
            .map(|_| Opcode::LEN)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(feature = "std")]
impl iter::FromIterator<Opcode> for Vec<u8> {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Opcode>,
    {
        iter.into_iter().map(Opcode::to_bytes).flatten().collect()
    }
}

#[cfg(feature = "std")]
impl iter::FromIterator<Instruction> for Vec<Opcode> {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Instruction>,
    {
        iter.into_iter().map(Opcode::from).collect()
    }
}
