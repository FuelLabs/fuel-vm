//! Arithmetic functions for the Fuel VM

use fuel_asm::{
    PanicReason,
    Word,
};

use crate::error::RuntimeError;

/// Add two unchecked words, returning an error if overflow
#[inline(always)]
pub fn checked_add_word(a: Word, b: Word) -> Result<Word, PanicReason> {
    a.checked_add(b).ok_or(PanicReason::ArithmeticOverflow)
}

/// Subtract two unchecked words, returning an error if overflow
#[inline(always)]
pub fn checked_sub_word(a: Word, b: Word) -> Result<Word, PanicReason> {
    a.checked_sub(b).ok_or(PanicReason::ArithmeticOverflow)
}

/// Add two unchecked numbers, returning an error if overflow
#[inline(always)]
pub fn checked_add_usize(a: usize, b: usize) -> Result<usize, PanicReason> {
    a.checked_add(b).ok_or(PanicReason::ArithmeticOverflow)
}

/// Subtract two unchecked numbers, returning an error if overflow
#[inline(always)]
pub fn checked_sub_usize(a: usize, b: usize) -> Result<usize, PanicReason> {
    a.checked_sub(b).ok_or(PanicReason::ArithmeticOverflow)
}

/// Add two checked words. Might wrap if overflow
///
/// The function is inlined so it will be optimized to `a + b` if `optimized` feature is
/// enabled so the operation is unchecked.
///
/// This should be used in contexts that are checked and guaranteed by the protocol to
/// never overflow, but then they might due to some bug in the code.
#[inline(always)]
pub fn add_word(a: Word, b: Word) -> Result<Word, RuntimeError> {
    #[cfg(feature = "optimized")]
    #[allow(clippy::arithmetic_side_effects)]
    {
        Ok(a + b)
    }

    #[cfg(not(feature = "optimized"))]
    {
        a.checked_add(b)
            .ok_or_else(|| RuntimeError::unexpected_behavior("unexpected overflow"))
    }
}

/// Subtract two checked words. Might wrap if overflow
///
/// The function is inlined so it will be optimized to `a + b` if `optimized` feature is
/// enabled so the operation is unchecked.
///
/// This should be used in contexts that are checked and guaranteed by the protocol to
/// never overflow, but then they might due to some bug in the code.
#[inline(always)]
#[allow(clippy::arithmetic_side_effects)]
pub fn sub_word(a: Word, b: Word) -> Result<Word, RuntimeError> {
    #[cfg(feature = "optimized")]
    {
        Ok(a - b)
    }

    #[cfg(not(feature = "optimized"))]
    {
        a.checked_sub(b)
            .ok_or_else(|| RuntimeError::unexpected_behavior("unexpected underflow"))
    }
}

/// Add two numbers. Should be used only in compile-time evaluations so the code won't
/// compile in case of unexpected overflow.
#[inline(always)]
#[allow(clippy::arithmetic_side_effects)]
pub const fn add_usize(a: usize, b: usize) -> usize {
    a + b
}
