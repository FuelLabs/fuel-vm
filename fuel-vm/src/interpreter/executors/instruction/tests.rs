#![allow(clippy::cast_possible_truncation)]
use super::*;
use fuel_asm::{
    op,
    Instruction,
    Opcode,
    PanicReason::ReservedRegisterNotWritable,
};

mod math_operations;
mod reserved_registers;
