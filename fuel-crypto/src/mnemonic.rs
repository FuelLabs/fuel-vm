/// FuelMnemonic is a simple mnemonic phrase generator.
pub struct FuelMnemonic;

#[cfg(all(feature = "std", feature = "random"))]
mod use_std {
    use super::FuelMnemonic;
    use crate::Error;
    use coins_bip39::{English, Mnemonic};

    pub type W = English;

    use rand::Rng;

    impl FuelMnemonic {
        /// Generates a random mnemonic phrase given a random number generator and
        /// the number of words to generate, `count`.
        pub fn generate_mnemonic_phrase<R: Rng>(rng: &mut R, count: usize) -> Result<String, Error> {
            Ok(Mnemonic::<W>::new_with_count(rng, count)?.to_phrase()?)
        }
    }
}
