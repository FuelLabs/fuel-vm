//! FuelVM instruction and opcodes representation.

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "std", doc = include_str!("../README.md"))]
#![warn(missing_docs)]
#![warn(unsafe_code)]
#![deny(unused_crate_dependencies)]

mod args;
mod panic_instruction;
// This is `pub` to make documentation for the private `impl_instructions!` macro more
// accessible.
#[macro_use]
pub mod macros;
pub mod op;
mod pack;
mod panic_reason;
mod unpack;

#[cfg(test)]
mod encoding_tests;

#[doc(no_inline)]
pub use args::{
    wideint,
    GMArgs,
    GTFArgs,
};
pub use fuel_types::{
    RegisterId,
    Word,
};
pub use panic_instruction::PanicInstruction;
pub use panic_reason::PanicReason;

/// Represents a 6-bit register ID, guaranteed to be masked by construction.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RegId(u8);

/// Represents a 6-bit immediate value, guaranteed to be masked by construction.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Imm06(u8);

/// Represents a 12-bit immediate value, guaranteed to be masked by construction.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Imm12(u16);

/// Represents a 18-bit immediate value, guaranteed to be masked by construction.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Imm18(u32);

/// Represents a 24-bit immediate value, guaranteed to be masked by construction.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Imm24(u32);

/// An instruction in its raw, packed, unparsed representation.
pub type RawInstruction = u32;

/// Failed to parse a `u8` as a valid or non-reserved opcode.
#[derive(Debug, Eq, PartialEq)]
pub struct InvalidOpcode;

bitflags::bitflags! {
    /// Possible values for the FLAG instruction.
    /// See https://github.com/FuelLabs/fuel-specs/blob/master/src/vm/index.md#flags
    pub struct Flags: Word {
        /// If set, arithmetic errors result in setting $err instead of panicking.
        /// This includes cases where result of a computation is undefined, like
        /// division by zero. Arithmetic overflows still cause a panic, but that be
        /// controlled with [`Flags::WRAPPING`].
        const UNSAFEMATH = 0x01;
        /// If set, arithmetic overflows result in setting $of instead of panicking.
        const WRAPPING = 0x02;
    }
}
/// Type is convertible to a [`RegId`]
pub trait CheckRegId {
    /// Convert to a [`RegId`], or panic
    fn check(self) -> RegId;
}

impl CheckRegId for RegId {
    fn check(self) -> RegId {
        self
    }
}

impl CheckRegId for u8 {
    fn check(self) -> RegId {
        RegId::new_checked(self).expect("CheckRegId was given invalid RegId")
    }
}

// Defines the `Instruction` and `Opcode` types, along with an `op` module declaring a
// unique type for each opcode's instruction variant. For a detailed explanation of how
// this works, see the `fuel_asm::macros` module level documentation.
impl_instructions! {
    "Adds two registers."
    0x10 ADD add [RegId RegId RegId]
    "Bitwise ANDs two registers."
    0x11 AND and [RegId RegId RegId]
    "Divides two registers."
    0x12 DIV div [RegId RegId RegId]
    "Compares two registers for equality."
    0x13 EQ eq [RegId RegId RegId]
    "Raises one register to the power of another."
    0x14 EXP exp [RegId RegId RegId]
    "Compares two registers for greater-than."
    0x15 GT gt [RegId RegId RegId]
    "Compares two registers for less-than."
    0x16 LT lt [RegId RegId RegId]
    "The integer logarithm of a register."
    0x17 MLOG mlog [RegId RegId RegId]
    "The integer root of a register."
    0x18 MROO mroo [RegId RegId RegId]
    "Modulo remainder of two registers."
    0x19 MOD mod_ [RegId RegId RegId]
    "Copy from one register to another."
    0x1A MOVE move_ [RegId RegId]
    "Multiplies two registers."
    0x1B MUL mul [RegId RegId RegId]
    "Bitwise NOT a register."
    0x1C NOT not [RegId RegId]
    "Bitwise ORs two registers."
    0x1D OR or [RegId RegId RegId]
    "Left shifts a register by a register."
    0x1E SLL sll [RegId RegId RegId]
    "Right shifts a register by a register."
    0x1F SRL srl [RegId RegId RegId]
    "Subtracts two registers."
    0x20 SUB sub [RegId RegId RegId]
    "Bitwise XORs two registers."
    0x21 XOR xor [RegId RegId RegId]
    "Fused multiply-divide with arbitrary precision intermediate step."
    0x22 MLDV mldv [RegId RegId RegId RegId]

    "Return from context."
    0x24 RET ret [RegId]
    "Return from context with data."
    0x25 RETD retd [RegId RegId]
    "Allocate a number of bytes from the heap."
    0x26 ALOC aloc [RegId]
    "Clear a variable number of bytes in memory."
    0x27 MCL mcl [RegId RegId]
    "Copy a variable number of bytes in memory."
    0x28 MCP mcp [RegId RegId RegId]
    "Compare bytes in memory."
    0x29 MEQ meq [RegId RegId RegId RegId]
    "Get block header hash for height."
    0x2A BHSH bhsh [RegId RegId]
    "Get current block height."
    0x2B BHEI bhei [RegId]
    "Burn coins of the current contract's asset ID."
    0x2C BURN burn [RegId]
    "Call a contract."
    0x2D CALL call [RegId RegId RegId RegId]
    "Copy contract code for a contract."
    0x2E CCP ccp [RegId RegId RegId RegId]
    "Get code root of a contract."
    0x2F CROO croo [RegId RegId]
    "Get code size of a contract."
    0x30 CSIZ csiz [RegId RegId]
    "Get current block proposer's address."
    0x31 CB cb [RegId]
    "Load a contract's code as executable."
    0x32 LDC ldc [RegId RegId RegId]
    "Log an event."
    0x33 LOG log [RegId RegId RegId RegId]
    "Log data."
    0x34 LOGD logd [RegId RegId RegId RegId]
    "Mint coins of the current contract's asset ID."
    0x35 MINT mint [RegId]
    "Halt execution, reverting state changes and returning a value."
    0x36 RVRT rvrt [RegId]
    "Clear a series of slots from contract storage."
    0x37 SCWQ scwq [RegId RegId RegId]
    "Load a word from contract storage."
    0x38 SRW srw [RegId RegId RegId]
    "Load a series of 32 byte slots from contract storage."
    0x39 SRWQ srwq [RegId RegId RegId RegId]
    "Store a word in contract storage."
    0x3A SWW sww [RegId RegId RegId]
    "Store a series of 32 byte slots in contract storage."
    0x3B SWWQ swwq [RegId RegId RegId RegId]
    "Transfer coins to a contract unconditionally."
    0x3C TR tr [RegId RegId RegId]
    "Transfer coins to a variable output."
    0x3D TRO tro [RegId RegId RegId RegId]
    "The 64-byte public key (x, y) recovered from 64-byte signature on 32-byte message."
    0x3E ECR ecr [RegId RegId RegId]
    "The keccak-256 hash of a slice."
    0x3F K256 k256 [RegId RegId RegId]
    "The SHA-2-256 hash of a slice."
    0x40 S256 s256 [RegId RegId RegId]
    "Get timestamp of block at given height."
    0x41 TIME time [RegId RegId]

    "Performs no operation."
    0x47 NOOP noop []
    "Set flag register to a register."
    0x48 FLAG flag [RegId]
    "Get the balance of contract of an asset ID."
    0x49 BAL bal [RegId RegId RegId]
    "Dynamic jump."
    0x4A JMP jmp [RegId]
    "Conditional dynamic jump."
    0x4B JNE jne [RegId RegId RegId]
    "Send a message to recipient address with call abi, coins, and output."
    0x4C SMO smo [RegId RegId RegId RegId]

    "Adds a register and an immediate value."
    0x50 ADDI addi [RegId RegId Imm12]
    "Bitwise ANDs a register and an immediate value."
    0x51 ANDI andi [RegId RegId Imm12]
    "Divides a register and an immediate value."
    0x52 DIVI divi [RegId RegId Imm12]
    "Raises one register to the power of an immediate value."
    0x53 EXPI expi [RegId RegId Imm12]
    "Modulo remainder of a register and an immediate value."
    0x54 MODI modi [RegId RegId Imm12]
    "Multiplies a register and an immediate value."
    0x55 MULI muli [RegId RegId Imm12]
    "Bitwise ORs a register and an immediate value."
    0x56 ORI ori [RegId RegId Imm12]
    "Left shifts a register by an immediate value."
    0x57 SLLI slli [RegId RegId Imm12]
    "Right shifts a register by an immediate value."
    0x58 SRLI srli [RegId RegId Imm12]
    "Subtracts a register and an immediate value."
    0x59 SUBI subi [RegId RegId Imm12]
    "Bitwise XORs a register and an immediate value."
    0x5A XORI xori [RegId RegId Imm12]
    "Conditional jump."
    0x5B JNEI jnei [RegId RegId Imm12]
    "A byte is loaded from the specified address offset by an immediate value."
    0x5C LB lb [RegId RegId Imm12]
    "A word is loaded from the specified address offset by an immediate value."
    0x5D LW lw [RegId RegId Imm12]
    "Write the least significant byte of a register to memory."
    0x5E SB sb [RegId RegId Imm12]
    "Write a register to memory."
    0x5F SW sw [RegId RegId Imm12]
    "Copy an immediate number of bytes in memory."
    0x60 MCPI mcpi [RegId RegId Imm12]
    "Get transaction fields."
    0x61 GTF gtf [RegId RegId Imm12]

    "Clear an immediate number of bytes in memory."
    0x70 MCLI mcli [RegId Imm18]
    "Get metadata from memory."
    0x71 GM gm [RegId Imm18]
    "Copy immediate value into a register"
    0x72 MOVI movi [RegId Imm18]
    "Conditional jump against zero."
    0x73 JNZI jnzi [RegId Imm18]
    "Unconditional dynamic relative jump forwards, with a constant offset."
    0x74 JMPF jmpf [RegId Imm18]
    "Unconditional dynamic relative jump backwards, with a constant offset."
    0x75 JMPB jmpb [RegId Imm18]
    "Dynamic relative jump forwards, conditional against zero, with a constant offset."
    0x76 JNZF jnzf [RegId RegId Imm12]
    "Dynamic relative jump backwards, conditional against zero, with a constant offset."
    0x77 JNZB jnzb [RegId RegId Imm12]
    "Dynamic relative jump forwards, conditional on comparsion, with a constant offset."
    0x78 JNEF jnef [RegId RegId RegId Imm06]
    "Dynamic relative jump backwards, conditional on comparsion, with a constant offset."
    0x79 JNEB jneb [RegId RegId RegId Imm06]

    "Jump."
    0x90 JI ji [Imm24]
    "Extend the current call frame's stack by an immediate value."
    0x91 CFEI cfei [Imm24]
    "Shrink the current call frame's stack by an immediate value."
    0x92 CFSI cfsi [Imm24]
    "Extend the current call frame's stack"
    0x93 CFE cfe [RegId]
    "Shrink the current call frame's stack"
    0x94 CFS cfs [RegId]

    "Compare 128bit integers"
    0xa0 WDCM wdcm [RegId RegId RegId Imm06]
    "Compare 256bit integers"
    0xa1 WQCM wqcm [RegId RegId RegId Imm06]
    "Simple 128bit operations"
    0xa2 WDOP wdop [RegId RegId RegId Imm06]
    "Simple 256bit operations"
    0xa3 WQOP wqop [RegId RegId RegId Imm06]
    "Multiply 128bit"
    0xa4 WDML wdml [RegId RegId RegId Imm06]
    "Multiply 256bit"
    0xa5 WQML wqml [RegId RegId RegId Imm06]
    "Divide 128bit"
    0xa6 WDDV wddv [RegId RegId RegId Imm06]
    "Divide 256bit"
    0xa7 WQDV wqdv [RegId RegId RegId Imm06]
    "Fused multiply-divide 128bit"
    0xa8 WDMD wdmd [RegId RegId RegId RegId]
    "Fused multiply-divide 256bit"
    0xa9 WQMD wqmd [RegId RegId RegId RegId]
    "AddMod 128bit"
    0xaa WDAM wdam [RegId RegId RegId RegId]
    "AddMod 256bit"
    0xab WQAM wqam [RegId RegId RegId RegId]
    "MulMod 128bit"
    0xac WDMM wdmm [RegId RegId RegId RegId]
    "MulMod 256bit"
    0xad WQMM wqmm [RegId RegId RegId RegId]
}

impl Instruction {
    /// Size of an instruction in bytes
    pub const SIZE: usize = core::mem::size_of::<Instruction>();

    /// Convenience method for converting to bytes
    pub fn to_bytes(self) -> [u8; 4] {
        self.into()
    }
}

impl RegId {
    /// Received balance for this context.
    pub const BAL: Self = Self(0x0B);
    /// Remaining gas in the context.
    pub const CGAS: Self = Self(0x0A);
    /// Error codes for particular operations.
    pub const ERR: Self = Self(0x08);
    /// Flags register.
    pub const FLAG: Self = Self(0x0F);
    /// Frame pointer. Memory address of beginning of current call frame.
    pub const FP: Self = Self(0x06);
    /// Remaining gas globally.
    pub const GGAS: Self = Self(0x09);
    /// Heap pointer. Memory address below the current bottom of the heap (points to free
    /// memory).
    pub const HP: Self = Self(0x07);
    /// Instructions start. Pointer to the start of the currently-executing code.
    pub const IS: Self = Self(0x0C);
    /// Contains overflow/underflow of addition, subtraction, and multiplication.
    pub const OF: Self = Self(0x02);
    /// Contains one (1), for convenience.
    pub const ONE: Self = Self(0x01);
    /// The program counter. Memory address of the current instruction.
    pub const PC: Self = Self(0x03);
    /// Return value or pointer.
    pub const RET: Self = Self(0x0D);
    /// Return value length in bytes.
    pub const RETL: Self = Self(0x0E);
    /// Stack pointer. Memory address on top of current writable stack area (points to
    /// free memory).
    pub const SP: Self = Self(0x05);
    /// Stack start pointer. Memory address of bottom of current writable stack area.
    pub const SSP: Self = Self(0x04);
    /// Smallest writable register.
    pub const WRITABLE: Self = Self(0x10);
    /// Contains zero (0), for convenience.
    pub const ZERO: Self = Self(0x00);

    /// Construct a register ID from the given value.
    ///
    /// The given value will be masked to 6 bits.
    pub const fn new(u: u8) -> Self {
        Self(u & 0b_0011_1111)
    }

    /// Construct a register ID from the given value.
    ///
    /// Returns `None` if the value is outside the 6-bit value range.
    pub fn new_checked(u: u8) -> Option<Self> {
        let r = Self::new(u);
        (r.0 == u).then_some(r)
    }

    /// A const alternative to the `Into<u8>` implementation.
    pub const fn to_u8(self) -> u8 {
        self.0
    }
}

impl Imm06 {
    /// Max value for the type
    pub const MAX: Self = Self(0b_0011_1111);

    /// Construct an immediate value.
    ///
    /// The given value will be masked to 6 bits.
    pub const fn new(u: u8) -> Self {
        Self(u & Self::MAX.0)
    }

    /// Construct an immediate value.
    ///
    /// Returns `None` if the value is outside the 6-bit value range.
    pub fn new_checked(u: u8) -> Option<Self> {
        let imm = Self::new(u);
        (imm.0 == u).then_some(imm)
    }

    /// A const alternative to the `Into<u8>` implementation.
    pub const fn to_u8(self) -> u8 {
        self.0
    }
}

impl Imm12 {
    /// Max value for the type
    pub const MAX: Self = Self(0b_0000_1111_1111_1111);

    /// Construct an immediate value.
    ///
    /// The given value will be masked to 12 bits.
    pub const fn new(u: u16) -> Self {
        Self(u & Self::MAX.0)
    }

    /// Construct an immediate value.
    ///
    /// Returns `None` if the value is outside the 12-bit value range.
    pub fn new_checked(u: u16) -> Option<Self> {
        let imm = Self::new(u);
        (imm.0 == u).then_some(imm)
    }

    /// A const alternative to the `Into<u16>` implementation.
    pub const fn to_u16(self) -> u16 {
        self.0
    }
}

impl Imm18 {
    /// Max value for the type
    pub const MAX: Self = Self(0b_0000_0000_0000_0011_1111_1111_1111_1111);

    /// Construct an immediate value.
    ///
    /// The given value will be masked to 18 bits.
    pub const fn new(u: u32) -> Self {
        Self(u & Self::MAX.0)
    }

    /// Construct an immediate value.
    ///
    /// Returns `None` if the value is outside the 18-bit value range.
    pub fn new_checked(u: u32) -> Option<Self> {
        let imm = Self::new(u);
        (imm.0 == u).then_some(imm)
    }

    /// A const alternative to the `Into<u32>` implementation.
    pub const fn to_u32(self) -> u32 {
        self.0
    }
}

impl Imm24 {
    /// Max value for the type
    pub const MAX: Self = Self(0b_0000_0000_1111_1111_1111_1111_1111_1111);

    /// Construct an immediate value.
    ///
    /// The given value will be masked to 24 bits.
    pub const fn new(u: u32) -> Self {
        Self(u & Self::MAX.0)
    }

    /// Construct an immediate value.
    ///
    /// Returns `None` if the value is outside the 24-bit value range.
    pub fn new_checked(u: u32) -> Option<Self> {
        let imm = Self::new(u);
        (imm.0 == u).then_some(imm)
    }

    /// A const alternative to the `Into<u32>` implementation.
    pub const fn to_u32(self) -> u32 {
        self.0
    }
}

impl Opcode {
    /// Check if the opcode is allowed for predicates.
    ///
    /// <https://github.com/FuelLabs/fuel-specs/blob/master/specs/vm/main.md#predicate-verification>
    /// <https://github.com/FuelLabs/fuel-specs/blob/master/specs/vm/opcodes.md#contract-opcodes>
    #[allow(clippy::match_like_matches_macro)]
    pub fn is_predicate_allowed(&self) -> bool {
        use Opcode::*;
        match self {
            ADD | AND | DIV | EQ | EXP | GT | LT | MLOG | MROO | MOD | MOVE | MUL
            | NOT | OR | SLL | SRL | SUB | XOR | WDCM | WQCM | WDOP | WQOP | WDML
            | WQML | WDDV | WQDV | WDMD | WQMD | WDAM | WQAM | WDMM | WQMM | RET
            | ALOC | MCL | MCP | MEQ | ECR | K256 | S256 | NOOP | FLAG | ADDI | ANDI
            | DIVI | EXPI | MODI | MULI | MLDV | ORI | SLLI | SRLI | SUBI | XORI
            | JNEI | LB | LW | SB | SW | MCPI | MCLI | GM | MOVI | JNZI | JI | JMP
            | JNE | JMPF | JMPB | JNZF | JNZB | JNEF | JNEB | CFEI | CFSI | CFE | CFS
            | GTF => true,
            _ => false,
        }
    }
}

// Direct conversions

impl From<u8> for RegId {
    fn from(u: u8) -> Self {
        RegId::new(u)
    }
}

impl From<u8> for Imm06 {
    fn from(u: u8) -> Self {
        Imm06::new(u)
    }
}

impl From<u16> for Imm12 {
    fn from(u: u16) -> Self {
        Imm12::new(u)
    }
}

impl From<u32> for Imm18 {
    fn from(u: u32) -> Self {
        Imm18::new(u)
    }
}

impl From<u32> for Imm24 {
    fn from(u: u32) -> Self {
        Imm24::new(u)
    }
}

impl From<RegId> for u8 {
    fn from(RegId(u): RegId) -> Self {
        u
    }
}

impl From<Imm06> for u8 {
    fn from(Imm06(u): Imm06) -> Self {
        u
    }
}

impl From<Imm12> for u16 {
    fn from(Imm12(u): Imm12) -> Self {
        u
    }
}

impl From<Imm18> for u32 {
    fn from(Imm18(u): Imm18) -> Self {
        u
    }
}

impl From<Imm24> for u32 {
    fn from(Imm24(u): Imm24) -> Self {
        u
    }
}

// Lossless, convenience conversions

impl From<RegId> for usize {
    fn from(r: RegId) -> usize {
        u8::from(r).into()
    }
}

impl From<Imm06> for u16 {
    fn from(imm: Imm06) -> Self {
        u8::from(imm).into()
    }
}

impl From<Imm06> for u32 {
    fn from(imm: Imm06) -> Self {
        u8::from(imm).into()
    }
}

impl From<Imm06> for u64 {
    fn from(imm: Imm06) -> Self {
        u8::from(imm).into()
    }
}

impl From<Imm06> for u128 {
    fn from(imm: Imm06) -> Self {
        u8::from(imm).into()
    }
}

impl From<Imm12> for u32 {
    fn from(imm: Imm12) -> Self {
        u16::from(imm).into()
    }
}

impl From<Imm12> for u64 {
    fn from(imm: Imm12) -> Self {
        u16::from(imm).into()
    }
}

impl From<Imm12> for u128 {
    fn from(imm: Imm12) -> Self {
        u16::from(imm).into()
    }
}

impl From<Imm18> for u64 {
    fn from(imm: Imm18) -> Self {
        u32::from(imm).into()
    }
}

impl From<Imm18> for u128 {
    fn from(imm: Imm18) -> Self {
        u32::from(imm).into()
    }
}

impl From<Imm24> for u64 {
    fn from(imm: Imm24) -> Self {
        u32::from(imm).into()
    }
}

impl From<Imm24> for u128 {
    fn from(imm: Imm24) -> Self {
        u32::from(imm).into()
    }
}

impl From<Opcode> for u8 {
    fn from(op: Opcode) -> Self {
        op as u8
    }
}

impl From<Instruction> for RawInstruction {
    fn from(inst: Instruction) -> Self {
        RawInstruction::from_be_bytes(inst.into())
    }
}

impl core::convert::TryFrom<RawInstruction> for Instruction {
    type Error = InvalidOpcode;

    fn try_from(u: RawInstruction) -> Result<Self, Self::Error> {
        Self::try_from(u.to_be_bytes())
    }
}

// Index slices with `RegId`

impl<T> core::ops::Index<RegId> for [T]
where
    [T]: core::ops::Index<usize, Output = T>,
{
    type Output = T;

    fn index(&self, ix: RegId) -> &Self::Output {
        &self[usize::from(ix)]
    }
}

impl<T> core::ops::IndexMut<RegId> for [T]
where
    [T]: core::ops::IndexMut<usize, Output = T>,
{
    fn index_mut(&mut self, ix: RegId) -> &mut Self::Output {
        &mut self[usize::from(ix)]
    }
}

// Collect instructions into bytes or halfwords

#[cfg(feature = "std")]
impl core::iter::FromIterator<Instruction> for Vec<u8> {
    fn from_iter<I: IntoIterator<Item = Instruction>>(iter: I) -> Self {
        iter.into_iter().flat_map(Instruction::to_bytes).collect()
    }
}

#[cfg(feature = "std")]
impl core::iter::FromIterator<Instruction> for Vec<u32> {
    fn from_iter<I: IntoIterator<Item = Instruction>>(iter: I) -> Self {
        iter.into_iter().map(u32::from).collect()
    }
}

/// Produce two raw instructions from a word's hi and lo parts.
pub fn raw_instructions_from_word(word: Word) -> [RawInstruction; 2] {
    let hi = (word >> 32) as RawInstruction;
    let lo = word as RawInstruction;
    [hi, lo]
}

/// Given an iterator yielding bytes, produces an iterator yielding `Instruction`s.
///
/// This function assumes each consecutive 4 bytes aligns with an instruction.
///
/// The produced iterator yields an `Err` in the case that an instruction fails to parse
/// from 4 consecutive bytes.
pub fn from_bytes<I>(bs: I) -> impl Iterator<Item = Result<Instruction, InvalidOpcode>>
where
    I: IntoIterator<Item = u8>,
{
    let mut iter = bs.into_iter();
    core::iter::from_fn(move || {
        let a = iter.next()?;
        let b = iter.next()?;
        let c = iter.next()?;
        let d = iter.next()?;
        Some(Instruction::try_from([a, b, c, d]))
    })
}

/// Given an iterator yielding u32s (i.e. "half words" or "raw instructions"), produces an
/// iterator yielding `Instruction`s.
///
/// This function assumes each consecutive 4 bytes aligns with an instruction.
///
/// The produced iterator yields an `Err` in the case that an instruction fails to parse.
pub fn from_u32s<I>(us: I) -> impl Iterator<Item = Result<Instruction, InvalidOpcode>>
where
    I: IntoIterator<Item = u32>,
{
    us.into_iter().map(Instruction::try_from)
}

// Short-hand, `panic!`ing constructors for the short-hand instruction construtors (e.g
// op::add).

fn check_imm06(u: u8) -> Imm06 {
    Imm06::new_checked(u)
        .unwrap_or_else(|| panic!("Value `{u}` out of range for 6-bit immediate"))
}

fn check_imm12(u: u16) -> Imm12 {
    Imm12::new_checked(u)
        .unwrap_or_else(|| panic!("Value `{u}` out of range for 12-bit immediate"))
}

fn check_imm18(u: u32) -> Imm18 {
    Imm18::new_checked(u)
        .unwrap_or_else(|| panic!("Value `{u}` out of range for 18-bit immediate"))
}

fn check_imm24(u: u32) -> Imm24 {
    Imm24::new_checked(u)
        .unwrap_or_else(|| panic!("Value `{u}` out of range for 24-bit immediate"))
}

// --------------------------------------------------------

// The size of the instruction isn't larger than necessary.
// 1 byte for the opcode, 3 bytes for registers and immediates.
#[test]
fn test_instruction_size() {
    // NOTE: Throughout `fuel-vm`, we use the `Instruction::SIZE` associated
    // const to refer to offsets within raw instruction data. As a result, it's
    // *essential* that this equivalence remains the same. If you've added
    // a new field or changed the size of `Instruction` somehow and have
    // arrived at this assertion, ensure that you also revisit all sites where
    // `Instruction::SIZE` is used and make sure we're using the right value
    // (in most cases, the right value is `core::mem::size_of::<RawInstruction>()`).
    assert_eq!(
        core::mem::size_of::<Instruction>(),
        core::mem::size_of::<RawInstruction>()
    );

    assert_eq!(core::mem::size_of::<Instruction>(), Instruction::SIZE);
}

// The size of the opcode is exactly one byte.
#[test]
fn test_opcode_size() {
    assert_eq!(core::mem::size_of::<Opcode>(), 1);
}

#[test]
#[allow(clippy::match_like_matches_macro)]
fn check_predicate_allowed() {
    use Opcode::*;
    for byte in 0..u8::MAX {
        if let Ok(repr) = Opcode::try_from(byte) {
            let should_allow = match repr {
                BAL | BHEI | BHSH | BURN | CALL | CB | CCP | CROO | CSIZ | LDC | LOG
                | LOGD | MINT | RETD | RVRT | SMO | SCWQ | SRW | SRWQ | SWW | SWWQ
                | TIME | TR | TRO => false,
                _ => true,
            };
            assert_eq!(should_allow, repr.is_predicate_allowed());
        }
    }
}

// Test roundtrip conversion for all valid opcodes.
#[test]
fn test_opcode_u8_conv() {
    for u in 0..=u8::MAX {
        if let Ok(op) = Opcode::try_from(u) {
            assert_eq!(op as u8, u);
        }
    }
}
