//! # The `impl_instructions!` macro
//!
//! The heart of this crate's implementation is the private `impl_instructions!` macro.
//! This macro is used to generate the `Instruction` and `Opcode` types along with their
//! implementations.
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
//! A `TryFrom<u8>` implementation is also provided, producing an `Err(InvalidOpcode)` in
//! the case that the byte represents a reserved or undefined value.
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
//! #[derive(Clone, Copy, Eq, Hash, PartialEq)]
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
//! The `From<Instruction> for u32` (aka `RawInstruction`) and `TryFrom<u32> for
//! Instruction` implementations can be found in the crate root.
//!
//! ## A unique unit type per operation
//!
//! In order to reduce the likelihood of misusing unrelated register IDs or immediate
//! values, we generate a unique unit type for each type of operation (i.e instruction
//! variant) and guard access to the relevant register IDs and immediate values behind
//! each type's unique methods.
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
//!             Self(pack::bytes_from_ra_rb_rc(ra, rb, rc))
//!         }
//!
//!         /// Convert the instruction into its parts.
//!         pub fn unpack(self) -> (RegId, RegId, RegId) {
//!             unpack::ra_rb_rc_from_bytes(self.0)
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
//!     // hand-write assembly for tests and benchmarking. As these constructors are public and
//!     // accept literal values, we check that the values are within range.
//!
//!     /// Adds two registers.
//!     pub fn add(ra: u8, rb: u8, rc: u8) -> Instruction {
//!         ADD::new(check_reg_id(ra), check_reg_id(rb), check_reg_id(rc)).into()
//!     }
//!
//!     /// Bitwise ANDs two registers.
//!     pub fn and(ra: u8, rb: u8, rc: u8) -> Instruction {
//!         AND::new(check_reg_id(ra), check_reg_id(rb), check_reg_id(rc)).into()
//!     }
//!
//!     // ...
//! };
//! ```
//!
//! ### Instruction Layout
//!
//! The function signatures of the `new` and `unpack` functions are derived from the
//! instruction's data layout described in the `impl_instructions!` table.
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
//! The shorthand instruction constructors (e.g. `add`, `and`, etc) are specifically
//! designed to make it easier to handwrite assembly for tests or benchmarking. Unlike the
//! `$OP::new` constructors which require typed register ID or immediate inputs, the
//! instruction constructors allow for constructing `Instruction`s from convenient literal
//! value inputs. E.g.
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
//!     op::eck1(0x10, 0x11, 0x12),// recover public key in memory[r[0x10], 64]
//!     op::ret(0x01),             // return `1`
//! ];
//! ```

#![deny(unused_must_use, missing_docs)]
#![deny(
    clippy::arithmetic_side_effects,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::string_slice
)]

extern crate proc_macro;

use input::InstructionList;
use quote::quote;

mod codegen;
mod input;
mod packing;

/// Generates implementations for the FuelVM instruction types.
#[proc_macro]
pub fn impl_instructions(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let instructions = syn::parse_macro_input!(input as InstructionList);

    let op_structs = codegen::op_structs(&instructions);
    let op_debug_impl = codegen::op_debug_impl(&instructions);
    let from_op = codegen::from_op(&instructions);
    let op_constructor_shorthand = codegen::op_constructor_shorthand(&instructions);
    let op_fn_new = codegen::op_fn_new(&instructions);
    let op_constructors_typescript = codegen::op_constructors_typescript(&instructions);
    let op_fn_unpack = codegen::op_fn_unpack(&instructions);
    let op_fn_reserved_part_is_zero = codegen::op_fn_reserved_part_is_zero(&instructions);
    let op_fn_reg_ids = codegen::op_fn_reg_ids(&instructions);

    let opcode_enum = codegen::opcode_enum(&instructions);
    let opcode_try_from = codegen::opcode_try_from(&instructions);
    let instruction_enum = codegen::instruction_enum(&instructions);
    let instruction_enum_debug = codegen::instruction_enum_debug(&instructions);
    let instruction_enum_fn_opcode = codegen::instruction_enum_fn_opcode(&instructions);
    let instruction_enum_fn_reg_ids = codegen::instruction_enum_fn_reg_ids(&instructions);
    let instruction_try_from_bytes = codegen::instruction_try_from_bytes(&instructions);
    let bytes_from_instruction = codegen::bytes_from_instruction(&instructions);

    (quote! {
        #[doc = "Opcode-specific definitions and implementations."]
        #[allow(clippy::unused_unit)] // Simplify codegen
        pub mod _op {
            use super::*;
            #op_structs
            #op_debug_impl
            #from_op
            #op_constructor_shorthand
            #op_fn_new
            #op_constructors_typescript
            #op_fn_unpack
            #op_fn_reserved_part_is_zero
            #op_fn_reg_ids
        }
        #opcode_enum
        #opcode_try_from
        #instruction_enum
        #instruction_enum_debug
        #instruction_enum_fn_opcode
        #instruction_enum_fn_reg_ids
        #instruction_try_from_bytes
        #bytes_from_instruction

        #[cfg(feature = "typescript")]
        impl From<Instruction> for typescript::Instruction {
            fn from(inst: Instruction) -> Self {
                typescript::Instruction::new(inst)
            }
        }

    })
    .into()
}
