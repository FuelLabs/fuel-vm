#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use rand::Rng;

#[cfg(feature = "std")]
pub use use_std::*;

use alloc::vec::Vec;

pub fn generate_nonempty_bytes<R>(rng: &mut R) -> Vec<u8>
where
    R: Rng,
{
    let len = rng.gen_range(1..512);

    let mut data = alloc::vec![0u8; len];
    rng.fill_bytes(data.as_mut_slice());

    data.into()
}

pub fn generate_bytes<R>(rng: &mut R) -> Vec<u8>
where
    R: Rng,
{
    let len = rng.gen_range(0..512);

    let mut data = alloc::vec![0u8; len];
    rng.fill_bytes(data.as_mut_slice());

    data.into()
}

#[cfg(feature = "std")]
mod use_std {
    use fuel_crypto::SecretKey;
    use fuel_tx::{Input, Output, Transaction, TransactionBuilder};
    use fuel_types::bytes::Deserializable;
    use rand::distributions::{Distribution, Uniform};
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    use crate::{generate_bytes, generate_nonempty_bytes};

    pub struct TransactionFactory<R>
    where
        R: Rng,
    {
        rng: R,
        input_sampler: Uniform<usize>,
        output_sampler: Uniform<usize>,
        tx_sampler: Uniform<usize>,
    }

    impl<R> From<R> for TransactionFactory<R>
    where
        R: Rng,
    {
        fn from(rng: R) -> Self {
            let tx_sampler = Uniform::from(0..2);
            let input_sampler = Uniform::from(0..3);
            let output_sampler = Uniform::from(0..6);

            // Trick to enforce coverage of all variants in compile-time
            //
            // When and if a new variant is added, this implementation enforces it will be
            // listed here.
            debug_assert!({
                Input::from_bytes(&[])
                    .map(|i| match i {
                        Input::CoinSigned { .. } => (),
                        Input::CoinPredicate { .. } => (),
                        Input::Contract { .. } => (),
                    })
                    .unwrap_or(());

                Output::from_bytes(&[])
                    .map(|o| match o {
                        Output::Coin { .. } => (),
                        Output::Contract { .. } => (),
                        Output::Withdrawal { .. } => (),
                        Output::Change { .. } => (),
                        Output::Variable { .. } => (),
                        Output::ContractCreated { .. } => (),
                    })
                    .unwrap_or(());

                Transaction::from_bytes(&[])
                    .map(|t| match t {
                        Transaction::Script { .. } => (),
                        Transaction::Create { .. } => (),
                    })
                    .unwrap_or(());

                true
            });

            Self {
                rng,
                tx_sampler,
                input_sampler,
                output_sampler,
            }
        }
    }

    impl TransactionFactory<StdRng> {
        pub fn from_seed(seed: u64) -> Self {
            StdRng::seed_from_u64(seed).into()
        }
    }

    impl<R> TransactionFactory<R>
    where
        R: Rng,
    {
        pub fn transaction(&mut self) -> Transaction {
            self.transaction_with_keys().0
        }

        pub fn transaction_with_keys(&mut self) -> (Transaction, Vec<SecretKey>) {
            let variant = self.tx_sampler.sample(&mut self.rng);

            let slots = self.rng.gen_range(0..10);
            let mut builder = match variant {
                0 => TransactionBuilder::script(
                    generate_bytes(&mut self.rng),
                    generate_bytes(&mut self.rng),
                ),
                1 => TransactionBuilder::create(
                    self.rng.gen(),
                    self.rng.gen(),
                    (0..slots).map(|_| self.rng.gen()).collect(),
                ),
                _ => unreachable!(),
            };

            let inputs = self.rng.gen_range(0..10);
            let mut input_keys = Vec::with_capacity(10);

            for _ in 0..inputs {
                let variant = self.input_sampler.sample(&mut self.rng);

                match variant {
                    0 => {
                        let secret = SecretKey::random(&mut self.rng);

                        input_keys.push(secret);
                    }

                    1 => {
                        let input = Input::coin_predicate(
                            self.rng.gen(),
                            self.rng.gen(),
                            self.rng.gen(),
                            self.rng.gen(),
                            self.rng.gen(),
                            generate_nonempty_bytes(&mut self.rng),
                            generate_bytes(&mut self.rng),
                        );

                        builder.add_input(input);
                    }

                    2 => {
                        let input = Input::contract(
                            self.rng.gen(),
                            self.rng.gen(),
                            self.rng.gen(),
                            self.rng.gen(),
                        );

                        builder.add_input(input);
                    }

                    _ => unreachable!(),
                }
            }

            for i in 0..input_keys.len() {
                builder.add_unsigned_coin_input(
                    self.rng.gen(),
                    &input_keys[i],
                    self.rng.gen(),
                    self.rng.gen(),
                    self.rng.gen(),
                );
            }

            let outputs = self.rng.gen_range(0..10);
            for _ in 0..outputs {
                let variant = self.output_sampler.sample(&mut self.rng);

                let output = match variant {
                    0 => Output::coin(self.rng.gen(), self.rng.gen(), self.rng.gen()),
                    1 => Output::contract(self.rng.gen(), self.rng.gen(), self.rng.gen()),
                    2 => Output::withdrawal(self.rng.gen(), self.rng.gen(), self.rng.gen()),
                    3 => Output::change(self.rng.gen(), self.rng.gen(), self.rng.gen()),
                    4 => Output::variable(self.rng.gen(), self.rng.gen(), self.rng.gen()),
                    5 => Output::contract_created(self.rng.gen(), self.rng.gen()),

                    _ => unreachable!(),
                };

                builder.add_output(output);
            }

            let witnesses = self.rng.gen_range(0..10);
            for _ in 0..witnesses {
                let witness = generate_bytes(&mut self.rng).into();

                builder.add_witness(witness);
            }

            let tx = builder.finalize();

            (tx, input_keys)
        }
    }

    impl<R> Iterator for TransactionFactory<R>
    where
        R: Rng,
    {
        type Item = (Transaction, Vec<SecretKey>);

        fn next(&mut self) -> Option<(Transaction, Vec<SecretKey>)> {
            Some(self.transaction_with_keys())
        }
    }
}
