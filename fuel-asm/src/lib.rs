//! FuelVM instruction and opcodes representation.

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "std", doc = include_str!("../README.md"))]
#![warn(missing_docs)]

mod gm_args;
mod instruction_result;
#[macro_use]
pub mod macros;
mod panic_reason;

pub use fuel_types::{RegisterId, Word};
#[doc(no_inline)]
pub use gm_args::{GMArgs, GTFArgs};
pub use instruction_result::InstructionResult;
pub use panic_reason::PanicReason;

// Defines the `Instruction` and `Opcode` types, along with an `op` module declaring a unique type
// for each opcode's instruction variant. For a detailed explanation of how this works, see the
// `fuel_asm::macros` module level documentation.
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

    "Jump."
    0x90 JI ji [Imm24]
    "Extend the current call frame's stack by an immediate value."
    0x91 CFEI cfei [Imm24]
    "Shrink the current call frame's stack by an immediate value."
    0x92 CFSI cfsi [Imm24]
}

/// Represents a 6-bit register ID, guaranteed to be masked by construction.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RegId(u8);

/// Represents a 12-bit immediate value, guaranteed to be masked by construction.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Imm12(u16);

/// Represents a 18-bit immediate value, guaranteed to be masked by construction.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Imm18(u32);

/// Represents a 24-bit immediate value, guaranteed to be masked by construction.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Imm24(u32);

/// An instruction in its raw, unparsed representation.
pub type RawInstruction = u32;

/// Failed to parse a `u8` as a valid or non-reserved opcode.
#[derive(Debug, Eq, PartialEq)]
pub struct InvalidOpcode;

impl RegId {
    /// Construct a register ID from the given value.
    ///
    /// The given value will be masked to 6 bits.
    pub fn new(u: u8) -> Self {
        Self(u & 0b_0011_1111)
    }
}

impl Imm12 {
    /// Construct an immediate value.
    ///
    /// The given value will be masked to 12 bits.
    pub fn new(u: u16) -> Self {
        Self(u & 0b_0000_1111_1111_1111)
    }
}

impl Imm18 {
    /// Construct an immediate value.
    ///
    /// The given value will be masked to 18 bits.
    pub fn new(u: u32) -> Self {
        Self(u & 0b_0000_0000_0000_0011_1111_1111_1111_1111)
    }
}

impl Imm24 {
    /// Construct an immediate value.
    ///
    /// The given value will be masked to 24 bits.
    pub fn new(u: u32) -> Self {
        Self(u & 0b_0000_0000_1111_1111_1111_1111_1111_1111)
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
            ADD | AND | DIV | EQ | EXP | GT | LT | MLOG | MROO | MOD | MOVE | MUL | NOT | OR
            | SLL | SRL | SUB | XOR | RET | ALOC | MCL | MCP | MEQ | ECR | K256 | S256 | NOOP
            | FLAG | ADDI | ANDI | DIVI | EXPI | MODI | MULI | ORI | SLLI | SRLI | SUBI | XORI
            | JNEI | LB | LW | SB | SW | MCPI | MCLI | GM | MOVI | JNZI | JI | JMP | JNE | CFEI
            | CFSI | GTF => true,
            _ => false,
        }
    }
}

impl op::GM {
    /// Construct a `GM` instruction from its arguments.
    pub fn from_args(ra: RegId, args: GMArgs) -> Self {
        Self::new(ra, Imm18::new(args as _))
    }
}

impl op::GTF {
    /// Construct a `GTF` instruction from its arguments.
    pub fn from_args(ra: RegId, rb: RegId, args: GTFArgs) -> Self {
        Self::new(ra, rb, Imm12::new(args as _))
    }
}

impl Instruction {
    /// Construct a `GM` instruction from its arguments.
    pub fn gm(ra: u8, args: GMArgs) -> Self {
        Self::GM(op::GM::from_args(RegId::from(ra), args))
    }

    /// Construct a `GM` instruction from its arguments.
    pub fn gtf(ra: u8, rb: u8, args: GTFArgs) -> Self {
        Self::GTF(op::GTF::from_args(RegId::from(ra), RegId::from(rb), args))
    }
}

// Direct conversions

impl From<u8> for RegId {
    fn from(u: u8) -> Self {
        RegId::new(u)
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

// --------------------------------------------------------

impl core::iter::FromIterator<Instruction> for Vec<u8> {
    fn from_iter<I: IntoIterator<Item = Instruction>>(iter: I) -> Self {
        iter.into_iter().flat_map(<[u8; 4]>::from).collect()
    }
}

impl core::iter::FromIterator<Instruction> for Vec<u32> {
    fn from_iter<I: IntoIterator<Item = Instruction>>(iter: I) -> Self {
        iter.into_iter().map(u32::from).collect()
    }
}

// --------------------------------------------------------

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
/// The produced iterator yields an `Err` in the case that an instruction fails to parse from 4
/// consecutive bytes.
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

/// Given an iterator yielding u32s (i.e. "half words" or "raw instructions"), produces an iterator
/// yielding `Instruction`s.
///
/// This function assumes each consecutive 4 bytes aligns with an instruction.
///
/// The produced iterator yields an `Err` in the case that an instruction fails to parse.
pub fn from_u32s<I>(us: I) -> impl Iterator<Item = Result<Instruction, InvalidOpcode>>
where
    I: IntoIterator<Item = u32>,
{
    us.into_iter().map(|u| Instruction::try_from(u))
}

// --------------------------------------------------------

fn ra_from_u32(u: u32) -> RegId {
    RegId::new((u >> 18) as u8)
}

fn rb_from_u32(u: u32) -> RegId {
    RegId::new((u >> 12) as u8)
}

fn rc_from_u32(u: u32) -> RegId {
    RegId::new((u >> 6) as u8)
}

fn rd_from_u32(u: u32) -> RegId {
    RegId::new(u as u8)
}

fn imm12_from_u32(u: u32) -> Imm12 {
    Imm12::new(u as u16)
}

fn imm18_from_u32(u: u32) -> Imm18 {
    Imm18::new(u)
}

fn imm24_from_u32(u: u32) -> Imm24 {
    Imm24::new(u)
}

// -----------------------------------------------------

fn ra_from_bytes(bs: [u8; 3]) -> RegId {
    ra_from_u32(u32::from_be_bytes(u8x4_from_u8x3(bs)))
}

fn rb_from_bytes(bs: [u8; 3]) -> RegId {
    rb_from_u32(u32::from_be_bytes(u8x4_from_u8x3(bs)))
}

fn rc_from_bytes(bs: [u8; 3]) -> RegId {
    rc_from_u32(u32::from_be_bytes(u8x4_from_u8x3(bs)))
}

fn rd_from_bytes(bs: [u8; 3]) -> RegId {
    rd_from_u32(u32::from_be_bytes(u8x4_from_u8x3(bs)))
}

fn imm12_from_bytes(bs: [u8; 3]) -> Imm12 {
    imm12_from_u32(u32::from_be_bytes(u8x4_from_u8x3(bs)))
}

fn imm18_from_bytes(bs: [u8; 3]) -> Imm18 {
    imm18_from_u32(u32::from_be_bytes(u8x4_from_u8x3(bs)))
}

fn imm24_from_bytes(bs: [u8; 3]) -> Imm24 {
    imm24_from_u32(u32::from_be_bytes(u8x4_from_u8x3(bs)))
}

fn ra_rb_from_bytes(bs: [u8; 3]) -> (RegId, RegId) {
    (ra_from_bytes(bs), rb_from_bytes(bs))
}

fn ra_rb_rc_from_bytes(bs: [u8; 3]) -> (RegId, RegId, RegId) {
    (ra_from_bytes(bs), rb_from_bytes(bs), rc_from_bytes(bs))
}

fn ra_rb_rc_rd_from_bytes(bs: [u8; 3]) -> (RegId, RegId, RegId, RegId) {
    (ra_from_bytes(bs), rb_from_bytes(bs), rc_from_bytes(bs), rd_from_bytes(bs))
}

fn ra_rb_imm12_from_bytes(bs: [u8; 3]) -> (RegId, RegId, Imm12) {
    (ra_from_bytes(bs), rb_from_bytes(bs), imm12_from_bytes(bs))
}

fn ra_imm18_from_bytes(bs: [u8; 3]) -> (RegId, Imm18) {
    (ra_from_bytes(bs), imm18_from_bytes(bs))
}

// -----------------------------------------------------

fn u32_from_ra(r: RegId) -> u32 {
    (r.0 as u32) << 18
}

fn u32_from_rb(r: RegId) -> u32 {
    (r.0 as u32) << 12
}

fn u32_from_rc(r: RegId) -> u32 {
    (r.0 as u32) << 6
}

fn u32_from_rd(r: RegId) -> u32 {
    r.0 as u32
}

fn u32_from_imm12(imm: Imm12) -> u32 {
    imm.0 as u32
}

fn u32_from_imm18(imm: Imm18) -> u32 {
    imm.0
}

fn u32_from_imm24(imm: Imm24) -> u32 {
    imm.0
}

fn u32_from_ra_rb(ra: RegId, rb: RegId) -> u32 {
    u32_from_ra(ra) | u32_from_rb(rb)
}

fn u32_from_ra_rb_rc(ra: RegId, rb: RegId, rc: RegId) -> u32 {
    u32_from_ra_rb(ra, rb) | u32_from_rc(rc)
}

fn u32_from_ra_rb_rc_rd(ra: RegId, rb: RegId, rc: RegId, rd: RegId) -> u32 {
    u32_from_ra_rb_rc(ra, rb, rc) | u32_from_rd(rd)
}

fn u32_from_ra_rb_imm12(ra: RegId, rb: RegId, imm: Imm12) -> u32 {
    u32_from_ra_rb(ra, rb) | u32_from_imm12(imm)
}

fn u32_from_ra_imm18(ra: RegId, imm: Imm18) -> u32 {
    u32_from_ra(ra) | u32_from_imm18(imm)
}

// --------------------------------------------------------

fn bytes_from_ra(ra: RegId) -> [u8; 3] {
    u8x3_from_u8x4(u32_from_ra(ra).to_be_bytes())
}

fn bytes_from_ra_rb(ra: RegId, rb: RegId) -> [u8; 3] {
    u8x3_from_u8x4(u32_from_ra_rb(ra, rb).to_be_bytes())
}

fn bytes_from_ra_rb_rc(ra: RegId, rb: RegId, rc: RegId) -> [u8; 3] {
    u8x3_from_u8x4(u32_from_ra_rb_rc(ra, rb, rc).to_be_bytes())
}

fn bytes_from_ra_rb_rc_rd(ra: RegId, rb: RegId, rc: RegId, rd: RegId) -> [u8; 3] {
    u8x3_from_u8x4(u32_from_ra_rb_rc_rd(ra, rb, rc, rd).to_be_bytes())
}

fn bytes_from_ra_rb_imm12(ra: RegId, rb: RegId, imm: Imm12) -> [u8; 3] {
    u8x3_from_u8x4(u32_from_ra_rb_imm12(ra, rb, imm).to_be_bytes())
}

fn bytes_from_ra_imm18(ra: RegId, imm: Imm18) -> [u8; 3] {
    u8x3_from_u8x4(u32_from_ra_imm18(ra, imm).to_be_bytes())
}

fn bytes_from_imm24(imm: Imm24) -> [u8; 3] {
    u8x3_from_u8x4(u32_from_imm24(imm).to_be_bytes())
}

// --------------------------------------------------------

// Ignore the opcode byte, take the remaining data.
fn u8x3_from_u8x4([_, a, b, c]: [u8; 4]) -> [u8; 3] {
    [a, b, c]
}

// Produce the big-endian bytes for a u32, ignoring the opcode byte.
fn u8x4_from_u8x3([a, b, c]: [u8; 3]) -> [u8; 4] {
    [0, a, b, c]
}

// --------------------------------------------------------

// The size of the instruction isn't larger than necessary.
// 1 byte for the opcode, 3 bytes for registers and immediates.
#[test]
fn test_instruction_size() {
    assert_eq!(core::mem::size_of::<Instruction>(), core::mem::size_of::<RawInstruction>());
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
                BAL | BHEI | BHSH | BURN | CALL | CB | CCP | CROO | CSIZ | LDC | LOG | LOGD | MINT
                | RETD | RVRT | SMO | SCWQ | SRW | SRWQ | SWW | SWWQ | TIME | TR | TRO => false,
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
