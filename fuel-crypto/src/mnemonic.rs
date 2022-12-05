/// FuelMnemonic is a simple mnemonic phrase generator.
pub struct FuelMnemonic;

#[cfg(feature = "std")]
mod use_std {
    use super::FuelMnemonic;
    use crate::Error;
    use coins_bip39::{English, Mnemonic};

    pub type W = English;

    #[cfg(feature = "random")]
    use rand::Rng;

    impl FuelMnemonic {
        /// Generates a random mnemonic phrase given a random number generator and
        /// the number of words to generate, `count`.
        #[cfg(feature = "random")]
        pub fn generate_mnemonic_phrase<R: Rng>(
            rng: &mut R,
            count: usize,
        ) -> Result<String, Error> {
            Ok(Mnemonic::<W>::new_with_count(rng, count)?.to_phrase()?)
        }
    }
}
