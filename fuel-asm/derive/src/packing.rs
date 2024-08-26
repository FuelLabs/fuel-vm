/// The shift amount for the given argument index, from left to right.
/// The input must be a valid argument index (0..=3).
#[allow(clippy::arithmetic_side_effects)] // Contract, double-checked with an assertion
pub fn argument_offset(i: usize) -> usize {
    assert!(i <= 3);
    6 * (3 - i)
}
