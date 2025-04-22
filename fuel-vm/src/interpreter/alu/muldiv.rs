use super::super::{
    ExecutableTransaction,
    Interpreter,
    internal::inc_pc,
    is_wrapping,
};
use crate::{
    constraints::reg_key::*,
    error::SimpleResult,
};

use fuel_asm::{
    PanicReason,
    RegId,
};
use fuel_types::Word;

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    Tx: ExecutableTransaction,
{
    /// Stores the overflowed wrapped value into RegId::OF
    pub(crate) fn alu_muldiv(
        &mut self,
        ra: RegId,
        lhs: Word,
        rhs: Word,
        divider: Word,
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
        let dest = &mut w[ra.try_into()?];

        let (result, overflow) = muldiv(lhs, rhs, divider);

        if overflow != 0 && !is_wrapping(flag.into()) {
            return Err(PanicReason::ArithmeticOverflow.into())
        }

        *of = overflow;
        *err = 0;
        *dest = result;

        inc_pc(pc)?;
        Ok(())
    }
}

/// Fused multiply-divide with arbitrary precision intermediate result.
/// Returns `(result, overflow)`.
/// Divider 0 is treated as `1<<64`.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn muldiv(lhs: u64, rhs: u64, divider: u64) -> (u64, u64) {
    // Widen all inputs so we never overflow
    let lhs = lhs as u128;
    let rhs = rhs as u128;
    let divider = divider as u128;

    // Compute intermediate result
    let intermediate = lhs
        .checked_mul(rhs)
        .expect("Cannot overflow as we have enough bits");

    // Divide
    if let Some(result) = intermediate.checked_div(divider) {
        // We want to truncate the `result` here and return a non-empty `overflow`.
        (result as u64, (result >> 64) as u64)
    } else {
        ((intermediate >> 64) as u64, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rstest::rstest]
    #[case(0, 0, 0, 0, 0)]
    #[case(0, 0, 1, 0, 0)]
    #[case(0, 5, 1, 0, 0)]
    #[case(9, 9, 1, 81, 0)]
    #[case(9, 9, 2, 40, 0)]
    #[case(9, 9, 3, 27, 0)]
    #[case(9, 9, 4, 20, 0)]
    #[case(u64::MAX, 5, 10, u64::MAX / 2, 0)]
    #[case(u64::MAX, 2, 6, u64::MAX / 3, 0)]
    #[case(u64::MAX, u64::MAX, u64::MAX, u64::MAX, 0)]
    #[case(u64::MAX, 4, 2, 0xfffffffffffffffe, 1)]
    #[case(u64::MAX, 3, 2, 0x7ffffffffffffffe, 1)]
    fn fused_muldiv(
        #[case] lhs: u64,
        #[case] rhs: u64,
        #[case] divisor: u64,
        #[case] expected: u64,
        #[case] expected_overflow: u64,
    ) {
        let (result, overflow) = muldiv(lhs, rhs, divisor);
        assert_eq!(result, expected);
        assert_eq!(overflow, expected_overflow);
    }
}
