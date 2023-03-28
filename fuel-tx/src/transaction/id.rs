use crate::input::coin::CoinSigned;
use crate::input::message::{MessageCoinSigned, MessageDataSigned};
use crate::{field, ConsensusParameters, Input, Transaction};
use fuel_crypto::{Message, PublicKey, SecretKey, Signature};
use fuel_types::Bytes32;

/// Means that transaction has a unique identifier.
pub trait UniqueIdentifier {
    /// The unique identifier of the transaction is based on its content.
    fn id(&self, params: &ConsensusParameters) -> Bytes32;
}

impl UniqueIdentifier for Transaction {
    fn id(&self, params: &ConsensusParameters) -> Bytes32 {
        match self {
            Transaction::Script(script) => script.id(params),
            Transaction::Create(create) => create.id(params),
            Self::Mint(mint) => mint.id(params),
        }
    }
}

/// Means that transaction can be singed.
///
/// # Note: Autogenerated transactions are not signable.
pub trait Signable: UniqueIdentifier {
    /// Signs inputs of the transaction.
    fn sign_inputs(&mut self, secret: &SecretKey, parameters: &ConsensusParameters);
}

impl<T> Signable for T
where
    T: UniqueIdentifier + field::Witnesses + field::Inputs,
{
    /// For all inputs of type `coin` or `message`, check if its `owner` equals the public
    /// counterpart of the provided key. Sign all matches.
    fn sign_inputs(&mut self, secret: &SecretKey, parameters: &ConsensusParameters) {
        use itertools::Itertools;

        let pk = PublicKey::from(secret);
        let pk = Input::owner(&pk);
        let id = self.id(parameters);

        let message = Message::from_bytes_ref(&id);

        let signature = Signature::sign(secret, message);

        let inputs = self.inputs();

        let witness_indexes = inputs
            .iter()
            .filter_map(|input| match input {
                Input::CoinSigned(CoinSigned {
                    owner, witness_index, ..
                })
                | Input::MessageCoinSigned(MessageCoinSigned {
                    recipient: owner,
                    witness_index,
                    ..
                })
                | Input::MessageDataSigned(MessageDataSigned {
                    recipient: owner,
                    witness_index,
                    ..
                }) if owner == &pk => Some(*witness_index as usize),
                _ => None,
            })
            .dedup()
            .collect_vec();

        for w in witness_indexes {
            if let Some(w) = self.witnesses_mut().get_mut(w) {
                *w = signature.as_ref().into();
            }
        }
    }
}

#[cfg(all(test, feature = "random"))]
mod tests {
    use fuel_tx::{
        field::*,
        input::{
            coin::{CoinPredicate, CoinSigned},
            contract::Contract,
            message::{MessageCoinPredicate, MessageCoinSigned, MessageDataPredicate, MessageDataSigned},
        },
        Buildable, ConsensusParameters, Input, Output, StorageSlot, Transaction, UtxoId,
    };
    use fuel_tx_test_helpers::{generate_bytes, generate_nonempty_padded_bytes};
    use rand::rngs::StdRng;
    use rand::{Rng, RngCore, SeedableRng};
    use std::io::{Read, Write};
    use std::mem;
    use std::ops::Not;

    fn invert<B>(mut bytes: B)
    where
        B: AsMut<[u8]>,
    {
        bytes.as_mut().iter_mut().for_each(|b| *b = b.not());
    }

    fn invert_utxo_id(utxo_id: &mut UtxoId) {
        let mut tx_id = *utxo_id.tx_id();
        let mut out_idx = [utxo_id.output_index()];

        invert(&mut tx_id);
        invert(&mut out_idx);

        *utxo_id = UtxoId::new(tx_id, out_idx[0])
    }

    fn invert_storage_slot(storage_slot: &mut StorageSlot) {
        let mut data = [0u8; 64];
        let _ = storage_slot.read(&mut data).unwrap();
        invert(&mut data);
        let _ = storage_slot.write(&data).unwrap();
    }

    fn inv_v(bytes: &mut Vec<u8>) {
        if bytes.is_empty() {
            bytes.push(0xfb);
        } else {
            invert(bytes.as_mut_slice());
        }
    }

    fn not<T>(t: &mut T)
    where
        T: Copy + Not<Output = T>,
    {
        let mut t_p = t.not();
        mem::swap(t, &mut t_p);
    }

    fn not_u32<T>(t: &mut T)
    where
        T: Copy + Into<u32> + From<u32>,
    {
        let u_32: u32 = (*t).into();
        let mut t_p = u_32.not().into();
        mem::swap(t, &mut t_p);
    }

    fn assert_id_eq<Tx: Buildable, F>(tx: &Tx, mut f: F)
    where
        F: FnMut(&mut Tx),
    {
        let mut tx_p = tx.clone();

        let mut tx_q = tx.clone();
        tx_q.precompute(&ConsensusParameters::DEFAULT);

        f(&mut tx_p);

        assert_eq!(
            tx.id(&ConsensusParameters::DEFAULT),
            tx_p.id(&ConsensusParameters::DEFAULT)
        );
        assert_eq!(
            tx.id(&ConsensusParameters::DEFAULT),
            tx_q.id(&ConsensusParameters::DEFAULT)
        );
    }

    fn assert_id_ne<Tx: Buildable, F>(tx: &Tx, mut f: F)
    where
        F: FnMut(&mut Tx),
    {
        let mut tx_p = tx.clone();

        f(&mut tx_p);

        let mut tx_q = tx_p.clone();
        tx_q.precompute(&ConsensusParameters::DEFAULT);

        assert_ne!(
            tx.id(&ConsensusParameters::DEFAULT),
            tx_p.id(&ConsensusParameters::DEFAULT)
        );
        assert_ne!(
            tx.id(&ConsensusParameters::DEFAULT),
            tx_q.id(&ConsensusParameters::DEFAULT)
        );
    }

    macro_rules! assert_io_ne {
        ($tx:expr, $t:ident, $i:path, $a:ident, $inv:expr) => {
            assert_id_ne($tx, |t| {
                t.$t().iter_mut().for_each(|x| match x {
                    $i { $a, .. } => $inv($a),
                    _ => (),
                })
            });
        };
        ($tx:expr, $t:ident, $i:path [ $it:path ], $a:ident, $inv:expr) => {
            assert_id_ne($tx, |t| {
                t.$t().iter_mut().for_each(|x| match x {
                    $i($it { $a, .. }) => $inv($a),
                    _ => (),
                })
            });
        };
    }

    macro_rules! assert_io_eq {
        ($tx:expr, $t:ident, $i:path, $a:ident, $inv:expr) => {
            assert_id_eq($tx, |t| {
                t.$t().iter_mut().for_each(|x| match x {
                    $i { $a, .. } => $inv($a),
                    _ => (),
                })
            });
        };
        ($tx:expr, $t:ident, $i:path [ $it:path ], $a:ident, $inv:expr) => {
            assert_id_eq($tx, |t| {
                t.$t().iter_mut().for_each(|x| match x {
                    $i($it { $a, .. }) => $inv($a),
                    _ => (),
                })
            });
        };
    }

    fn assert_id_common_attrs<Tx: Buildable>(tx: &Tx) {
        use core::ops::Deref;
        assert_id_ne(tx, |t| t.set_gas_price(t.gas_price().not()));
        assert_id_ne(tx, |t| t.set_gas_limit(t.gas_limit().not()));
        assert_id_ne(tx, |t| t.set_maturity((t.maturity().deref().not()).into()));

        if !tx.inputs().is_empty() {
            assert_io_ne!(tx, inputs_mut, Input::CoinSigned[CoinSigned], utxo_id, invert_utxo_id);
            assert_io_ne!(tx, inputs_mut, Input::CoinSigned[CoinSigned], owner, invert);
            assert_io_ne!(tx, inputs_mut, Input::CoinSigned[CoinSigned], amount, not);
            assert_io_ne!(tx, inputs_mut, Input::CoinSigned[CoinSigned], asset_id, invert);
            assert_io_ne!(tx, inputs_mut, Input::CoinSigned[CoinSigned], witness_index, not);
            assert_io_ne!(tx, inputs_mut, Input::CoinSigned[CoinSigned], maturity, not_u32);

            assert_io_ne!(
                tx,
                inputs_mut,
                Input::CoinPredicate[CoinPredicate],
                utxo_id,
                invert_utxo_id
            );
            assert_io_ne!(tx, inputs_mut, Input::CoinPredicate[CoinPredicate], owner, invert);
            assert_io_ne!(tx, inputs_mut, Input::CoinPredicate[CoinPredicate], amount, not);
            assert_io_ne!(tx, inputs_mut, Input::CoinPredicate[CoinPredicate], asset_id, invert);
            assert_io_ne!(tx, inputs_mut, Input::CoinPredicate[CoinPredicate], maturity, not_u32);
            assert_io_ne!(tx, inputs_mut, Input::CoinPredicate[CoinPredicate], predicate, inv_v);
            assert_io_ne!(
                tx,
                inputs_mut,
                Input::CoinPredicate[CoinPredicate],
                predicate_data,
                inv_v
            );

            assert_io_eq!(tx, inputs_mut, Input::Contract[Contract], utxo_id, invert_utxo_id);
            assert_io_eq!(tx, inputs_mut, Input::Contract[Contract], balance_root, invert);
            assert_io_eq!(tx, inputs_mut, Input::Contract[Contract], state_root, invert);
            assert_io_ne!(tx, inputs_mut, Input::Contract[Contract], contract_id, invert);

            assert_io_ne!(
                tx,
                inputs_mut,
                Input::MessageCoinSigned[MessageCoinSigned],
                sender,
                invert
            );
            assert_io_ne!(
                tx,
                inputs_mut,
                Input::MessageCoinSigned[MessageCoinSigned],
                recipient,
                invert
            );
            assert_io_ne!(tx, inputs_mut, Input::MessageCoinSigned[MessageCoinSigned], amount, not);
            assert_io_ne!(
                tx,
                inputs_mut,
                Input::MessageCoinSigned[MessageCoinSigned],
                nonce,
                invert
            );
            assert_io_ne!(
                tx,
                inputs_mut,
                Input::MessageCoinSigned[MessageCoinSigned],
                witness_index,
                not
            );

            assert_io_ne!(
                tx,
                inputs_mut,
                Input::MessageDataSigned[MessageDataSigned],
                sender,
                invert
            );
            assert_io_ne!(
                tx,
                inputs_mut,
                Input::MessageDataSigned[MessageDataSigned],
                recipient,
                invert
            );
            assert_io_ne!(tx, inputs_mut, Input::MessageDataSigned[MessageDataSigned], amount, not);
            assert_io_ne!(
                tx,
                inputs_mut,
                Input::MessageDataSigned[MessageDataSigned],
                nonce,
                invert
            );
            assert_io_ne!(
                tx,
                inputs_mut,
                Input::MessageDataSigned[MessageDataSigned],
                witness_index,
                not
            );
            assert_io_ne!(tx, inputs_mut, Input::MessageDataSigned[MessageDataSigned], data, inv_v);

            assert_io_ne!(
                tx,
                inputs_mut,
                Input::MessageDataPredicate[MessageDataPredicate],
                sender,
                invert
            );
            assert_io_ne!(
                tx,
                inputs_mut,
                Input::MessageCoinPredicate[MessageCoinPredicate],
                recipient,
                invert
            );
            assert_io_ne!(
                tx,
                inputs_mut,
                Input::MessageCoinPredicate[MessageCoinPredicate],
                amount,
                not
            );
            assert_io_ne!(
                tx,
                inputs_mut,
                Input::MessageCoinPredicate[MessageCoinPredicate],
                nonce,
                invert
            );
            assert_io_ne!(
                tx,
                inputs_mut,
                Input::MessageCoinPredicate[MessageCoinPredicate],
                predicate,
                inv_v
            );
            assert_io_ne!(
                tx,
                inputs_mut,
                Input::MessageCoinPredicate[MessageCoinPredicate],
                predicate_data,
                inv_v
            );

            assert_io_ne!(
                tx,
                inputs_mut,
                Input::MessageDataPredicate[MessageDataPredicate],
                sender,
                invert
            );
            assert_io_ne!(
                tx,
                inputs_mut,
                Input::MessageDataPredicate[MessageDataPredicate],
                recipient,
                invert
            );
            assert_io_ne!(
                tx,
                inputs_mut,
                Input::MessageDataPredicate[MessageDataPredicate],
                amount,
                not
            );
            assert_io_ne!(
                tx,
                inputs_mut,
                Input::MessageDataPredicate[MessageDataPredicate],
                data,
                inv_v
            );
            assert_io_ne!(
                tx,
                inputs_mut,
                Input::MessageDataPredicate[MessageDataPredicate],
                nonce,
                invert
            );
            assert_io_ne!(
                tx,
                inputs_mut,
                Input::MessageDataPredicate[MessageDataPredicate],
                data,
                inv_v
            );
            assert_io_ne!(
                tx,
                inputs_mut,
                Input::MessageDataPredicate[MessageDataPredicate],
                predicate,
                inv_v
            );
            assert_io_ne!(
                tx,
                inputs_mut,
                Input::MessageDataPredicate[MessageDataPredicate],
                predicate_data,
                inv_v
            );
        }

        if !tx.outputs().is_empty() {
            assert_io_ne!(tx, outputs_mut, Output::Coin, to, invert);
            assert_io_ne!(tx, outputs_mut, Output::Coin, amount, not);
            assert_io_ne!(tx, outputs_mut, Output::Coin, asset_id, invert);

            assert_io_ne!(tx, outputs_mut, Output::Contract, input_index, not);
            assert_io_eq!(tx, outputs_mut, Output::Contract, balance_root, invert);
            assert_io_eq!(tx, outputs_mut, Output::Contract, state_root, invert);

            assert_io_ne!(tx, outputs_mut, Output::Change, to, invert);
            assert_io_eq!(tx, outputs_mut, Output::Change, amount, not);
            assert_io_ne!(tx, outputs_mut, Output::Change, asset_id, invert);

            assert_io_eq!(tx, outputs_mut, Output::Variable, to, invert);
            assert_io_eq!(tx, outputs_mut, Output::Variable, amount, not);
            assert_io_eq!(tx, outputs_mut, Output::Variable, asset_id, invert);

            assert_io_ne!(tx, outputs_mut, Output::ContractCreated, contract_id, invert);
        }

        if !tx.witnesses().is_empty() {
            assert_id_eq(tx, |t| inv_v(t.witnesses_mut().first_mut().unwrap().as_vec_mut()));
        }
    }

    #[test]
    fn id() {
        let rng = &mut StdRng::seed_from_u64(8586);

        let inputs = vec![
            vec![],
            vec![
                Input::coin_signed(
                    rng.gen(),
                    rng.gen(),
                    rng.next_u64(),
                    rng.gen(),
                    rng.gen(),
                    rng.next_u32().to_be_bytes()[0],
                    rng.gen(),
                ),
                Input::coin_predicate(
                    rng.gen(),
                    rng.gen(),
                    rng.next_u64(),
                    rng.gen(),
                    rng.gen(),
                    rng.gen(),
                    generate_nonempty_padded_bytes(rng),
                    generate_bytes(rng),
                ),
                Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen()),
                Input::message_coin_signed(
                    rng.gen(),
                    rng.gen(),
                    rng.next_u64(),
                    rng.gen(),
                    rng.next_u32().to_be_bytes()[0],
                ),
                Input::message_coin_predicate(
                    rng.gen(),
                    rng.gen(),
                    rng.next_u64(),
                    rng.gen(),
                    generate_nonempty_padded_bytes(rng),
                    generate_bytes(rng),
                ),
                Input::message_data_signed(
                    rng.gen(),
                    rng.gen(),
                    rng.next_u64(),
                    rng.gen(),
                    rng.next_u32().to_be_bytes()[0],
                    generate_nonempty_padded_bytes(rng),
                ),
                Input::message_data_predicate(
                    rng.gen(),
                    rng.gen(),
                    rng.next_u64(),
                    rng.gen(),
                    generate_nonempty_padded_bytes(rng),
                    generate_nonempty_padded_bytes(rng),
                    generate_bytes(rng),
                ),
            ],
        ];

        let outputs = vec![
            vec![],
            vec![
                Output::coin(rng.gen(), rng.next_u64(), rng.gen()),
                Output::contract(rng.next_u32().to_be_bytes()[0], rng.gen(), rng.gen()),
                Output::change(rng.gen(), rng.next_u64(), rng.gen()),
                Output::variable(rng.gen(), rng.next_u64(), rng.gen()),
                Output::contract_created(rng.gen(), rng.gen()),
            ],
        ];

        let witnesses = vec![vec![], vec![generate_bytes(rng).into(), generate_bytes(rng).into()]];

        let scripts = vec![vec![], generate_bytes(rng), generate_bytes(rng)];
        let script_data = vec![vec![], generate_bytes(rng), generate_bytes(rng)];
        let storage_slots = vec![vec![], vec![rng.gen(), rng.gen()]];

        for inputs in inputs.iter() {
            for outputs in outputs.iter() {
                for witnesses in witnesses.iter() {
                    for script in scripts.iter() {
                        for script_data in script_data.iter() {
                            let tx = Transaction::script(
                                rng.next_u64(),
                                rng.next_u64(),
                                rng.gen(),
                                script.clone(),
                                script_data.clone(),
                                inputs.clone(),
                                outputs.clone(),
                                witnesses.clone(),
                            );

                            assert_id_common_attrs(&tx);
                            assert_id_ne(&tx, |t| inv_v(t.script_mut()));
                            assert_id_ne(&tx, |t| inv_v(t.script_data_mut()));
                        }
                    }

                    for storage_slots in storage_slots.iter() {
                        let tx = Transaction::create(
                            rng.next_u64(),
                            ConsensusParameters::DEFAULT.max_gas_per_tx,
                            rng.gen(),
                            rng.next_u32().to_be_bytes()[0],
                            rng.gen(),
                            storage_slots.clone(),
                            inputs.clone(),
                            outputs.clone(),
                            witnesses.clone(),
                        );

                        assert_id_ne(&tx, |t| not(t.bytecode_witness_index_mut()));
                        assert_id_ne(&tx, |t| invert(t.salt_mut()));
                        assert_id_ne(&tx, |t| invert(t.salt_mut()));

                        if !storage_slots.is_empty() {
                            assert_id_ne(&tx, |t| invert_storage_slot(t.storage_slots_mut().first_mut().unwrap()));
                        }

                        assert_id_common_attrs(&tx);
                    }
                }
            }
        }
    }
}
