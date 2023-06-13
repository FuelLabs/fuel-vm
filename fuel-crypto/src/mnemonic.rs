#![cfg(all(feature = "std", feature = "random"))]

use crate::Error;
use coins_bip39::{
    English,
    Mnemonic,
};
use rand::Rng;

/// Generates a random mnemonic phrase given a random number generator and
/// the number of words to generate, `count`.
pub fn generate_mnemonic_phrase<R: Rng>(
    rng: &mut R,
    count: usize,
) -> Result<String, Error> {
    Ok(Mnemonic::<English>::new_with_count(rng, count)?.to_phrase())
}
