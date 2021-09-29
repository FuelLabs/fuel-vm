use crate::crypto::Hasher;
use crate::{Input, Metadata, Output, Transaction, Witness};

use fuel_types::bytes::SerializableVec;
use fuel_types::Bytes32;

impl Transaction {
    pub(crate) fn inputs_mut(&mut self) -> &mut [Input] {
        match self {
            Self::Script { inputs, .. } => inputs.as_mut_slice(),
            Self::Create { inputs, .. } => inputs.as_mut_slice(),
        }
    }

    pub(crate) fn outputs_mut(&mut self) -> &mut [Output] {
        match self {
            Self::Script { outputs, .. } => outputs.as_mut_slice(),
            Self::Create { outputs, .. } => outputs.as_mut_slice(),
        }
    }

    pub(crate) fn witnesses_mut(&mut self) -> &mut [Witness] {
        match self {
            Self::Script { witnesses, .. } => witnesses.as_mut_slice(),
            Self::Create { witnesses, .. } => witnesses.as_mut_slice(),
        }
    }

    pub fn id(&self) -> Bytes32 {
        self.metadata()
            .map(Metadata::id)
            .copied()
            .unwrap_or(self._id())
    }

    pub(crate) fn _id(&self) -> Bytes32 {
        let mut tx = self.clone();
        tx.prepare_sign();

        Hasher::hash(tx.to_bytes().as_slice())
    }

    fn prepare_sign(&mut self) {
        self.set_receipts_root(Default::default());

        self.inputs_mut().iter_mut().for_each(|input| {
            if let Input::Contract {
                utxo_id,
                balance_root,
                state_root,
                ..
            } = input
            {
                utxo_id.iter_mut().for_each(|b| *b = 0);
                balance_root.iter_mut().for_each(|b| *b = 0);
                state_root.iter_mut().for_each(|b| *b = 0);
            }
        });

        self.outputs_mut()
            .iter_mut()
            .for_each(|output| match output {
                Output::Contract {
                    balance_root,
                    state_root,
                    ..
                } => {
                    balance_root.iter_mut().for_each(|b| *b = 0);
                    state_root.iter_mut().for_each(|b| *b = 0);
                }

                Output::Change { amount, .. } => *amount = 0,

                Output::Variable {
                    to, amount, color, ..
                } => {
                    to.iter_mut().for_each(|b| *b = 0);
                    *amount = 0;
                    color.iter_mut().for_each(|b| *b = 0);
                }

                _ => (),
            });
    }
}

#[cfg(all(test, feature = "random"))]
mod tests {
    use crate::*;
    use rand::rngs::StdRng;
    use rand::{Rng, RngCore, SeedableRng};
    use std::mem;
    use std::ops::Not;

    fn invert<B>(mut bytes: B)
    where
        B: AsMut<[u8]>,
    {
        bytes.as_mut().iter_mut().for_each(|b| *b = b.not());
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

    fn assert_id_eq<F>(tx: &Transaction, mut f: F)
    where
        F: FnMut(&mut Transaction),
    {
        let mut tx_p = tx.clone();

        let mut tx_q = tx.clone();
        tx_q.precompute_metadata();

        f(&mut tx_p);

        assert_eq!(tx.id(), tx_p.id());
        assert_eq!(tx.id(), tx_q.id());
    }

    fn assert_id_ne<F>(tx: &Transaction, mut f: F)
    where
        F: FnMut(&mut Transaction),
    {
        let mut tx_p = tx.clone();

        f(&mut tx_p);

        let mut tx_q = tx_p.clone();
        tx_q.precompute_metadata();

        assert_ne!(tx.id(), tx_p.id());
        assert_ne!(tx.id(), tx_q.id());
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
    }

    fn assert_id_common_attrs(tx: &Transaction) {
        assert_id_ne(tx, |t| t.set_gas_price(t.gas_price().not()));
        assert_id_ne(tx, |t| t.set_gas_limit(t.gas_limit().not()));
        assert_id_ne(tx, |t| t.set_maturity(t.maturity().not()));

        if !tx.inputs().is_empty() {
            assert_io_ne!(tx, inputs_mut, Input::Coin, utxo_id, invert);
            assert_io_ne!(tx, inputs_mut, Input::Coin, owner, invert);
            assert_io_ne!(tx, inputs_mut, Input::Coin, amount, not);
            assert_io_ne!(tx, inputs_mut, Input::Coin, color, invert);
            assert_io_ne!(tx, inputs_mut, Input::Coin, witness_index, not);
            assert_io_ne!(tx, inputs_mut, Input::Coin, maturity, not);
            assert_io_ne!(tx, inputs_mut, Input::Coin, predicate, inv_v);
            assert_io_ne!(tx, inputs_mut, Input::Coin, predicate_data, inv_v);

            assert_io_eq!(tx, inputs_mut, Input::Contract, utxo_id, invert);
            assert_io_eq!(tx, inputs_mut, Input::Contract, balance_root, invert);
            assert_io_eq!(tx, inputs_mut, Input::Contract, state_root, invert);
            assert_io_ne!(tx, inputs_mut, Input::Contract, contract_id, invert);
        }

        if !tx.outputs().is_empty() {
            assert_io_ne!(tx, outputs_mut, Output::Coin, to, invert);
            assert_io_ne!(tx, outputs_mut, Output::Coin, amount, not);
            assert_io_ne!(tx, outputs_mut, Output::Coin, color, invert);

            assert_io_ne!(tx, outputs_mut, Output::Contract, input_index, not);
            assert_io_eq!(tx, outputs_mut, Output::Contract, balance_root, invert);
            assert_io_eq!(tx, outputs_mut, Output::Contract, state_root, invert);

            assert_io_ne!(tx, outputs_mut, Output::Withdrawal, to, invert);
            assert_io_ne!(tx, outputs_mut, Output::Withdrawal, amount, not);
            assert_io_ne!(tx, outputs_mut, Output::Withdrawal, color, invert);

            assert_io_ne!(tx, outputs_mut, Output::Change, to, invert);
            assert_io_eq!(tx, outputs_mut, Output::Change, amount, not);
            assert_io_ne!(tx, outputs_mut, Output::Change, color, invert);

            assert_io_eq!(tx, outputs_mut, Output::Variable, to, invert);
            assert_io_eq!(tx, outputs_mut, Output::Variable, amount, not);
            assert_io_eq!(tx, outputs_mut, Output::Variable, color, invert);

            assert_io_ne!(
                tx,
                outputs_mut,
                Output::ContractCreated,
                contract_id,
                invert
            );
        }

        if !tx.witnesses().is_empty() {
            assert_id_ne(tx, |t| {
                inv_v(t.witnesses_mut().first_mut().unwrap().as_vec_mut())
            });
        }
    }

    #[test]
    fn id() {
        let mut rng_base = StdRng::seed_from_u64(8586);
        let rng = &mut rng_base;

        let inputs = vec![
            vec![],
            vec![
                Input::coin(
                    rng.gen(),
                    rng.gen(),
                    rng.next_u64(),
                    rng.gen(),
                    rng.next_u32().to_be_bytes()[0],
                    rng.next_u64(),
                    rng.gen::<Witness>().into_inner(),
                    rng.gen::<Witness>().into_inner(),
                ),
                Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen()),
            ],
        ];

        let outputs = vec![
            vec![],
            vec![
                Output::coin(rng.gen(), rng.next_u64(), rng.gen()),
                Output::contract(rng.next_u32().to_be_bytes()[0], rng.gen(), rng.gen()),
                Output::withdrawal(rng.gen(), rng.next_u64(), rng.gen()),
                Output::change(rng.gen(), rng.next_u64(), rng.gen()),
                Output::variable(rng.gen(), rng.next_u64(), rng.gen()),
                Output::contract_created(rng.gen()),
            ],
        ];

        let witnesses = vec![vec![], vec![rng.gen(), rng.gen()]];

        let scripts = vec![
            vec![],
            rng.gen::<Witness>().into_inner(),
            rng.gen::<Witness>().into_inner(),
        ];
        let script_data = vec![
            vec![],
            rng.gen::<Witness>().into_inner(),
            rng.gen::<Witness>().into_inner(),
        ];
        let static_contracts = vec![vec![], vec![rng.gen(), rng.gen()]];

        for inputs in inputs.iter() {
            for outputs in outputs.iter() {
                for witnesses in witnesses.iter() {
                    for script in scripts.iter() {
                        for script_data in script_data.iter() {
                            let tx = Transaction::script(
                                rng.next_u64(),
                                rng.next_u64(),
                                rng.next_u64(),
                                script.clone(),
                                script_data.clone(),
                                inputs.clone(),
                                outputs.clone(),
                                witnesses.clone(),
                            );

                            assert_id_common_attrs(&tx);

                            assert_id_ne(&tx, |t| match t {
                                Transaction::Script { script, .. } => inv_v(script),
                                _ => unreachable!(),
                            });

                            assert_id_ne(&tx, |t| match t {
                                Transaction::Script { script_data, .. } => inv_v(script_data),
                                _ => unreachable!(),
                            });
                        }
                    }

                    for static_contracts in static_contracts.iter() {
                        let tx = Transaction::create(
                            rng.next_u64(),
                            rng.next_u64(),
                            rng.next_u64(),
                            rng.next_u32().to_be_bytes()[0],
                            rng.gen(),
                            static_contracts.clone(),
                            inputs.clone(),
                            outputs.clone(),
                            witnesses.clone(),
                        );

                        assert_id_ne(&tx, |t| match t {
                            Transaction::Create {
                                bytecode_witness_index,
                                ..
                            } => not(bytecode_witness_index),
                            _ => unreachable!(),
                        });

                        assert_id_ne(&tx, |t| match t {
                            Transaction::Create { salt, .. } => invert(salt),
                            _ => unreachable!(),
                        });

                        if !static_contracts.is_empty() {
                            assert_id_ne(&tx, |t| match t {
                                Transaction::Create {
                                    static_contracts, ..
                                } => invert(static_contracts.first_mut().unwrap()),
                                _ => unreachable!(),
                            });
                        }

                        assert_id_common_attrs(&tx);
                    }
                }
            }
        }
    }
}
