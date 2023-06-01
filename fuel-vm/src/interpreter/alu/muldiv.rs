use super::super::{ExecutableTransaction, Interpreter};
use crate::{constraints::reg_key::*, error::RuntimeError};

use fuel_asm::PanicReason;
use fuel_types::{RegisterId, Word};

impl<S, Tx> Interpreter<S, Tx>
where
    Tx: ExecutableTransaction,
{
    /// Stores the overflowed wrapped value into RegId::OF
    pub(crate) fn alu_muldiv(
        &mut self,
        ra: RegisterId,
        lhs: Word,
        rhs: Word,
        divider: Word,
    ) -> Result<(), RuntimeError> {
        let wrk = WriteRegKey::try_from(ra)?;
        let (result, overflow) = muldiv(lhs, rhs, divider);

        if overflow != 0 && !self.flag_wrapping() {
            return Err(PanicReason::ArithmeticOverflow.into());
        }

        let (SystemRegisters { mut of, mut err, .. }, mut w) = split_registers(&mut self.registers);
        *of = overflow;
        *err = 0;
        w[wrk] = result;

        Ok(())
    }
}

/// Fused multiply-divide with arbitrary precision intermediate result.
/// Returns `(result, overflow)`.
/// Divider 0 is treated as `1<<64`.
pub(crate) fn muldiv(lhs: u64, rhs: u64, divider: u64) -> (u64, u64) {
    let intermediate = lhs as u128 * rhs as u128; // Never overflows
    if divider == 0 {
        ((intermediate >> 64) as u64, 0)
    } else {
        let result = intermediate / (divider as u128);
        (result as u64, (result >> 64) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rstest::rstest]
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
