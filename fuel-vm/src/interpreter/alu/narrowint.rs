//! Operations on subword integers u8, u16, u32.

use fuel_asm::{
    PanicReason,
    RegId,
    narrowint::*,
};
use fuel_types::Word;

use super::super::{
    ExecutableTransaction,
    Interpreter,
    internal::inc_pc,
    is_wrapping,
};
use crate::{
    constraints::reg_key::*,
    error::SimpleResult,
    interpreter::Memory,
};

/// Split the subword integer into two parts.
/// The left part is the value of the subword,
/// while the right part is the overflow amount.
fn split_overflow(value: u64, width: OpWidth) -> (u64, u64) {
    match width {
        OpWidth::U8 => (value & (u8::MAX as u64), value >> 8),
        OpWidth::U16 => (value & (u16::MAX as u64), value >> 16),
        OpWidth::U32 => (value & (u32::MAX as u64), value >> 32),
    }
}

/// Truncate the subword integer to the specified width.
fn truncate(value: u64, width: OpWidth) -> u64 {
    split_overflow(value, width).0
}

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    M: Memory,
    Tx: ExecutableTransaction,
{
    pub(crate) fn alu_narrowint_op(
        &mut self,
        dst: RegId,
        lhs: Word,
        rhs: Word,
        args: MathArgs,
    ) -> SimpleResult<()> {
        let (
            SystemRegisters {
                flag,
                mut of,
                mut err,
                pc,
                ..
            },
            mut w,
        ) = split_registers(&mut self.registers);
        let dst: &mut Word = &mut w[dst.try_into()?];

        // Truncate the operands to the specified width.
        let lhs = truncate(lhs, args.width);
        let rhs = truncate(rhs, args.width);

        #[allow(clippy::cast_possible_truncation)] // Already truncated above
        let rhs_u32 = rhs as u32;

        // Perform the operation.
        // We use raw arithmetic add and multiply operators here,
        // as the truncated values cannot overflow.
        #[allow(clippy::arithmetic_side_effects)]
        let (wrapped, overflow) = match args.op {
            MathOp::ADD => split_overflow(lhs + rhs, args.width),
            MathOp::MUL => split_overflow(lhs * rhs, args.width),
            MathOp::EXP => match lhs.checked_pow(rhs_u32) {
                Some(v) => {
                    let (wrapped, overflow) = split_overflow(v, args.width);
                    if overflow != 0 { (0, 1) } else { (wrapped, 0) }
                }
                None => (0, 1),
            },
            MathOp::SLL => (
                truncate(lhs.checked_shl(rhs_u32).unwrap_or(0), args.width),
                0,
            ),
            MathOp::XNOR => (truncate(lhs ^ !rhs, args.width), 0),
        };

        if overflow != 0 && !is_wrapping(flag.into()) {
            return Err(PanicReason::ArithmeticOverflow.into());
        }

        *dst = wrapped;
        *of = overflow;
        *err = 0;

        inc_pc(pc)?;
        Ok(())
    }
}
