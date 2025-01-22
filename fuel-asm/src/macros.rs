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

// Generate a shorthand free function named after the $op for constructing an
// `Instruction`.
macro_rules! op_constructor {
    ($doc:literal $Op:ident $op:ident[$ra:ident : RegId]) => {
        #[doc = $doc]
        pub fn $op<A: CheckRegId>($ra: A) -> Instruction {
            $Op::new($ra.check()).into()
        }

        #[cfg(feature = "typescript")]
        const _: () = {
            use super::*;

            #[wasm_bindgen::prelude::wasm_bindgen]
            #[doc = $doc]
            pub fn $op($ra: u8) -> typescript::Instruction {
                crate::op::$op($ra).into()
            }
        };
    };
    ($doc:literal $Op:ident $op:ident[$ra:ident : RegId $rb:ident : RegId]) => {
        #[doc = $doc]
        pub fn $op<A: CheckRegId, B: CheckRegId>($ra: A, $rb: B) -> Instruction {
            $Op::new($ra.check(), $rb.check()).into()
        }

        #[cfg(feature = "typescript")]
        const _: () = {
            use super::*;

            #[wasm_bindgen::prelude::wasm_bindgen]
            #[doc = $doc]
            pub fn $op($ra: u8, $rb: u8) -> typescript::Instruction {
                crate::op::$op($ra, $rb).into()
            }
        };
    };
    (
        $doc:literal
        $Op:ident
        $op:ident[$ra:ident : RegId $rb:ident : RegId $rc:ident : RegId]
    ) => {
        #[doc = $doc]
        pub fn $op<A: CheckRegId, B: CheckRegId, C: CheckRegId>(
            $ra: A,
            $rb: B,
            $rc: C,
        ) -> Instruction {
            $Op::new($ra.check(), $rb.check(), $rc.check()).into()
        }

        #[cfg(feature = "typescript")]
        const _: () = {
            use super::*;

            #[wasm_bindgen::prelude::wasm_bindgen]
            #[doc = $doc]
            pub fn $op($ra: u8, $rb: u8, $rc: u8) -> typescript::Instruction {
                crate::op::$op($ra, $rb, $rc).into()
            }
        };
    };
    (
        $doc:literal
        $Op:ident
        $op:ident[$ra:ident : RegId $rb:ident : RegId $rc:ident : RegId $rd:ident : RegId]
    ) => {
        #[doc = $doc]
        pub fn $op<A: CheckRegId, B: CheckRegId, C: CheckRegId, D: CheckRegId>(
            $ra: A,
            $rb: B,
            $rc: C,
            $rd: D,
        ) -> Instruction {
            $Op::new($ra.check(), $rb.check(), $rc.check(), $rd.check()).into()
        }

        #[cfg(feature = "typescript")]
        const _: () = {
            use super::*;

            #[wasm_bindgen::prelude::wasm_bindgen]
            #[doc = $doc]
            pub fn $op($ra: u8, $rb: u8, $rc: u8, $rd: u8) -> typescript::Instruction {
                crate::op::$op($ra, $rb, $rc, $rd).into()
            }
        };
    };
    (
        $doc:literal
        $Op:ident
        $op:ident[$ra:ident : RegId $rb:ident : RegId $rc:ident : RegId $imm:ident : Imm06]
    ) => {
        #[doc = $doc]
        pub fn $op<A: CheckRegId, B: CheckRegId, C: CheckRegId>(
            $ra: A,
            $rb: B,
            $rc: C,
            $imm: u8,
        ) -> Instruction {
            $Op::new($ra.check(), $rb.check(), $rc.check(), check_imm06($imm)).into()
        }

        #[cfg(feature = "typescript")]
        const _: () = {
            use super::*;

            #[wasm_bindgen::prelude::wasm_bindgen]
            #[doc = $doc]
            pub fn $op($ra: u8, $rb: u8, $rc: u8, $imm: u8) -> typescript::Instruction {
                crate::op::$op($ra, $rb, $rc, $imm).into()
            }
        };
    };
    (
        $doc:literal
        $Op:ident
        $op:ident[$ra:ident : RegId $rb:ident : RegId $imm:ident : Imm12]
    ) => {
        #[doc = $doc]
        pub fn $op<A: CheckRegId, B: CheckRegId>(
            $ra: A,
            $rb: B,
            $imm: u16,
        ) -> Instruction {
            $Op::new($ra.check(), $rb.check(), check_imm12($imm)).into()
        }

        #[cfg(feature = "typescript")]
        const _: () = {
            use super::*;

            #[wasm_bindgen::prelude::wasm_bindgen]
            #[doc = $doc]
            pub fn $op($ra: u8, $rb: u8, $imm: u16) -> typescript::Instruction {
                crate::op::$op($ra, $rb, $imm).into()
            }
        };
    };
    ($doc:literal $Op:ident $op:ident[$ra:ident : RegId $imm:ident : Imm18]) => {
        #[doc = $doc]
        pub fn $op<A: CheckRegId>($ra: A, $imm: u32) -> Instruction {
            $Op::new($ra.check(), check_imm18($imm)).into()
        }

        #[cfg(feature = "typescript")]
        const _: () = {
            use super::*;

            #[wasm_bindgen::prelude::wasm_bindgen]
            #[doc = $doc]
            pub fn $op($ra: u8, $imm: u32) -> typescript::Instruction {
                crate::op::$op($ra, $imm).into()
            }
        };
    };
    ($doc:literal $Op:ident $op:ident[$imm:ident : Imm24]) => {
        #[doc = $doc]
        pub fn $op($imm: u32) -> Instruction {
            $Op::new(check_imm24($imm)).into()
        }

        #[cfg(feature = "typescript")]
        const _: () = {
            use super::*;

            #[wasm_bindgen::prelude::wasm_bindgen]
            #[doc = $doc]
            pub fn $op($imm: u32) -> typescript::Instruction {
                crate::op::$op($imm).into()
            }
        };
    };
    ($doc:literal $Op:ident $op:ident[]) => {
        #[doc = $doc]
        pub fn $op() -> Instruction {
            $Op::new().into()
        }

        #[cfg(feature = "typescript")]
        const _: () = {
            use super::*;

            #[wasm_bindgen::prelude::wasm_bindgen]
            #[doc = $doc]
            pub fn $op() -> typescript::Instruction {
                crate::op::$op().into()
            }
        };
    };
}

// Generate approriate `new` constructor for the instruction
macro_rules! op_new {
    // Generate a constructor based on the field layout.
    ($Op:ident $ra:ident : RegId) => {
        impl $Op {
            /// Construct the instruction from its parts.
            pub fn new($ra: RegId) -> Self {
                Self(pack::bytes_from_ra($ra))
            }
        }

        #[cfg(feature = "typescript")]
        #[wasm_bindgen::prelude::wasm_bindgen]
        impl $Op {
            #[wasm_bindgen(constructor)]
            /// Construct the instruction from its parts.
            pub fn new_typescript($ra: RegId) -> Self {
                Self::new($ra)
            }
        }
    };
    ($Op:ident $ra:ident : RegId $rb:ident : RegId) => {
        impl $Op {
            /// Construct the instruction from its parts.
            pub fn new($ra: RegId, $rb: RegId) -> Self {
                Self(pack::bytes_from_ra_rb($ra, $rb))
            }
        }

        #[cfg(feature = "typescript")]
        #[wasm_bindgen::prelude::wasm_bindgen]
        impl $Op {
            #[wasm_bindgen(constructor)]
            /// Construct the instruction from its parts.
            pub fn new_typescript($ra: RegId, $rb: RegId) -> Self {
                Self::new($ra, $rb)
            }
        }
    };
    ($Op:ident $ra:ident : RegId $rb:ident : RegId $rc:ident : RegId) => {
        impl $Op {
            /// Construct the instruction from its parts.
            pub fn new($ra: RegId, $rb: RegId, $rc: RegId) -> Self {
                Self(pack::bytes_from_ra_rb_rc($ra, $rb, $rc))
            }
        }

        #[cfg(feature = "typescript")]
        #[wasm_bindgen::prelude::wasm_bindgen]
        impl $Op {
            #[wasm_bindgen(constructor)]
            /// Construct the instruction from its parts.
            pub fn new_typescript($ra: RegId, $rb: RegId, $rc: RegId) -> Self {
                Self::new($ra, $rb, $rc)
            }
        }
    };
    (
        $Op:ident $ra:ident : RegId $rb:ident : RegId $rc:ident : RegId $rd:ident : RegId
    ) => {
        impl $Op {
            /// Construct the instruction from its parts.
            pub fn new($ra: RegId, $rb: RegId, $rc: RegId, $rd: RegId) -> Self {
                Self(pack::bytes_from_ra_rb_rc_rd($ra, $rb, $rc, $rd))
            }
        }

        #[cfg(feature = "typescript")]
        #[wasm_bindgen::prelude::wasm_bindgen]
        impl $Op {
            #[wasm_bindgen(constructor)]
            /// Construct the instruction from its parts.
            pub fn new_typescript(
                $ra: RegId,
                $rb: RegId,
                $rc: RegId,
                $rd: RegId,
            ) -> Self {
                Self::new($ra, $rb, $rc, $rd)
            }
        }
    };
    (
        $Op:ident
        $ra:ident : RegId
        $rb:ident : RegId
        $rc:ident : RegId
        $imm:ident : Imm06
    ) => {
        impl $Op {
            /// Construct the instruction from its parts.
            pub fn new($ra: RegId, $rb: RegId, $rc: RegId, $imm: Imm06) -> Self {
                Self(pack::bytes_from_ra_rb_rc_imm06($ra, $rb, $rc, $imm))
            }
        }

        #[cfg(feature = "typescript")]
        #[wasm_bindgen::prelude::wasm_bindgen]
        impl $Op {
            #[wasm_bindgen(constructor)]
            /// Construct the instruction from its parts.
            pub fn new_typescript(
                $ra: RegId,
                $rb: RegId,
                $rc: RegId,
                $imm: Imm06,
            ) -> Self {
                Self::new($ra, $rb, $rc, $imm)
            }
        }
    };
    ($Op:ident $ra:ident : RegId $rb:ident : RegId $imm:ident : Imm12) => {
        impl $Op {
            /// Construct the instruction from its parts.
            pub fn new($ra: RegId, $rb: RegId, $imm: Imm12) -> Self {
                Self(pack::bytes_from_ra_rb_imm12($ra, $rb, $imm))
            }
        }

        #[cfg(feature = "typescript")]
        #[wasm_bindgen::prelude::wasm_bindgen]
        impl $Op {
            #[wasm_bindgen(constructor)]
            /// Construct the instruction from its parts.
            pub fn new_typescript($ra: RegId, $rb: RegId, $imm: Imm12) -> Self {
                Self::new($ra, $rb, $imm)
            }
        }
    };
    ($Op:ident $ra:ident : RegId $imm:ident : Imm18) => {
        impl $Op {
            /// Construct the instruction from its parts.
            pub fn new($ra: RegId, $imm: Imm18) -> Self {
                Self(pack::bytes_from_ra_imm18($ra, $imm))
            }
        }

        #[cfg(feature = "typescript")]
        #[wasm_bindgen::prelude::wasm_bindgen]
        impl $Op {
            #[wasm_bindgen(constructor)]
            /// Construct the instruction from its parts.
            pub fn new_typescript($ra: RegId, $imm: Imm18) -> Self {
                Self::new($ra, $imm)
            }
        }
    };
    ($Op:ident $imm:ident : Imm24) => {
        impl $Op {
            /// Construct the instruction from its parts.
            pub fn new($imm: Imm24) -> Self {
                Self(pack::bytes_from_imm24($imm))
            }
        }

        #[cfg(feature = "typescript")]
        #[wasm_bindgen::prelude::wasm_bindgen]
        impl $Op {
            #[wasm_bindgen(constructor)]
            /// Construct the instruction from its parts.
            pub fn new_typescript($imm: Imm24) -> Self {
                Self::new($imm)
            }
        }
    };
    ($Op:ident) => {
        impl $Op {
            /// Construct the instruction.
            #[allow(clippy::new_without_default)]
            pub fn new() -> Self {
                Self([0; 3])
            }
        }

        #[cfg(feature = "typescript")]
        #[wasm_bindgen::prelude::wasm_bindgen]
        impl $Op {
            #[wasm_bindgen(constructor)]
            /// Construct the instruction.
            #[allow(clippy::new_without_default)]
            pub fn new_typescript() -> Self {
                Self::new()
            }
        }
    };
}

// Generate an accessor method for each field. Recurse based on layout.
macro_rules! op_accessors {
    ($Op:ident $ra:ident: RegId) => {
        #[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
        impl $Op {
            /// Access the ID for register A.
            pub fn ra(&self) -> RegId {
                unpack::ra_from_bytes(self.0)
            }
        }
    };
    ($Op:ident $ra:ident: RegId $rb:ident: RegId) => {
        op_accessors!($Op ra: RegId);

        #[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
        impl $Op {
            /// Access the ID for register B.
            pub fn rb(&self) -> RegId {
                unpack::rb_from_bytes(self.0)
            }
        }
    };
    ($Op:ident $ra:ident: RegId $rb:ident: RegId $rc:ident: RegId) => {
        op_accessors!($Op $ra: RegId $rb: RegId);

        #[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
        impl $Op {
            /// Access the ID for register C.
            pub fn rc(&self) -> RegId {
                unpack::rc_from_bytes(self.0)
            }
        }
    };
    ($Op:ident $ra:ident: RegId $rb:ident: RegId $rc:ident: RegId $rd:ident: RegId) => {
        op_accessors!($Op $ra: RegId $rb: RegId $rc: RegId);

        #[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
        impl $Op {
            /// Access the ID for register D.
            pub fn rd(&self) -> RegId {
                unpack::rd_from_bytes(self.0)
            }
        }
    };
    ($Op:ident $ra:ident: RegId $rb:ident: RegId $rc:ident: RegId $imm:ident: Imm06) => {
        op_accessors!($Op $ra: RegId rb: RegId $rc: RegId);

        #[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
        impl $Op {
            /// Access the 6-bit immediate value.
            pub fn imm06(&self) -> Imm06 {
                unpack::imm06_from_bytes(self.0)
            }
        }
    };
    ($Op:ident $ra:ident: RegId $rb:ident: RegId $imm:ident: Imm12) => {
        op_accessors!($Op $ra: RegId $rb: RegId);

        #[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
        impl $Op {
            /// Access the 12-bit immediate value.
            pub fn imm12(&self) -> Imm12 {
                unpack::imm12_from_bytes(self.0)
            }
        }
    };
    ($Op:ident $ra:ident: RegId $imm:ident: Imm18) => {
        op_accessors!($Op $ra: RegId);

        #[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
        impl $Op {
            /// Access the 18-bit immediate value.
            pub fn imm18(&self) -> Imm18 {
                unpack::imm18_from_bytes(self.0)
            }
        }
    };
    ($Op:ident $ra:ident: Imm24) => {
        #[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
        impl $Op {
            /// Access the 24-bit immediate value.
            pub fn imm24(&self) -> Imm24 {
                unpack::imm24_from_bytes(self.0)
            }
        }
    };
    ($Op:ident) => {};
}

// Generate a method for converting the instruction into its parts.
macro_rules! op_unpack {
    (RegId) => {
        /// Convert the instruction into its parts.
        pub fn unpack(self) -> RegId {
            unpack::ra_from_bytes(self.0)
        }
    };
    (RegId RegId) => {
        /// Convert the instruction into its parts.
        pub fn unpack(self) -> (RegId, RegId) {
            unpack::ra_rb_from_bytes(self.0)
        }
    };
    (RegId RegId RegId) => {
        /// Convert the instruction into its parts.
        pub fn unpack(self) -> (RegId, RegId, RegId) {
            unpack::ra_rb_rc_from_bytes(self.0)
        }
    };
    (RegId RegId RegId RegId) => {
        /// Convert the instruction into its parts.
        pub fn unpack(self) -> (RegId, RegId, RegId, RegId) {
            unpack::ra_rb_rc_rd_from_bytes(self.0)
        }
    };
    (RegId RegId RegId Imm06) => {
        /// Convert the instruction into its parts.
        pub fn unpack(self) -> (RegId, RegId, RegId, Imm06) {
            unpack::ra_rb_rc_imm06_from_bytes(self.0)
        }
    };
    (RegId RegId Imm12) => {
        /// Convert the instruction into its parts.
        pub fn unpack(self) -> (RegId, RegId, Imm12) {
            unpack::ra_rb_imm12_from_bytes(self.0)
        }
    };
    (RegId Imm18) => {
        /// Convert the instruction into its parts.
        pub fn unpack(self) -> (RegId, Imm18) {
            unpack::ra_imm18_from_bytes(self.0)
        }
    };
    (Imm24) => {
        /// Convert the instruction into its parts.
        pub fn unpack(self) -> Imm24 {
            unpack::imm24_from_bytes(self.0)
        }
    };
    () => {};
}

// Generate a method for checking that the reserved part of the
// instruction is zero. This is private, as invalid instructions
// cannot be constructed outside this crate.
macro_rules! op_reserved_part {
    (RegId) => {
        pub(crate) fn reserved_part_is_zero(self) -> bool {
            let (_, imm) = unpack::ra_imm18_from_bytes(self.0);
            imm.0 == 0
        }
    };
    (RegId RegId) => {
        pub(crate) fn reserved_part_is_zero(self) -> bool {
            let (_, _, imm) = unpack::ra_rb_imm12_from_bytes(self.0);
            imm.0 == 0
        }
    };
    (RegId RegId RegId) => {
        pub(crate) fn reserved_part_is_zero(self) -> bool {
            let (_, _, _, imm) = unpack::ra_rb_rc_imm06_from_bytes(self.0);
            imm.0 == 0
        }
    };
    (RegId RegId RegId RegId) => {
        pub(crate) fn reserved_part_is_zero(self) -> bool {
            true
        }
    };
    (RegId RegId RegId Imm06) => {
        pub(crate) fn reserved_part_is_zero(self) -> bool {
            true
        }
    };
    (RegId RegId Imm12) => {
        pub(crate) fn reserved_part_is_zero(self) -> bool {
            true
        }
    };
    (RegId Imm18) => {
        pub(crate) fn reserved_part_is_zero(self) -> bool {
            true
        }
    };
    (Imm24) => {
        pub(crate) fn reserved_part_is_zero(self) -> bool {
            true
        }
    };
    () => {
        pub(crate) fn reserved_part_is_zero(self) -> bool {
            self.0 == [0; 3]
        }
    };
}

// Generate a private fn for use within the `Instruction::reg_ids` implementation.
macro_rules! op_reg_ids {
    (RegId) => {
        pub(super) fn reg_ids(&self) -> [Option<RegId>; 4] {
            let ra = self.unpack();
            [Some(ra), None, None, None]
        }
    };
    (RegId RegId) => {
        pub(super) fn reg_ids(&self) -> [Option<RegId>; 4] {
            let (ra, rb) = self.unpack();
            [Some(ra), Some(rb), None, None]
        }
    };
    (RegId RegId RegId) => {
        pub(super) fn reg_ids(&self) -> [Option<RegId>; 4] {
            let (ra, rb, rc) = self.unpack();
            [Some(ra), Some(rb), Some(rc), None]
        }
    };
    (RegId RegId RegId RegId) => {
        pub(super) fn reg_ids(&self) -> [Option<RegId>; 4] {
            let (ra, rb, rc, rd) = self.unpack();
            [Some(ra), Some(rb), Some(rc), Some(rd)]
        }
    };
    (RegId RegId RegId Imm06) => {
        pub(super) fn reg_ids(&self) -> [Option<RegId>; 4] {
            let (ra, rb, rc, _) = self.unpack();
            [Some(ra), Some(rb), Some(rc), None]
        }
    };
    (RegId RegId Imm12) => {
        pub(super) fn reg_ids(&self) -> [Option<RegId>; 4] {
            let (ra, rb, _) = self.unpack();
            [Some(ra), Some(rb), None, None]
        }
    };
    (RegId Imm18) => {
        pub(super) fn reg_ids(&self) -> [Option<RegId>; 4] {
            let (ra, _) = self.unpack();
            [Some(ra), None, None, None]
        }
    };
    ($($rest:tt)*) => {
        pub(super) fn reg_ids(&self) -> [Option<RegId>; 4] {
            [None; 4]
        }
    };
}

// Generate test constructors that can be used to generate instructions from non-matching
// input.
#[cfg(test)]
macro_rules! op_test_construct_fn {
    (RegId) => {
        /// Construct the instruction from all possible raw fields, ignoring inapplicable
        /// ones.
        pub fn test_construct(
            ra: RegId,
            _rb: RegId,
            _rc: RegId,
            _rd: RegId,
            _imm: u32,
        ) -> Self {
            Self(pack::bytes_from_ra(ra))
        }
    };
    (RegId RegId) => {
        /// Construct the instruction from all possible raw fields, ignoring inapplicable
        /// ones.
        pub fn test_construct(
            ra: RegId,
            rb: RegId,
            _rc: RegId,
            _rd: RegId,
            _imm: u32,
        ) -> Self {
            Self(pack::bytes_from_ra_rb(ra, rb))
        }
    };
    (RegId RegId RegId) => {
        /// Construct the instruction from all possible raw fields, ignoring inapplicable
        /// ones.
        pub fn test_construct(
            ra: RegId,
            rb: RegId,
            rc: RegId,
            _rd: RegId,
            _imm: u32,
        ) -> Self {
            Self(pack::bytes_from_ra_rb_rc(ra, rb, rc))
        }
    };
    (RegId RegId RegId RegId) => {
        /// Construct the instruction from all possible raw fields, ignoring inapplicable
        /// ones.
        pub fn test_construct(
            ra: RegId,
            rb: RegId,
            rc: RegId,
            rd: RegId,
            _imm: u32,
        ) -> Self {
            Self(pack::bytes_from_ra_rb_rc_rd(ra, rb, rc, rd))
        }
    };
    (RegId RegId RegId Imm06) => {
        /// Construct the instruction from all possible raw fields, ignoring inapplicable
        /// ones.
        pub fn test_construct(
            ra: RegId,
            rb: RegId,
            rc: RegId,
            _rd: RegId,
            imm: u32,
        ) -> Self {
            Self(pack::bytes_from_ra_rb_rc_imm06(
                ra,
                rb,
                rc,
                Imm06::from(imm as u8),
            ))
        }
    };
    (RegId RegId Imm12) => {
        /// Construct the instruction from all possible raw fields, ignoring inapplicable
        /// ones.
        pub fn test_construct(
            ra: RegId,
            rb: RegId,
            _rc: RegId,
            _rd: RegId,
            imm: u32,
        ) -> Self {
            Self(pack::bytes_from_ra_rb_imm12(
                ra,
                rb,
                Imm12::from(imm as u16),
            ))
        }
    };
    (RegId Imm18) => {
        /// Construct the instruction from all possible raw fields, ignoring inapplicable
        /// ones.
        pub fn test_construct(
            ra: RegId,
            _rb: RegId,
            _rc: RegId,
            _rd: RegId,
            imm: u32,
        ) -> Self {
            Self(pack::bytes_from_ra_imm18(ra, Imm18::from(imm)))
        }
    };
    (Imm24) => {
        /// Construct the instruction from all possible raw fields, ignoring inapplicable
        /// ones.
        pub fn test_construct(
            _ra: RegId,
            _rb: RegId,
            _rc: RegId,
            _rd: RegId,
            imm: u32,
        ) -> Self {
            Self(pack::bytes_from_imm24(Imm24::from(imm)))
        }
    };
    () => {
        /// Construct the instruction from all possible raw fields, ignoring inapplicable
        /// ones.
        #[allow(clippy::new_without_default)]
        pub fn test_construct(
            _ra: RegId,
            _rb: RegId,
            _rc: RegId,
            _rd: RegId,
            _imm: u32,
        ) -> Self {
            Self([0; 3])
        }
    };
}

// Debug implementations for each instruction.
macro_rules! op_debug_fmt {
    ($Op:ident[$ra:ident : RegId]) => {
        fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            let ra = self.unpack();
            f.debug_struct(stringify!($Op))
                .field(stringify!($ra), &format_args!("{:#02x}", u8::from(ra)))
                .finish()
        }
    };
    ($Op:ident[$ra:ident : RegId $rb:ident : RegId]) => {
        fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            let (ra, rb) = self.unpack();
            f.debug_struct(stringify!($Op))
                .field(stringify!($ra), &format_args!("{:#02x}", u8::from(ra)))
                .field(stringify!($rb), &format_args!("{:#02x}", u8::from(rb)))
                .finish()
        }
    };
    ($Op:ident[$ra:ident : RegId $rb:ident : RegId $rc:ident : RegId]) => {
        fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            let (ra, rb, rc) = self.unpack();
            f.debug_struct(stringify!($Op))
                .field(stringify!($ra), &format_args!("{:#02x}", u8::from(ra)))
                .field(stringify!($rb), &format_args!("{:#02x}", u8::from(rb)))
                .field(stringify!($rc), &format_args!("{:#02x}", u8::from(rc)))
                .finish()
        }
    };
    (
        $Op:ident[$ra:ident : RegId $rb:ident : RegId $rc:ident : RegId $rd:ident : RegId]
    ) => {
        fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            let (ra, rb, rc, rd) = self.unpack();
            f.debug_struct(stringify!($Op))
                .field(stringify!($ra), &format_args!("{:#02x}", u8::from(ra)))
                .field(stringify!($rb), &format_args!("{:#02x}", u8::from(rb)))
                .field(stringify!($rc), &format_args!("{:#02x}", u8::from(rc)))
                .field(stringify!($rd), &format_args!("{:#02x}", u8::from(rd)))
                .finish()
        }
    };
    (
        $Op:ident[$ra:ident : RegId $rb:ident : RegId $rc:ident : RegId $imm:ident : Imm06]
    ) => {
        fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            let (ra, rb, rc, imm) = self.unpack();
            f.debug_struct(stringify!($Op))
                .field(stringify!($ra), &format_args!("{:#02x}", u8::from(ra)))
                .field(stringify!($rb), &format_args!("{:#02x}", u8::from(rb)))
                .field(stringify!($rc), &format_args!("{:#02x}", u8::from(rc)))
                .field(stringify!($imm), &u8::from(imm))
                .finish()
        }
    };
    ($Op:ident[$ra:ident : RegId $rb:ident : RegId $imm:ident : Imm12]) => {
        fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            let (ra, rb, imm) = self.unpack();
            f.debug_struct(stringify!($Op))
                .field(stringify!($ra), &format_args!("{:#02x}", u8::from(ra)))
                .field(stringify!($rb), &format_args!("{:#02x}", u8::from(rb)))
                .field(stringify!($imm), &u16::from(imm))
                .finish()
        }
    };
    ($Op:ident[$ra:ident : RegId $imm:ident : Imm18]) => {
        fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            let (ra, imm) = self.unpack();
            f.debug_struct(stringify!($Op))
                .field(stringify!($ra), &format_args!("{:#02x}", u8::from(ra)))
                .field(stringify!($imm), &u32::from(imm))
                .finish()
        }
    };
    ($Op:ident[$imm:ident : Imm24]) => {
        fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            let imm = self.unpack();
            f.debug_struct(stringify!($Op))
                .field(stringify!($imm), &u32::from(imm))
                .finish()
        }
    };
    ($Op:ident[]) => {
        fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            f.debug_struct(stringify!($Op)).finish()
        }
    };
}

// Recursively declares a unique struct for each opcode.
macro_rules! decl_op_struct {
    ($doc:literal $ix:literal $Op:ident $op:ident [$($fname:ident: $field:ident)*] $($rest:tt)*) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Eq, Hash, PartialEq)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
        pub struct $Op(pub (super) [u8; 3]);
        decl_op_struct!($($rest)*);
    };
    () => {};
}

/// This macro is intentionaly private. See the module-level documentation for a thorough
/// explanation of how this macro works.
macro_rules! impl_instructions {
    // Define the `Opcode` enum.
    (decl_opcode_enum $($doc:literal $ix:literal $Op:ident $op:ident [$($fname:ident: $field:ident)*])*) => {
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

    // Define the `Instruction` enum.
    (decl_instruction_enum $($doc:literal $ix:literal $Op:ident $op:ident [$($fname:ident: $field:ident)*])*) => {
        /// Representation of a single instruction for the interpreter.
        ///
        /// The opcode is represented in the tag (variant), or may be retrieved in the form of an
        /// `Opcode` byte using the `opcode` method.
        ///
        /// The register and immediate data associated with the instruction is represented within
        /// an inner unit type wrapper around the 3 remaining bytes.
        #[derive(Clone, Copy, Eq, Hash, PartialEq)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub enum Instruction {
            $(
                #[doc = $doc]
                $Op(op::$Op),
            )*
        }
    };

    // Recursively generate a test constructor for each opcode
    (impl_opcode_test_construct $doc:literal $ix:literal $Op:ident $op:ident [$($fname:ident: $field:ident)*] $($rest:tt)*) => {
        #[cfg(test)]
        #[allow(clippy::cast_possible_truncation)]
        impl crate::_op::$Op {
            op_test_construct_fn!($($field)*);
        }
        impl_instructions!(impl_opcode_test_construct $($rest)*);
    };
    (impl_opcode_test_construct) => {};

    // Recursively generate a test constructor for each opcode
    (tests $doc:literal $ix:literal $Op:ident $op:ident [$($fname:ident: $field:ident)*] $($rest:tt)*) => {
        op_test!($Op $op [$($field)*]);
        impl_instructions!(tests $($rest)*);
    };
    (tests) => {};

    // Implement constructors and accessors for register and immediate values.
    (impl_op $doc:literal $ix:literal $Op:ident $op:ident [$($fname:ident: $field:ident)*] $($rest:tt)*) => {
        impl $Op {
            /// The associated 8-bit Opcode value.
            pub const OPCODE: Opcode = Opcode::$Op;
        }

        op_new!($Op $($fname: $field)*);
        op_accessors!($Op $($fname: $field)*);

        impl $Op {
            op_unpack!($($field)*);
            op_reserved_part!($($field)*);
            op_reg_ids!($($field)*);
        }

        op_constructor!($doc $Op $op [$($fname: $field)*]);

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

        #[cfg(feature = "typescript")]
        impl From<$Op> for typescript::Instruction {
            fn from(opcode: $Op) -> Self {
                typescript::Instruction::new(opcode.into())
            }
        }

        impl core::fmt::Debug for $Op {
            op_debug_fmt!($Op [$($fname: $field)*]);
        }

        impl_instructions!(impl_op $($rest)*);
    };
    (impl_op) => {};

    // Implement functions for all opcode variants
    (impl_opcode $($doc:literal $ix:literal $Op:ident $op:ident [$($fname:ident: $field:ident)*])*) => {
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

        impl Opcode {
            /// Construct the instruction from all possible raw fields, ignoring inapplicable ones.
            #[cfg(test)]
            pub fn test_construct(self, ra: RegId, rb: RegId, rc: RegId, rd: RegId, imm: u32) -> Instruction {
                match self {
                    $(
                        Self::$Op => Instruction::$Op(crate::_op::$Op::test_construct(ra, rb, rc, rd, imm)),
                    )*
                }
            }
        }
    };

    // Implement accessors for register and immediate values.
    (impl_instruction $($doc:literal $ix:literal $Op:ident $op:ident [$($fname:ident: $field:ident)*])*) => {
        impl Instruction {
            /// This instruction's opcode.
            pub fn opcode(&self) -> Opcode {
                match self {
                    $(
                        Self::$Op(_) => Opcode::$Op,
                    )*
                }
            }

            /// Unpacks all register IDs into a slice of options.
            pub fn reg_ids(&self) -> [Option<RegId>; 4] {
                match self {
                    $(
                        Self::$Op(op) => op.reg_ids(),
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

        #[cfg(feature = "typescript")]
        impl From<Instruction> for typescript::Instruction {
            fn from(inst: Instruction) -> Self {
                typescript::Instruction::new(inst)
            }
        }

        impl core::convert::TryFrom<[u8; 4]> for Instruction {
            type Error = InvalidOpcode;
            fn try_from([op, a, b, c]: [u8; 4]) -> Result<Self, Self::Error> {
                let op = match op {
                    $(
                        $ix => {
                            let op = op::$Op([a, b, c]);
                            if !op.reserved_part_is_zero() {
                                return Err(InvalidOpcode);
                            }

                            Self::$Op(op)
                        },
                    )*
                    _ => return Err(InvalidOpcode),
                };

                Ok(op)
            }
        }

        impl core::fmt::Debug for Instruction {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                match self {
                    $(
                        Self::$Op(op) => op.fmt(f),
                    )*
                }
            }
        }
    };

    // Entrypoint to the macro, generates structs, methods, opcode enum and instruction enum
    // separately.
    ($($tts:tt)*) => {
        mod _op {
            use super::*;
            decl_op_struct!($($tts)*);
            impl_instructions!(impl_op $($tts)*);
        }
        impl_instructions!(decl_opcode_enum $($tts)*);
        impl_instructions!(decl_instruction_enum $($tts)*);
        impl_instructions!(impl_opcode $($tts)*);
        impl_instructions!(impl_instruction $($tts)*);
        impl_instructions!(impl_opcode_test_construct $($tts)*);


        #[cfg(test)]
        mod opcode_tests {
            use super::*;
            impl_instructions!(tests $($tts)*);
        }
    };
}

/// Defines the enum with `TryFrom` trait implementation.
#[macro_export]
macro_rules! enum_try_from {
    (
        $(#[$meta:meta])* $vis:vis enum $name:ident {
            $($(#[$vmeta:meta])* $vname:ident $(= $val:expr)?,)*
        },
        $from:ident
    ) => {
        $(#[$meta])*
        $vis enum $name {
            $($(#[$vmeta])* $vname $(= $val)?,)*
        }

        impl core::convert::TryFrom<$from> for $name {
            type Error = $crate::PanicReason;

            fn try_from(v: $from) -> Result<Self, Self::Error> {
                match v {
                    $(x if x == $name::$vname as $from => Ok($name::$vname),)*
                    _ => Err($crate::PanicReason::InvalidMetadataIdentifier),
                }
            }
        }
    }
}

#[cfg(test)]
// Generate a test for the instruction.
macro_rules! op_test {
    ($Op:ident $op:ident[RegId]) => {
        #[test]
        fn $op() {
            crate::macros::test_reserved_part(Opcode::$Op, true, false, false, false);
        }
    };
    ($Op:ident $op:ident[RegId RegId]) => {
        #[test]
        fn $op() {
            crate::macros::test_reserved_part(Opcode::$Op, true, true, false, false);
        }
    };
    ($Op:ident $op:ident[RegId RegId RegId]) => {
        #[test]
        fn $op() {
            crate::macros::test_reserved_part(Opcode::$Op, true, true, true, false);
        }
    };
    ($Op:ident $op:ident[RegId RegId RegId RegId]) => {
        #[test]
        fn $op() {
            crate::macros::test_reserved_part(Opcode::$Op, true, true, true, true);
        }
    };
    ($Op:ident $op:ident[RegId RegId RegId Imm06]) => {
        #[test]
        fn $op() {
            crate::macros::test_reserved_part(Opcode::$Op, true, true, true, true);
        }
    };
    ($Op:ident $op:ident[RegId RegId Imm12]) => {
        #[test]
        fn $op() {
            crate::macros::test_reserved_part(Opcode::$Op, true, true, true, true);
        }
    };
    ($Op:ident $op:ident[RegId Imm18]) => {
        #[test]
        fn $op() {
            crate::macros::test_reserved_part(Opcode::$Op, true, true, true, true);
        }
    };
    ($Op:ident $op:ident[Imm24]) => {
        #[test]
        fn $op() {
            crate::macros::test_reserved_part(Opcode::$Op, true, true, true, true);
        }
    };
    ($Op:ident $op:ident[]) => {
        #[test]
        fn $op() {
            crate::macros::test_reserved_part(Opcode::$Op, false, false, false, false);
        }
    };
}

#[cfg(test)]
fn bytes(a: u8, b: u8, c: u8, d: u8) -> [u8; 3] {
    use crate::RegId;
    crate::pack::bytes_from_ra_rb_rc_rd(
        RegId::new(a),
        RegId::new(b),
        RegId::new(c),
        RegId::new(d),
    )
}

#[cfg(test)]
pub(crate) fn test_reserved_part(
    opcode: crate::Opcode,
    zero_should_pass: bool,
    first_should_pass: bool,
    second_should_pass: bool,
    third_should_pass: bool,
) {
    use crate::Instruction;

    // Args: 0
    let [a, b, c] = bytes(0, 0, 0, 0);
    Instruction::try_from([opcode as u8, a, b, c]).unwrap();
    let [a, b, c] = bytes(1, 0, 0, 0);
    let zero_is_error = Instruction::try_from([opcode as u8, a, b, c]).is_ok();
    assert_eq!(
        zero_should_pass, zero_is_error,
        "Opcode: {opcode:?} failed zero"
    );

    // Args: 1
    let [a, b, c] = bytes(0, 0, 0, 0);
    Instruction::try_from([opcode as u8, a, b, c]).unwrap();
    let [a, b, c] = bytes(0, 1, 0, 0);
    let first_is_error = Instruction::try_from([opcode as u8, a, b, c]).is_ok();
    assert_eq!(
        first_should_pass, first_is_error,
        "Opcode: {opcode:?} failed first"
    );

    // Args: 2
    let [a, b, c] = bytes(0, 0, 0, 0);
    Instruction::try_from([opcode as u8, a, b, c]).unwrap();
    let [a, b, c] = bytes(0, 0, 1, 0);
    let second_is_error = Instruction::try_from([opcode as u8, a, b, c]).is_ok();
    assert_eq!(
        second_should_pass, second_is_error,
        "Opcode: {opcode:?} failed second"
    );

    // Args: 3
    let [a, b, c] = bytes(0, 0, 0, 0);
    Instruction::try_from([opcode as u8, a, b, c]).unwrap();
    let [a, b, c] = bytes(0, 0, 0, 1);
    let third_is_error = Instruction::try_from([opcode as u8, a, b, c]).is_ok();
    assert_eq!(
        third_should_pass, third_is_error,
        "Opcode: {opcode:?} failed third"
    );
}
