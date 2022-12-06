//! # The `impl_instructions!` macro
//!
//! The heart of this crate's implementation is the private `impl_instructions!` macro. This macro
//! is used to generate the `Instruction` and `Opcode` types along with their implementations.
//!
//! The intention is to allow for having a single source of truth from which each of the
//! instruction-related types and implementations are derived.
//!
//! Its usage looks like this:
//!
//! ```rust,ignore
//! impl_instructions! {
//!     "Adds two registers."
//!     0x10 ADD add [RegId RegId RegId]
//!     "Bitwise ANDs two registers."
//!     0x11 AND and [RegId RegId RegId]
//!     // ...
//! }
//! ```
//!
//! Each instruction's row includes:
//!
//! - A short docstring.
//! - The Opcode byte value.
//! - An uppercase identifier (for generating variants and types).
//! - A lowercase identifier (for generating the shorthand instruction constructor).
//! - The instruction layout (for the `new` and `unpack` functions).
//!
//! The following sections describe each of the items that are derived from the
//! `impl_instructions!` table in more detail.
//!
//! ## The `Opcode` enum
//!
//! Represents the bytecode portion of an instruction.
//!
//! ```rust,ignore
//! /// Solely the opcode portion of an instruction represented as a single byte.
//! #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
//! #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
//! #[repr(u8)]
//! pub enum Opcode {
//!     /// Adds two registers.
//!     ADD = 0x10,
//!     /// Bitwise ANDs two registers.
//!     AND = 0x11,
//!     // ...
//! }
//! ```
//!
//! A `TryFrom<u8>` implementation is also provided, producing an `Err(InvalidOpcode)` in the case
//! that the byte represents a reserved or undefined value.
//!
//! ```rust
//! # use fuel_asm::{InvalidOpcode, Opcode};
//! assert_eq!(Opcode::try_from(0x10), Ok(Opcode::ADD));
//! assert_eq!(Opcode::try_from(0x11), Ok(Opcode::AND));
//! assert_eq!(Opcode::try_from(0), Err(InvalidOpcode));
//! ```
//!
//! ## The `Instruction` enum
//!
//! Represents a single, full instruction, discriminated by its `Opcode`.
//!
//! ```rust,ignore
//! /// Representation of a single instruction for the interpreter.
//! ///
//! /// The opcode is represented in the tag (variant), or may be retrieved in the form of an
//! /// `Opcode` byte using the `opcode` method.
//! ///
//! /// The register and immediate data associated with the instruction is represented within
//! /// an inner unit type wrapper around the 3 remaining bytes.
//! #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
//! #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
//! pub enum Instruction {
//!     /// Adds two registers.
//!     ADD(op::ADD),
//!     /// Bitwise ANDs two registers.
//!     AND(op::AND),
//!     // ...
//! }
//! ```
//!
//! The `From<Instruction> for u32` (aka `RawInstruction`) and `TryFrom<u32> for Instruction`
//! implementations can be found in the crate root.
//!
//! ## A unique unit type per operation
//!
//! In order to reduce the likelihood of misusing unrelated register IDs or immediate values, we
//! generate a unique unit type for each type of operation (i.e instruction variant) and guard
//! access to the relevant register IDs and immediate values behind each type's unique methods.
//!
//! These unique operation types are generated as follows within a dedicated `op` module:
//!
//! ```rust,ignore
//! pub mod op {
//!     //! Definitions and implementations for each unique instruction type, one for each
//!     //! unique `Opcode` variant.
//!
//!     // A unique type for each operation.
//!
//!     /// Adds two registers.
//!     pub struct ADD([u8; 3]);
//!
//!     /// Bitwise ANDs two registers.
//!     pub struct AND([u8; 3]);
//!
//!     // ...
//!
//!     // An implementation for each unique type.
//!
//!     impl ADD {
//!         pub const OPCODE: Opcode = Opcode::ADD;
//!
//!         /// Construct the instruction from its parts.
//!         pub fn new(ra: RegId, rb: RegId, rc: RegId) -> Self {
//!             Self(bytes_from_ra_rb_rc(ra, rb, rc))
//!         }
//!
//!         /// Convert the instruction into its parts.
//!         pub fn unpack(self) -> (RegId, RegId, RegId) {
//!             ra_rb_rc_from_bytes(self.0)
//!         }
//!     }
//!
//!     impl AND {
//!         // ...
//!     }
//!
//!     // ...
//!
//!     // A short-hand `Instruction` constructor for each operation to make it easier to
//!     // hand-write assembly for tests and benchmarking.
//!
//!     /// Adds two registers.
//!     pub fn add(ra: u8, rb: u8, rc: u8) -> Instruction {
//!         ADD::new(RegId::new(ra), RegId::new(rb), RegId::new(rc)).into()
//!     }
//!
//!     /// Bitwise ANDs two registers.
//!     pub fn and(ra: u8, rb: u8, rc: u8) -> Instruction {
//!         AND::new(RegId::new(ra), RegId::new(rb), RegId::new(rc)).into()
//!     }
//!
//!     // ...
//! };
//! ```
//!
//! ### Instruction Layout
//!
//! The function signatures of the `new` and `unpack` functions are derived from the instruction's
//! data layout described in the `impl_instructions!` table.
//!
//! For example, the `unpack` method for `ADD` looks like this:
//!
//! ```rust,ignore
//! // 0x10 ADD add [RegId RegId RegId]
//! pub fn unpack(self) -> (RegId, RegId, RegId)
//! ```
//!
//! While the `unpack` method for `ADDI` looks like this:
//!
//! ```rust,ignore
//! // 0x50 ADDI addi [RegId RegId Imm12]
//! pub fn unpack(self) -> (RegId, RegId, Imm12)
//! ```
//!
//! ### Shorthand Constructors
//!
//! The shorthand instruction constructors (e.g. `add`, `and`, etc) are specifically designed to
//! make it easier to handwrite assembly for tests or benchmarking. Unlike the `$OP::new`
//! constructors which require typed register ID or immediate inputs, the instruction constructors
//! allow for constructing `Instruction`s from convenient literal value inputs. E.g.
//!
//! ```rust
//! use fuel_asm::{op, Instruction};
//!
//! // A sample program to perform ecrecover
//! let program: Vec<Instruction> = vec![
//!     op::move_(0x10, 0x01),     // set r[0x10] := $one
//!     op::slli(0x20, 0x10, 5),   // set r[0x20] := `r[0x10] << 5 == 32`
//!     op::slli(0x21, 0x10, 6),   // set r[0x21] := `r[0x10] << 6 == 64`
//!     op::aloc(0x21),            // alloc `r[0x21] == 64` to the heap
//!     op::addi(0x10, 0x07, 1),   // set r[0x10] := `$hp + 1` (allocated heap)
//!     op::move_(0x11, 0x04),     // set r[0x11] := $ssp
//!     op::add(0x12, 0x04, 0x20), // set r[0x12] := `$ssp + r[0x20]`
//!     op::ecr(0x10, 0x11, 0x12), // recover public key in memory[r[0x10], 64]
//!     op::ret(0x01),             // return `1`
//! ];
//! ```

/// This macro is intentionaly private. See the module-level documentation for a thorough
/// explanation of how this macro works.
macro_rules! impl_instructions {
    // Recursively declares a unique struct for each opcode.
    (decl_op_struct $doc:literal $ix:literal $Op:ident $op:ident [$($field:ident)*] $($rest:tt)*) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub struct $Op(pub (super) [u8; 3]);
        impl_instructions!(decl_op_struct $($rest)*);
    };
    (decl_op_struct) => {};

    // Define the `OpcodeRepr` enum.
    (decl_opcode_enum $($doc:literal $ix:literal $Op:ident $op:ident [$($field:ident)*])*) => {
        /// Solely the opcode portion of an instruction represented as a single byte.
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[repr(u8)]
        pub enum Opcode {
            $(
                #[doc = $doc]
                $Op = $ix,
            )*
        }
    };

    // Define the `Opcode` enum.
    (decl_instruction_enum $($doc:literal $ix:literal $Op:ident $op:ident [$($field:ident)*])*) => {
        /// Representation of a single instruction for the interpreter.
        ///
        /// The opcode is represented in the tag (variant), or may be retrieved in the form of an
        /// `Opcode` byte using the `opcode` method.
        ///
        /// The register and immediate data associated with the instruction is represented within
        /// an inner unit type wrapper around the 3 remaining bytes.
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub enum Instruction {
            $(
                #[doc = $doc]
                $Op(op::$Op),
            )*
        }
    };

    // Generate a constructor based on the field layout.
    (impl_op_new [RegId]) => {
        /// Construct the instruction from its parts.
        pub fn new(ra: RegId) -> Self {
            Self(bytes_from_ra(ra))
        }
    };
    (impl_op_new [RegId RegId]) => {
        /// Construct the instruction from its parts.
        pub fn new(ra: RegId, rb: RegId) -> Self {
            Self(bytes_from_ra_rb(ra, rb))
        }
    };
    (impl_op_new [RegId RegId RegId]) => {
        /// Construct the instruction from its parts.
        pub fn new(ra: RegId, rb: RegId, rc: RegId) -> Self {
            Self(bytes_from_ra_rb_rc(ra, rb, rc))
        }
    };
    (impl_op_new [RegId RegId RegId RegId]) => {
        /// Construct the instruction from its parts.
        pub fn new(ra: RegId, rb: RegId, rc: RegId, rd: RegId) -> Self {
            Self(bytes_from_ra_rb_rc_rd(ra, rb, rc, rd))
        }
    };
    (impl_op_new [RegId RegId Imm12]) => {
        /// Construct the instruction from its parts.
        pub fn new(ra: RegId, rb: RegId, imm: Imm12) -> Self {
            Self(bytes_from_ra_rb_imm12(ra, rb, imm))
        }
    };
    (impl_op_new [RegId Imm18]) => {
        /// Construct the instruction from its parts.
        pub fn new(ra: RegId, imm: Imm18) -> Self {
            Self(bytes_from_ra_imm18(ra, imm))
        }
    };
    (impl_op_new [Imm24]) => {
        /// Construct the instruction from its parts.
        pub fn new(imm: Imm24) -> Self {
            Self(bytes_from_imm24(imm))
        }
    };
    (impl_op_new []) => {
        /// Construct the instruction.
        pub fn new() -> Self {
            Self([0; 3])
        }
    };

    // Generate an accessor method for each field. Recurse based on layout.
    (impl_op_accessors [RegId]) => {
        /// Access the ID for register A.
        pub fn ra(&self) -> RegId {
            ra_from_bytes(self.0)
        }
    };
    (impl_op_accessors [RegId RegId]) => {
        impl_instructions!(impl_op_accessors [RegId]);
        /// Access the ID for register B.
        pub fn rb(&self) -> RegId {
            rb_from_bytes(self.0)
        }
    };
    (impl_op_accessors [RegId RegId RegId]) => {
        impl_instructions!(impl_op_accessors [RegId RegId]);
        /// Access the ID for register C.
        pub fn rc(&self) -> RegId {
            rc_from_bytes(self.0)
        }
    };
    (impl_op_accessors [RegId RegId RegId RegId]) => {
        impl_instructions!(impl_op_accessors [RegId RegId RegId]);
        /// Access the ID for register D.
        pub fn rd(&self) -> RegId {
            rd_from_bytes(self.0)
        }
    };
    (impl_op_accessors [RegId RegId Imm12]) => {
        impl_instructions!(impl_op_accessors [RegId RegId]);
        /// Access the 12-bit immediate value.
        pub fn imm12(&self) -> Imm12 {
            imm12_from_bytes(self.0)
        }
    };
    (impl_op_accessors [RegId Imm18]) => {
        impl_instructions!(impl_op_accessors [RegId]);
        /// Access the 18-bit immediate value.
        pub fn imm18(&self) -> Imm18 {
            imm18_from_bytes(self.0)
        }
    };
    (impl_op_accessors [Imm24]) => {
        /// Access the 24-bit immediate value.
        pub fn imm24(&self) -> Imm24 {
            imm24_from_bytes(self.0)
        }
    };
    (impl_op_accessors []) => {};

    // Generate a method for converting the instruction into its parts.
    (impl_op_unpack [RegId]) => {
        /// Convert the instruction into its parts.
        pub fn unpack(self) -> RegId {
            ra_from_bytes(self.0)
        }
    };
    (impl_op_unpack [RegId RegId]) => {
        /// Convert the instruction into its parts.
        pub fn unpack(self) -> (RegId, RegId) {
            ra_rb_from_bytes(self.0)
        }
    };
    (impl_op_unpack [RegId RegId RegId]) => {
        /// Convert the instruction into its parts.
        pub fn unpack(self) -> (RegId, RegId, RegId) {
            ra_rb_rc_from_bytes(self.0)
        }
    };
    (impl_op_unpack [RegId RegId RegId RegId]) => {
        /// Convert the instruction into its parts.
        pub fn unpack(self) -> (RegId, RegId, RegId, RegId) {
            ra_rb_rc_rd_from_bytes(self.0)
        }
    };
    (impl_op_unpack [RegId RegId Imm12]) => {
        /// Convert the instruction into its parts.
        pub fn unpack(self) -> (RegId, RegId, Imm12) {
            ra_rb_imm12_from_bytes(self.0)
        }
    };
    (impl_op_unpack [RegId Imm18]) => {
        /// Convert the instruction into its parts.
        pub fn unpack(self) -> (RegId, Imm18) {
            ra_imm18_from_bytes(self.0)
        }
    };
    (impl_op_unpack [Imm24]) => {
        /// Convert the instruction into its parts.
        pub fn unpack(self) -> Imm24 {
            imm24_from_bytes(self.0)
        }
    };
    (impl_op_unpack []) => {};

    // Generate a free function named after the $op for constructing an `Instruction`.
    (impl_op_constructor $doc:literal $Op:ident $op:ident [RegId]) => {
        #[doc = $doc]
        pub fn $op(ra: u8) -> Instruction {
            $Op::new(RegId::new(ra)).into()
        }
    };
    (impl_op_constructor $doc:literal $Op:ident $op:ident [RegId RegId]) => {
        #[doc = $doc]
        pub fn $op(ra: u8, rb: u8) -> Instruction {
            $Op::new(RegId::new(ra), RegId::new(rb)).into()
        }
    };
    (impl_op_constructor $doc:literal $Op:ident $op:ident [RegId RegId RegId]) => {
        #[doc = $doc]
        pub fn $op(ra: u8, rb: u8, rc: u8) -> Instruction {
            $Op::new(RegId::new(ra), RegId::new(rb), RegId::new(rc)).into()
        }
    };
    (impl_op_constructor $doc:literal $Op:ident $op:ident [RegId RegId RegId RegId]) => {
        #[doc = $doc]
        pub fn $op(ra: u8, rb: u8, rc: u8, rd: u8) -> Instruction {
            $Op::new(RegId::new(ra), RegId::new(rb), RegId::new(rc), RegId::new(rd)).into()
        }
    };
    (impl_op_constructor $doc:literal $Op:ident $op:ident [RegId RegId Imm12]) => {
        #[doc = $doc]
        pub fn $op(ra: u8, rb: u8, imm: u16) -> Instruction {
            $Op::new(RegId::new(ra), RegId::new(rb), Imm12::new(imm)).into()
        }
    };
    (impl_op_constructor $doc:literal $Op:ident $op:ident [RegId Imm18]) => {
        #[doc = $doc]
        pub fn $op(ra: u8, imm: u32) -> Instruction {
            $Op::new(RegId::new(ra), Imm18::new(imm)).into()
        }
    };
    (impl_op_constructor $doc:literal $Op:ident $op:ident [Imm24]) => {
        #[doc = $doc]
        pub fn $op(imm: u32) -> Instruction {
            $Op::new(Imm24::new(imm)).into()
        }
    };
    (impl_op_constructor $doc:literal $Op:ident $op:ident []) => {
        #[doc = $doc]
        pub fn $op() -> Instruction {
            $Op::new().into()
        }
    };

    // Implement constructors and accessors for register and immediate values.
    (impl_op $doc:literal $ix:literal $Op:ident $op:ident [$($field:ident)*] $($rest:tt)*) => {
        impl $Op {
            /// The associated 8-bit Opcode value.
            pub const OPCODE: Opcode = Opcode::$Op;

            impl_instructions!(impl_op_new [$($field)*]);
            impl_instructions!(impl_op_accessors [$($field)*]);
            impl_instructions!(impl_op_unpack [$($field)*]);
        }

        impl_instructions!(impl_op_constructor $doc $Op $op [$($field)*]);

        impl From<$Op> for [u8; 3] {
            fn from($Op(arr): $Op) -> Self {
                arr
            }
        }

        impl From<$Op> for [u8; 4] {
            fn from($Op([a, b, c]): $Op) -> Self {
                [$Op::OPCODE as u8, a, b, c]
            }
        }

        impl From<$Op> for u32 {
            fn from(op: $Op) -> Self {
                u32::from_be_bytes(op.into())
            }
        }

        impl From<$Op> for Instruction {
            fn from(op: $Op) -> Self {
                Instruction::$Op(op)
            }
        }

        impl_instructions!(impl_op $($rest)*);
    };
    (impl_op) => {};

    // Implement `TryFrom<u8>` for `Opcode`.
    (impl_opcode $($doc:literal $ix:literal $Op:ident $op:ident [$($field:ident)*])*) => {
        impl core::convert::TryFrom<u8> for Opcode {
            type Error = InvalidOpcode;
            fn try_from(u: u8) -> Result<Self, Self::Error> {
                match u {
                    $(
                        $ix => Ok(Opcode::$Op),
                    )*
                    _ => Err(InvalidOpcode),
                }
            }
        }
    };

    // Implement accessors for register and immediate values.
    (impl_instruction $($doc:literal $ix:literal $Op:ident $op:ident [$($field:ident)*])*) => {
        impl Instruction {
            /// This instruction's opcode.
            pub fn opcode(&self) -> Opcode {
                match self {
                    $(
                        Self::$Op(_) => Opcode::$Op,
                    )*
                }
            }
        }

        impl From<Instruction> for [u8; 4] {
            fn from(inst: Instruction) -> Self {
                match inst {
                    $(
                        Instruction::$Op(op) => op.into(),
                    )*
                }
            }
        }

        impl core::convert::TryFrom<[u8; 4]> for Instruction {
            type Error = InvalidOpcode;
            fn try_from([op, a, b, c]: [u8; 4]) -> Result<Self, Self::Error> {
                match Opcode::try_from(op)? {
                    $(
                        Opcode::$Op => Ok(Self::$Op(op::$Op([a, b, c]))),
                    )*
                }
            }
        }
    };

    // Entrypoint to the macro, generates structs, methods, opcode enum and instruction enum
    // separately.
    ($($tts:tt)*) => {
        pub mod op {
            //! Definitions and implementations for each unique instruction type, one for each
            //! unique `Opcode` variant.
            use super::*;
            impl_instructions!(decl_op_struct $($tts)*);
            impl_instructions!(impl_op $($tts)*);
        }
        impl_instructions!(decl_opcode_enum $($tts)*);
        impl_instructions!(decl_instruction_enum $($tts)*);
        impl_instructions!(impl_opcode $($tts)*);
        impl_instructions!(impl_instruction $($tts)*);
    };
}
