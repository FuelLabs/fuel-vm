#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use fuel_types::bytes;
use rand::Rng;

#[cfg(feature = "std")]
pub use use_std::*;

use alloc::vec::Vec;

pub fn generate_nonempty_padded_bytes<R>(rng: &mut R) -> Vec<u8>
where
    R: Rng,
{
    let len = rng.gen_range(1..512);
    let len = bytes::padded_len_usize(len);

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
    use fuel_tx::{
        Buildable, Contract, Create, Input, Output, Script, Transaction, TransactionBuilder,
    };
    use fuel_types::bytes::Deserializable;
    use rand::distributions::{Distribution, Uniform};
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    use std::marker::PhantomData;

    use crate::{generate_bytes, generate_nonempty_padded_bytes};

    pub struct TransactionFactory<R, Tx>
    where
        R: Rng,
        Tx: Buildable,
    {
        rng: R,
        input_sampler: Uniform<usize>,
        output_sampler: Uniform<usize>,
        marker: PhantomData<Tx>,
    }

    impl<R, Tx> From<R> for TransactionFactory<R, Tx>
    where
        R: Rng,
        Tx: Buildable,
    {
        fn from(rng: R) -> Self {
            let input_sampler = Uniform::from(0..5);
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
                        Input::MessageSigned { .. } => (),
                        Input::MessagePredicate { .. } => (),
                    })
                    .unwrap_or(());

                Output::from_bytes(&[])
                    .map(|o| match o {
                        Output::Coin { .. } => (),
                        Output::Contract { .. } => (),
                        Output::Message { .. } => (),
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
                input_sampler,
                output_sampler,
                marker: Default::default(),
            }
        }
    }

    impl<Tx: Buildable> TransactionFactory<StdRng, Tx> {
        pub fn from_seed(seed: u64) -> Self {
            StdRng::seed_from_u64(seed).into()
        }
    }

    impl<R, Tx> TransactionFactory<R, Tx>
    where
        R: Rng,
        Tx: Buildable,
    {
        fn fill_transaction(
            &mut self,
            mut builder: TransactionBuilder<Tx>,
        ) -> (Tx, Vec<SecretKey>) {
            let inputs = self.rng.gen_range(0..10);
            let mut input_coin_keys = Vec::with_capacity(10);
            let mut input_message_keys = Vec::with_capacity(10);

            for _ in 0..inputs {
                let variant = self.input_sampler.sample(&mut self.rng);

                match variant {
                    0 => {
                        let secret = SecretKey::random(&mut self.rng);

                        input_coin_keys.push(secret);
                    }

                    1 => {
                        let predicate = generate_nonempty_padded_bytes(&mut self.rng);
                        let owner = (*Contract::root_from_code(&predicate)).into();

                        let input = Input::coin_predicate(
                            self.rng.gen(),
                            owner,
                            self.rng.gen(),
                            self.rng.gen(),
                            self.rng.gen(),
                            self.rng.gen(),
                            predicate,
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
                            self.rng.gen(),
                        );

                        builder.add_input(input);
                    }

                    3 => {
                        let secret = SecretKey::random(&mut self.rng);

                        input_message_keys.push(secret);
                    }

                    4 => {
                        let predicate = generate_nonempty_padded_bytes(&mut self.rng);
                        let recipient = (*Contract::root_from_code(&predicate)).into();

                        let input = Input::message_predicate(
                            self.rng.gen(),
                            self.rng.gen(),
                            recipient,
                            self.rng.gen(),
                            self.rng.gen(),
                            generate_bytes(&mut self.rng),
                            predicate,
                            generate_bytes(&mut self.rng),
                        );

                        builder.add_input(input);
                    }

                    _ => unreachable!(),
                }
            }

            input_coin_keys.iter().for_each(|k| {
                builder.add_unsigned_coin_input(
                    *k,
                    self.rng.gen(),
                    self.rng.gen(),
                    self.rng.gen(),
                    self.rng.gen(),
                    self.rng.gen(),
                );
            });

            input_message_keys.iter().for_each(|k| {
                builder.add_unsigned_message_input(
                    *k,
                    self.rng.gen(),
                    self.rng.gen(),
                    self.rng.gen(),
                    generate_bytes(&mut self.rng),
                );
            });

            let outputs = self.rng.gen_range(0..10);
            for _ in 0..outputs {
                let variant = self.output_sampler.sample(&mut self.rng);

                let output = match variant {
                    0 => Output::coin(self.rng.gen(), self.rng.gen(), self.rng.gen()),
                    1 => Output::contract(self.rng.gen(), self.rng.gen(), self.rng.gen()),
                    2 => Output::message(self.rng.gen(), self.rng.gen()),
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
            let mut input_keys = input_coin_keys;

            input_keys.append(&mut input_message_keys);
            (tx, input_keys)
        }
    }

    impl<R> TransactionFactory<R, Create>
    where
        R: Rng,
    {
        pub fn transaction(&mut self) -> Create {
            self.transaction_with_keys().0
        }

        pub fn transaction_with_keys(&mut self) -> (Create, Vec<SecretKey>) {
            let slots = self.rng.gen_range(0..10);
            let builder = TransactionBuilder::<Create>::create(
                self.rng.gen(),
                self.rng.gen(),
                (0..slots).map(|_| self.rng.gen()).collect(),
            );

            self.fill_transaction(builder)
        }
    }

    impl<R> TransactionFactory<R, Script>
    where
        R: Rng,
    {
        pub fn transaction(&mut self) -> Script {
            self.transaction_with_keys().0
        }

        pub fn transaction_with_keys(&mut self) -> (Script, Vec<SecretKey>) {
            let builder = TransactionBuilder::<Script>::script(
                generate_bytes(&mut self.rng),
                generate_bytes(&mut self.rng),
            );

            self.fill_transaction(builder)
        }
    }

    impl<R> Iterator for TransactionFactory<R, Create>
    where
        R: Rng,
    {
        type Item = (Create, Vec<SecretKey>);

        fn next(&mut self) -> Option<(Create, Vec<SecretKey>)> {
            Some(self.transaction_with_keys())
        }
    }

    impl<R> Iterator for TransactionFactory<R, Script>
    where
        R: Rng,
    {
        type Item = (Script, Vec<SecretKey>);

        fn next(&mut self) -> Option<(Script, Vec<SecretKey>)> {
            Some(self.transaction_with_keys())
        }
    }
}
