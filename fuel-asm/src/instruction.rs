use crate::opcode::{Opcode, OpcodeRepr};

use fuel_types::{Immediate06, Immediate12, Immediate18, Immediate24, RegisterId, Word};

#[cfg(feature = "std")]
use std::{io, iter};

/// A version of Opcode that can used without unnecessary branching
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Instruction {
    /// Opcode
    op: u8,
    /// Register A
    ra: RegisterId,
    /// Register B
    rb: RegisterId,
    /// Register C
    rc: RegisterId,
    /// Register D
    rd: RegisterId,
    /// Immediate with 6 bits
    imm06: Immediate06,
    /// Immediate with 12 bits
    imm12: Immediate12,
    /// Immediate with 18 bits
    imm18: Immediate18,
    /// Immediate with 24 bits
    imm24: Immediate24,
}

impl Instruction {
    /// Size of an opcode in bytes
    pub const LEN: usize = 4;

    /// Extracts fields from a raw instruction
    pub const fn new(instruction: u32) -> Self {
        // TODO Optimize with native architecture (eg SIMD?) or facilitate
        // auto-vectorization

        let op = (instruction >> 24) as u8;

        let ra = ((instruction >> 18) & 0x3f) as RegisterId;
        let rb = ((instruction >> 12) & 0x3f) as RegisterId;
        let rc = ((instruction >> 6) & 0x3f) as RegisterId;
        let rd = (instruction & 0x3f) as RegisterId;

        let imm06 = (instruction & 0xff) as Immediate06;
        let imm12 = (instruction & 0x0fff) as Immediate12;
        let imm18 = (instruction & 0x3ffff) as Immediate18;
        let imm24 = (instruction & 0xffffff) as Immediate24;

        Self {
            op,
            ra,
            rb,
            rc,
            rd,
            imm06,
            imm12,
            imm18,
            imm24,
        }
    }

    /// Opcode
    pub const fn op(&self) -> u8 {
        self.op
    }

    /// Register A
    pub const fn ra(&self) -> RegisterId {
        self.ra
    }

    /// Register B
    pub const fn rb(&self) -> RegisterId {
        self.rb
    }

    /// Register C
    pub const fn rc(&self) -> RegisterId {
        self.rc
    }

    /// Register D
    pub const fn rd(&self) -> RegisterId {
        self.rd
    }

    /// Immediate with 6 bits
    pub const fn imm06(&self) -> Immediate06 {
        self.imm06
    }

    /// Immediate with 12 bits
    pub const fn imm12(&self) -> Immediate12 {
        self.imm12
    }

    /// Immediate with 18 bits
    pub const fn imm18(&self) -> Immediate18 {
        self.imm18
    }

    /// Immediate with 24 bits
    pub const fn imm24(&self) -> Immediate24 {
        self.imm24
    }

    /// Create a `Instruction` from a slice of bytes
    ///
    /// # Safety
    ///
    /// The caller must ensure that the slice is has at least `Self::LEN` bytes.
    pub unsafe fn from_slice_unchecked(buf: &[u8]) -> Self {
        debug_assert!(buf.len() >= Self::LEN);

        let instr = fuel_types::bytes::from_slice_unchecked(buf);
        let instr = u32::from_be_bytes(instr);

        Self::from(instr)
    }

    /// Convert the opcode to bytes representation
    pub fn to_bytes(self) -> [u8; Self::LEN] {
        u32::from(self).to_be_bytes()
    }

    /// Splits a Word into two [`Instruction`] that can be used to construct [`crate::Opcode`]
    pub const fn parse_word(word: Word) -> (Instruction, Instruction) {
        // Assumes Word is u64
        // https://doc.rust-lang.org/nightly/reference/expressions/operator-expr.html#numeric-cast4
        let lo = word as u32; // truncates, see link above
        let hi = (word >> 32) as u32;

        (Instruction::new(hi), Instruction::new(lo))
    }

    /// Convert the instruction into its internal representation
    ///
    /// `(repr, $ra, $rb, $rc, $rd, immediate)`
    pub const fn into_inner(
        self,
    ) -> (
        OpcodeRepr,
        RegisterId,
        RegisterId,
        RegisterId,
        RegisterId,
        Word,
    ) {
        let Self {
            op,
            ra,
            rb,
            rc,
            rd,
            imm06,
            imm12,
            imm18,
            imm24,
        } = self;

        let repr = OpcodeRepr::from_u8(op);

        let _ = imm06;
        let imm12 = imm12 as Word;
        let imm18 = imm18 as Word;
        let imm24 = imm24 as Word;

        let imm12_mask = (op & 0xf0 == 0x50) || (op & 0xf0 == 0x60);
        let imm18_mask = (op & 0xf0 == 0x70) || (op & 0xf0 == 0x80);
        let imm24_mask = (op & 0xf0 == 0x90) || (op & 0xf0 == 0xa0);

        let imm12_mask = imm12_mask as Word;
        let imm18_mask = imm18_mask as Word;
        let imm24_mask = imm24_mask as Word;

        let imm = imm12 * imm12_mask + imm18 * imm18_mask + imm24 * imm24_mask;

        (repr, ra, rb, rc, rd, imm)
    }
}

impl From<u32> for Instruction {
    fn from(instruction: u32) -> Self {
        Self::new(instruction)
    }
}

impl From<[u8; Instruction::LEN]> for Instruction {
    fn from(instruction: [u8; Instruction::LEN]) -> Self {
        u32::from_be_bytes(instruction).into()
    }
}

impl From<Opcode> for Instruction {
    fn from(op: Opcode) -> Self {
        u32::from(op).into()
    }
}

impl From<Instruction> for u32 {
    fn from(parsed: Instruction) -> u32 {
        // Convert all fields to u32 with correct shifting to just OR together
        // This truncates the field if they are too large

        let a = (parsed.ra as u32) << 18;
        let b = (parsed.rb as u32) << 12;
        let c = (parsed.rc as u32) << 6;
        let d = parsed.rd as u32;

        let imm12 = parsed.imm12 as u32;
        let imm18 = parsed.imm18 as u32;
        let imm24 = parsed.imm24 as u32;

        let repr = OpcodeRepr::from_u8(parsed.op);

        let args = match repr {
            OpcodeRepr::ADD
            | OpcodeRepr::AND
            | OpcodeRepr::DIV
            | OpcodeRepr::EQ
            | OpcodeRepr::EXP
            | OpcodeRepr::GT
            | OpcodeRepr::LT
            | OpcodeRepr::MLOG
            | OpcodeRepr::MROO
            | OpcodeRepr::MOD
            | OpcodeRepr::MUL
            | OpcodeRepr::OR
            | OpcodeRepr::SLL
            | OpcodeRepr::SRL
            | OpcodeRepr::SUB
            | OpcodeRepr::XOR
            | OpcodeRepr::CIMV
            | OpcodeRepr::MCP
            | OpcodeRepr::LDC
            | OpcodeRepr::SLDC
            | OpcodeRepr::TR
            | OpcodeRepr::ECR
            | OpcodeRepr::K256
            | OpcodeRepr::S256 => a | b | c,

            OpcodeRepr::ADDI
            | OpcodeRepr::ANDI
            | OpcodeRepr::DIVI
            | OpcodeRepr::EXPI
            | OpcodeRepr::MODI
            | OpcodeRepr::MULI
            | OpcodeRepr::ORI
            | OpcodeRepr::SLLI
            | OpcodeRepr::SRLI
            | OpcodeRepr::SUBI
            | OpcodeRepr::XORI
            | OpcodeRepr::JNEI
            | OpcodeRepr::LB
            | OpcodeRepr::LW
            | OpcodeRepr::SB
            | OpcodeRepr::SW => a | b | imm12,

            OpcodeRepr::MOVE
            | OpcodeRepr::NOT
            | OpcodeRepr::CTMV
            | OpcodeRepr::RETD
            | OpcodeRepr::MCL
            | OpcodeRepr::BHSH
            | OpcodeRepr::CROO
            | OpcodeRepr::CSIZ
            | OpcodeRepr::SRW
            | OpcodeRepr::SRWQ
            | OpcodeRepr::SWW
            | OpcodeRepr::SWWQ
            | OpcodeRepr::XIL
            | OpcodeRepr::XIS
            | OpcodeRepr::XOL
            | OpcodeRepr::XOS
            | OpcodeRepr::XWL
            | OpcodeRepr::XWS => a | b,

            OpcodeRepr::RET
            | OpcodeRepr::ALOC
            | OpcodeRepr::BHEI
            | OpcodeRepr::BURN
            | OpcodeRepr::CB
            | OpcodeRepr::MINT
            | OpcodeRepr::RVRT
            | OpcodeRepr::FLAG => a,

            OpcodeRepr::JI | OpcodeRepr::CFEI | OpcodeRepr::CFSI => imm24,

            OpcodeRepr::MCLI | OpcodeRepr::GM | OpcodeRepr::JNZI | OpcodeRepr::MOVI => a | imm18,

            OpcodeRepr::MEQ
            | OpcodeRepr::CALL
            | OpcodeRepr::CCP
            | OpcodeRepr::LOG
            | OpcodeRepr::LOGD
            | OpcodeRepr::TRO => a | b | c | d,

            _ => 0,
        };

        ((parsed.op as u32) << 24) | args
    }
}

#[cfg(feature = "std")]
impl Instruction {
    /// Create a `Instruction` from a slice of bytes
    ///
    /// This function will fail if the length of the bytes is smaller than
    /// [`Instruction::LEN`].
    pub fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() < Self::LEN {
            Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "The provided buffer is not big enough!",
            ))
        } else {
            // Safety: we check the length above
            unsafe { Ok(Self::from_slice_unchecked(bytes)) }
        }
    }

    /// Create a set of `Instruction` from an iterator of bytes
    ///
    /// If not padded to [`Self::LEN`], will consume the unaligned bytes but won't try to parse an
    /// instruction from them.
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

#[cfg(feature = "std")]
impl iter::FromIterator<Instruction> for Vec<u8> {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Instruction>,
    {
        iter.into_iter()
            .map(Instruction::to_bytes)
            .flatten()
            .collect()
    }
}

#[cfg(feature = "std")]
impl iter::FromIterator<Opcode> for Vec<Instruction> {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Opcode>,
    {
        iter.into_iter().map(Instruction::from).collect()
    }
}
