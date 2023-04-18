use super::super::{internal::inc_pc, is_wrapping, ExecutableTransaction, Interpreter};
use crate::{constraints::reg_key::*, error::RuntimeError};

use fuel_asm::PanicReason;
use fuel_types::{RegisterId, Word};

impl<S, Tx> Interpreter<S, Tx>
where
    Tx: ExecutableTransaction,
{
    /// Stores the overflowed wrapped value into RegId::OF
    pub(crate) fn alu_muldiv(&mut self, ra: RegisterId, lhs: Word, rhs: Word, divider: Word) -> Result<(), RuntimeError> {
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

        let MulDivResult { wrapped, overflow } = muldiv(lhs, rhs, divider);

        if overflow != 0 && !is_wrapping(flag.into()) {
            return Err(PanicReason::ArithmeticOverflow.into());
        }

        *of = overflow;
        *err = 0;
        *dest = wrapped;

        inc_pc(pc)
    }
}

pub(crate) struct MulDivResult {
    /// Low byte after division. If there was no overflow, this is the actual result.
    pub wrapped: u64,
    /// The result of division that doesn't fit into `u64`.
    pub overflow: u64,
}

/// Fused multiply-divide with arbitrary precision intermediate result.
/// Returns `(wrapped, overflow)`.
/// Divider 0 is treated as `1<<64`.
pub(crate) fn muldiv(lhs: u64, rhs: u64, divider: u64) -> MulDivResult {
    let intermediate = lhs as u128 * rhs as u128; // Never overflows
    if divider == 0 {
        MulDivResult {
            wrapped: (intermediate >> 64) as u64,
            overflow: 0,
        }
    } else {
        let d = divider as u128;
        MulDivResult {
            wrapped: (intermediate / d) as u64,
            overflow: (intermediate % d) as u64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[rstest::rstest]
    #[case(0, 0, 1, 0)]
    #[case(0, 5, 1, 0)]
    #[case(9, 9, 1, 81)]
    #[case(9, 9, 2, 40)]
    #[case(9, 9, 3, 27)]
    #[case(9, 9, 4, 20)]
    #[case(u64::MAX, 5, 10, u64::MAX / 2)]
    #[case(u64::MAX, 2, 6, u64::MAX / 3)]
    #[case(u64::MAX, u64::MAX, u64::MAX, u64::MAX)]
    fn fused_muldiv(#[case] lhs: u64, #[case] rhs: u64, #[case] divisor: u64, #[case] expected: u64) {
        let result = muldiv(lhs, rhs, divisor);
        assert_eq!(result.wrapped, expected);
    }
}