//! VM parameters

use fuel_types::{Bytes32, Word};

use std::mem;

/* FLAG AND REGISTER TYPES */

/// Register count for checking constraints
pub const VM_REGISTER_COUNT: usize = 64;

/// Contains zero (0), for convenience.
pub const REG_ZERO: usize = 0x00;

/// Contains one (1), for convenience.
pub const REG_ONE: usize = 0x01;

/// Contains overflow/underflow of addition, subtraction, and multiplication.
pub const REG_OF: usize = 0x02;

/// The program counter. Memory address of the current instruction.
pub const REG_PC: usize = 0x03;

/// Memory address of bottom of current writable stack area.
pub const REG_SSP: usize = 0x04;

/// Memory address on top of current writable stack area (points to free
/// memory).
pub const REG_SP: usize = 0x05;

/// Memory address of beginning of current call frame.
pub const REG_FP: usize = 0x06;

/// Memory address below the current bottom of the heap (points to free memory).
pub const REG_HP: usize = 0x07;

/// Error codes for particular operations.
pub const REG_ERR: usize = 0x08;

/// Remaining gas globally.
pub const REG_GGAS: usize = 0x09;

/// Remaining gas in the context.
pub const REG_CGAS: usize = 0x0a;

/// Received balance for this context.
pub const REG_BAL: usize = 0x0b;

/// Pointer to the start of the currently-executing code.
pub const REG_IS: usize = 0x0c;

/// Return value or pointer.
pub const REG_RET: usize = 0x0d;

/// Return value length in bytes.
pub const REG_RETL: usize = 0x0e;

/// Flags register.
pub const REG_FLAG: usize = 0x0f;

/// Smallest writable register
pub const REG_WRITABLE: usize = 0x10;

/* END */

/* MEMORY TYPES */

/// Length of a word, in bytes
pub const WORD_SIZE: usize = mem::size_of::<Word>();

/// Maximum memory word count
pub const FUEL_MAX_MEMORY_SIZE: u64 = 64;

/// Maximum VM RAM, in bytes.
pub const VM_MAX_RAM: u64 = 16 * 1024 * FUEL_MAX_MEMORY_SIZE * WORD_SIZE as u64;

/// Maximum memory access size, in bytes.
pub const MEM_MAX_ACCESS_SIZE: u64 = VM_MAX_RAM;

/// Maximum size of a fuel program (unused)
pub const FUEL_MAX_PROGRAM_SIZE: usize = 16 * /* KB */ 1024;

// no limits to heap for now.

/// Encoded len of a register id in an instruction (unused)
pub const VM_REGISTER_WIDTH: u8 = 6;

/// Transaction offset in the VM memory
pub const VM_TX_MEMORY: usize = Bytes32::LEN // Tx ID
            + WORD_SIZE; // Tx size

/// Empty merkle root for receipts tree
pub const EMPTY_RECEIPTS_MERKLE_ROOT: [u8; 32] = [
    0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9, 0x24, 0x27, 0xae, 0x41,
    0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52, 0xb8, 0x55,
];
