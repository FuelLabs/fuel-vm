use fuel_tx::*;
use std::mem;
use std::ops::Not;

fn d<T: Default>() -> T {
    Default::default()
}

fn invert(bytes: &mut [u8]) {
    bytes.iter_mut().for_each(|b| *b = b.not());
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
    f(&mut tx_p);
    assert_eq!(tx.id(), tx_p.id());
}

fn assert_id_ne<F>(tx: &Transaction, mut f: F)
where
    F: FnMut(&mut Transaction),
{
    let mut tx_p = tx.clone();
    f(&mut tx_p);
    assert_ne!(tx.id(), tx_p.id());
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

        assert_io_ne!(tx, outputs_mut, Output::ContractCreated, contract_id, invert);
    }

    if !tx.witnesses().is_empty() {
        assert_id_ne(tx, |t| inv_v(t.witnesses_mut().first_mut().unwrap().as_vec_mut()));
    }
}

#[test]
fn id() {
    let inputs = vec![
        vec![],
        vec![
            Input::coin(d(), d(), d(), d(), d(), d(), d(), d()),
            Input::contract(d(), d(), d(), d()),
        ],
    ];

    let outputs = vec![
        vec![],
        vec![
            Output::coin(d(), d(), d()),
            Output::contract(d(), d(), d()),
            Output::withdrawal(d(), d(), d()),
            Output::change(d(), d(), d()),
            Output::variable(d(), d(), d()),
            Output::contract_created(d()),
        ],
    ];

    let witnesses = vec![vec![], vec![Witness::from(vec![0xab]), Witness::from(vec![0xbc, 0xbd])]];

    let scripts = vec![vec![], vec![0xfa], vec![0xfa, 0xfb]];
    let script_data = vec![vec![], vec![0xfa], vec![0xfa, 0xfb]];
    let static_contracts = vec![vec![], vec![d(), d()]];

    for inputs in inputs.iter() {
        for outputs in outputs.iter() {
            for witnesses in witnesses.iter() {
                for script in scripts.iter() {
                    for script_data in script_data.iter() {
                        let tx = Transaction::script(
                            d(),
                            d(),
                            d(),
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
                        d(),
                        d(),
                        d(),
                        d(),
                        d(),
                        static_contracts.clone(),
                        inputs.clone(),
                        outputs.clone(),
                        witnesses.clone(),
                    );

                    assert_id_ne(&tx, |t| match t {
                        Transaction::Create {
                            bytecode_witness_index, ..
                        } => not(bytecode_witness_index),
                        _ => unreachable!(),
                    });

                    assert_id_ne(&tx, |t| match t {
                        Transaction::Create { salt, .. } => invert(salt),
                        _ => unreachable!(),
                    });

                    if !static_contracts.is_empty() {
                        assert_id_ne(&tx, |t| match t {
                            Transaction::Create { static_contracts, .. } => {
                                invert(static_contracts.first_mut().unwrap())
                            }
                            _ => unreachable!(),
                        });
                    }

                    assert_id_common_attrs(&tx);
                }
            }
        }
    }
}
