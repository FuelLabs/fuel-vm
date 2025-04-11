#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use fuel_types::bytes;
use rand::{
    CryptoRng,
    Rng,
};

#[cfg(feature = "std")]
pub use use_std::*;

use alloc::vec::Vec;

pub fn generate_nonempty_padded_bytes<R>(rng: &mut R) -> Vec<u8>
where
    R: Rng + CryptoRng,
{
    let len = rng.gen_range(1..512);
    let len = bytes::padded_len_usize(len).unwrap();

    let mut data = alloc::vec![0u8; len];
    rng.fill_bytes(data.as_mut_slice());

    data
}

pub fn generate_bytes<R>(rng: &mut R) -> Vec<u8>
where
    R: Rng + CryptoRng,
{
    let len = rng.gen_range(1..512);

    let mut data = alloc::vec![0u8; len];
    rng.fill_bytes(data.as_mut_slice());

    data
}

#[cfg(feature = "std")]
mod use_std {
    use super::{
        generate_bytes,
        generate_nonempty_padded_bytes,
    };
    use crate::{
        field,
        Blob,
        BlobBody,
        BlobIdExt,
        Buildable,
        ConsensusParameters,
        Contract,
        Create,
        Finalizable,
        Input,
        Mint,
        Output,
        Script,
        Transaction,
        TransactionBuilder,
        Upgrade,
        UpgradePurpose,
        Upload,
        UploadBody,
        UploadSubsection,
    };
    use core::marker::PhantomData;
    use fuel_crypto::{
        Hasher,
        SecretKey,
    };
    use fuel_types::{
        canonical::Deserialize,
        BlobId,
    };
    use rand::{
        distributions::{
            Distribution,
            Uniform,
        },
        rngs::StdRng,
        CryptoRng,
        Rng,
        SeedableRng,
    };
    use strum::EnumCount;

    pub struct TransactionFactory<R, Tx>
    where
        R: Rng + CryptoRng,
    {
        rng: R,
        input_sampler: Uniform<usize>,
        output_sampler: Uniform<usize>,
        marker: PhantomData<Tx>,
    }

    impl<R, Tx> From<R> for TransactionFactory<R, Tx>
    where
        R: Rng + CryptoRng,
    {
        fn from(rng: R) -> Self {
            let input_sampler = Uniform::from(0..Input::COUNT);
            let output_sampler = Uniform::from(0..Output::COUNT);

            // Trick to enforce coverage of all variants in compile-time
            //
            // When and if a new variant is added, this implementation enforces it will be
            // listed here.
            let empty: [u8; 0] = [];
            debug_assert!({
                Input::decode(&mut &empty[..])
                    .map(|i| match i {
                        Input::CoinSigned(_) => (),
                        Input::CoinPredicate(_) => (),
                        Input::Contract(_) => (),
                        Input::MessageCoinSigned(_) => (),
                        Input::MessageCoinPredicate(_) => (),
                        Input::MessageDataSigned(_) => (),
                        Input::MessageDataPredicate(_) => (),
                    })
                    .unwrap_or(());

                Output::decode(&mut &empty[..])
                    .map(|o| match o {
                        Output::Coin { .. } => (),
                        Output::Contract(_) => (),
                        Output::Change { .. } => (),
                        Output::Variable { .. } => (),
                        Output::ContractCreated { .. } => (),
                    })
                    .unwrap_or(());

                Transaction::decode(&mut &empty[..])
                    .map(|t| match t {
                        Transaction::Script(_) => (),
                        Transaction::Create(_) => (),
                        Transaction::Mint(_) => (),
                        Transaction::Upgrade(_) => (),
                        Transaction::Upload(_) => (),
                        Transaction::Blob(_) => (),
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

    impl<Tx> TransactionFactory<StdRng, Tx> {
        pub fn from_seed(seed: u64) -> Self {
            StdRng::seed_from_u64(seed).into()
        }
    }

    impl<R, Tx> TransactionFactory<R, Tx>
    where
        R: Rng + CryptoRng,
        Tx: field::Outputs,
    {
        fn fill_outputs(&mut self, builder: &mut TransactionBuilder<Tx>) {
            let outputs = self.rng.gen_range(0..10);
            for _ in 0..outputs {
                let variant = self.output_sampler.sample(&mut self.rng);

                let output = match variant {
                    0 => Output::coin(self.rng.r#gen(), self.rng.r#gen(), self.rng.r#gen()),
                    1 => Output::contract(self.rng.r#gen(), self.rng.r#gen(), self.rng.r#gen()),
                    2 => Output::change(self.rng.r#gen(), self.rng.r#gen(), self.rng.r#gen()),
                    3 => Output::variable(self.rng.r#gen(), self.rng.r#gen(), self.rng.r#gen()),
                    4 => Output::contract_created(self.rng.r#gen(), self.rng.r#gen()),

                    _ => unreachable!(),
                };

                builder.add_output(output);
            }
        }
    }

    impl<R, Tx> TransactionFactory<R, Tx>
    where
        R: Rng + CryptoRng,
        Tx: Buildable,
    {
        fn fill_transaction(
            &mut self,
            builder: &mut TransactionBuilder<Tx>,
        ) -> Vec<SecretKey> {
            let inputs = self.rng.gen_range(0..10);
            let mut input_coin_keys = Vec::with_capacity(10);
            let mut input_message_keys = Vec::with_capacity(10);

            enum MessageType {
                MessageCoin,
                MessageData,
            }

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
                            self.rng.r#gen(),
                            owner,
                            self.rng.r#gen(),
                            self.rng.r#gen(),
                            self.rng.r#gen(),
                            self.rng.r#gen(),
                            predicate,
                            generate_bytes(&mut self.rng),
                        );

                        builder.add_input(input);
                    }

                    2 => {
                        let input = Input::contract(
                            self.rng.r#gen(),
                            self.rng.r#gen(),
                            self.rng.r#gen(),
                            self.rng.r#gen(),
                            self.rng.r#gen(),
                        );

                        builder.add_input(input);
                    }

                    3 => {
                        let secret = SecretKey::random(&mut self.rng);

                        input_message_keys.push((MessageType::MessageCoin, secret));
                    }

                    4 => {
                        let predicate = generate_nonempty_padded_bytes(&mut self.rng);
                        let recipient = (*Contract::root_from_code(&predicate)).into();

                        let input = Input::message_coin_predicate(
                            self.rng.r#gen(),
                            recipient,
                            self.rng.r#gen(),
                            self.rng.r#gen(),
                            self.rng.r#gen(),
                            predicate,
                            generate_bytes(&mut self.rng),
                        );

                        builder.add_input(input);
                    }

                    5 => {
                        let secret = SecretKey::random(&mut self.rng);

                        input_message_keys.push((MessageType::MessageData, secret));
                    }

                    6 => {
                        let predicate = generate_nonempty_padded_bytes(&mut self.rng);
                        let recipient = (*Contract::root_from_code(&predicate)).into();

                        let input = Input::message_data_predicate(
                            self.rng.r#gen(),
                            recipient,
                            self.rng.r#gen(),
                            self.rng.r#gen(),
                            self.rng.r#gen(),
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
                    self.rng.r#gen(),
                    self.rng.r#gen(),
                    self.rng.r#gen(),
                    self.rng.r#gen(),
                );
            });

            input_message_keys.iter().for_each(|(t, k)| match t {
                MessageType::MessageCoin => {
                    builder.add_unsigned_message_input(
                        *k,
                        self.rng.r#gen(),
                        self.rng.r#gen(),
                        self.rng.r#gen(),
                        vec![],
                    );
                }
                MessageType::MessageData => {
                    builder.add_unsigned_message_input(
                        *k,
                        self.rng.r#gen(),
                        self.rng.r#gen(),
                        self.rng.r#gen(),
                        generate_bytes(&mut self.rng),
                    );
                }
            });

            self.fill_outputs(builder);

            let witnesses = self.rng.gen_range(0..10);
            for _ in 0..witnesses {
                let witness = generate_bytes(&mut self.rng).into();

                builder.add_witness(witness);
            }

            let mut input_keys = input_coin_keys;
            input_keys.extend(input_message_keys.into_iter().map(|(_, k)| k));
            input_keys
        }
    }

    impl<R> TransactionFactory<R, Create>
    where
        R: Rng + CryptoRng,
    {
        pub fn transaction(&mut self) -> Create {
            self.transaction_with_keys().0
        }

        pub fn transaction_with_keys(&mut self) -> (Create, Vec<SecretKey>) {
            let slots = self.rng.gen_range(0..10);
            let mut builder = TransactionBuilder::<Create>::create(
                self.rng.r#gen(),
                self.rng.r#gen(),
                (0..slots).map(|_| self.rng.r#gen()).collect(),
            );

            let keys = self.fill_transaction(&mut builder);
            (builder.finalize(), keys)
        }
    }

    impl<R> TransactionFactory<R, Script>
    where
        R: Rng + CryptoRng,
    {
        pub fn transaction(&mut self) -> Script {
            self.transaction_with_keys().0
        }

        pub fn transaction_with_keys(&mut self) -> (Script, Vec<SecretKey>) {
            let mut builder = TransactionBuilder::<Script>::script(
                generate_bytes(&mut self.rng),
                generate_bytes(&mut self.rng),
            );

            let keys = self.fill_transaction(&mut builder);
            (builder.finalize(), keys)
        }
    }

    impl<R> TransactionFactory<R, Upgrade>
    where
        R: Rng + CryptoRng,
    {
        pub fn transaction(&mut self) -> Upgrade {
            self.transaction_with_keys().0
        }

        pub fn transaction_with_keys(&mut self) -> (Upgrade, Vec<SecretKey>) {
            let variant = self.rng.gen_range(0..UpgradePurpose::COUNT);
            let consensus_params =
                postcard::to_allocvec(&ConsensusParameters::default()).unwrap();
            let checksum = Hasher::hash(consensus_params.as_slice());

            let purpose = match variant {
                0 => UpgradePurpose::StateTransition {
                    root: self.rng.r#gen(),
                },
                1 => UpgradePurpose::ConsensusParameters {
                    witness_index: 0,
                    checksum,
                },
                _ => {
                    panic!("Not supported")
                }
            };

            let mut builder = TransactionBuilder::<Upgrade>::upgrade(purpose);
            builder.add_witness(consensus_params.into());

            let keys = self.fill_transaction(&mut builder);
            (builder.finalize(), keys)
        }
    }

    impl<R> TransactionFactory<R, Upload>
    where
        R: Rng + CryptoRng,
    {
        pub fn transaction(&mut self) -> Upload {
            self.transaction_with_keys().0
        }

        pub fn transaction_with_keys(&mut self) -> (Upload, Vec<SecretKey>) {
            let len = self.rng.gen_range(1..1024 * 1024);

            let mut bytecode = alloc::vec![0u8; len];
            self.rng.fill_bytes(bytecode.as_mut_slice());

            let subsection = UploadSubsection::split_bytecode(&bytecode, len / 10)
                .expect("Should split the bytecode")[0]
                .clone();

            let mut builder = TransactionBuilder::<Upload>::upload(UploadBody {
                root: subsection.root,
                witness_index: 0,
                subsection_index: subsection.subsection_index,
                subsections_number: subsection.subsections_number,
                proof_set: subsection.proof_set,
            });
            debug_assert_eq!(builder.witnesses().len(), 0);
            builder.add_witness(subsection.subsection.into());

            let keys = self.fill_transaction(&mut builder);
            (builder.finalize(), keys)
        }
    }

    impl<R> TransactionFactory<R, Blob>
    where
        R: Rng + CryptoRng,
    {
        pub fn transaction(&mut self) -> Blob {
            self.transaction_with_keys().0
        }

        pub fn transaction_with_keys(&mut self) -> (Blob, Vec<SecretKey>) {
            let len = self.rng.gen_range(1..1024 * 1024);

            let mut bytecode = alloc::vec![0u8; len];
            self.rng.fill_bytes(bytecode.as_mut_slice());

            let mut builder = TransactionBuilder::<Blob>::blob(BlobBody {
                id: BlobId::compute(&bytecode),
                witness_index: 0,
            });
            debug_assert_eq!(builder.witnesses().len(), 0);
            builder.add_witness(bytecode.into());

            let keys = self.fill_transaction(&mut builder);
            (builder.finalize(), keys)
        }
    }

    impl<R> TransactionFactory<R, Mint>
    where
        R: Rng + CryptoRng,
    {
        pub fn transaction(&mut self) -> Mint {
            let builder = TransactionBuilder::<Mint>::mint(
                self.rng.r#gen(),
                self.rng.r#gen(),
                self.rng.r#gen(),
                self.rng.r#gen(),
                self.rng.r#gen(),
                self.rng.r#gen(),
                self.rng.r#gen(),
            );

            builder.finalize()
        }
    }

    impl<R> Iterator for TransactionFactory<R, Create>
    where
        R: Rng + CryptoRng,
    {
        type Item = (Create, Vec<SecretKey>);

        fn next(&mut self) -> Option<(Create, Vec<SecretKey>)> {
            Some(self.transaction_with_keys())
        }
    }

    impl<R> Iterator for TransactionFactory<R, Script>
    where
        R: Rng + CryptoRng,
    {
        type Item = (Script, Vec<SecretKey>);

        fn next(&mut self) -> Option<(Script, Vec<SecretKey>)> {
            Some(self.transaction_with_keys())
        }
    }

    impl<R> Iterator for TransactionFactory<R, Upgrade>
    where
        R: Rng + CryptoRng,
    {
        type Item = (Upgrade, Vec<SecretKey>);

        fn next(&mut self) -> Option<(Upgrade, Vec<SecretKey>)> {
            Some(self.transaction_with_keys())
        }
    }

    impl<R> Iterator for TransactionFactory<R, Upload>
    where
        R: Rng + CryptoRng,
    {
        type Item = (Upload, Vec<SecretKey>);

        fn next(&mut self) -> Option<(Upload, Vec<SecretKey>)> {
            Some(self.transaction_with_keys())
        }
    }

    impl<R> Iterator for TransactionFactory<R, Blob>
    where
        R: Rng + CryptoRng,
    {
        type Item = (Blob, Vec<SecretKey>);

        fn next(&mut self) -> Option<(Blob, Vec<SecretKey>)> {
            Some(self.transaction_with_keys())
        }
    }

    impl<R> Iterator for TransactionFactory<R, Mint>
    where
        R: Rng + CryptoRng,
    {
        type Item = Mint;

        fn next(&mut self) -> Option<Mint> {
            Some(self.transaction())
        }
    }
}
