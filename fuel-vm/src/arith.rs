//! Arithmetic functions for the Fuel VM

use fuel_asm::{
    PanicReason,
    Word,
};

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
