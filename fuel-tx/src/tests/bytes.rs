use fuel_asm::{
    op,
    PanicInstruction,
    PanicReason,
};
use fuel_tx::*;
use fuel_tx_test_helpers::{
    generate_bytes,
    generate_nonempty_padded_bytes,
};
use fuel_types::{
    bytes,
    canonical::{
        Deserialize,
        Serialize,
    },
    Immediate24,
};
use rand::{
    rngs::StdRng,
    Rng,
    RngCore,
    SeedableRng,
};

use crate::TxParameters;
use fuel_tx::field::{
    Inputs,
    Script,
    ScriptData,
};
use std::fmt;
use strum::IntoEnumIterator;

pub fn assert_encoding_correct<T>(data: &[T])
where
    T: Serialize
        + Deserialize
        + fmt::Debug
        + Clone
        + PartialEq
        + serde::Serialize
        + for<'a> serde::Deserialize<'a>,
{
    for data in data.iter() {
        let d_s = bincode::serialize(&data).expect("Failed to serialize data");
        // Safety: bincode/serde fails to understand the elision so this is a cheap way to
        // convince it
        let d_s: T =
            bincode::deserialize(d_s.as_slice()).expect("Failed to deserialize data");

        assert_eq!(&d_s, data);

        let mut d_bytes = Vec::new();
        data.clone()
            .encode_static(&mut d_bytes)
            .expect("Failed to encode");
        assert_eq!(data.size_static(), d_bytes.len());
        let mut d_p =
            T::decode_static(&mut &d_bytes[..]).expect("Failed to deserialize T");

        let mut d_bytes = Vec::new();
        data.clone()
            .encode_dynamic(&mut d_bytes)
            .expect("Failed to encode");
        assert_eq!(data.size_dynamic(), d_bytes.len());
        d_p.decode_dynamic(&mut d_bytes.as_slice())
            .expect("Failed to deserialize T");
        assert_eq!(*data, d_p);

        let mut d_bytes = Vec::new();
        data.clone().encode(&mut d_bytes).expect("Failed to encode");
        let d_p = T::decode(&mut &d_bytes[..]).expect("Failed to deserialize T");

        assert_eq!(*data, d_p);
        assert_eq!(data.size(), d_p.size());
    }
}

#[test]
fn witness() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let w = generate_bytes(rng).into();

    assert_encoding_correct(&[w, Witness::default()]);
}

#[test]
fn input() {
    let rng = &mut StdRng::seed_from_u64(8586);

    assert_encoding_correct(&[
        Input::coin_signed(
            rng.gen(),
            rng.gen(),
            rng.next_u64(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
        ),
        Input::coin_predicate(
            rng.gen(),
            rng.gen(),
            rng.next_u64(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            generate_nonempty_padded_bytes(rng),
            generate_bytes(rng),
        ),
        Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen()),
        Input::message_data_signed(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            generate_bytes(rng),
        ),
        Input::message_data_predicate(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            generate_bytes(rng),
            generate_nonempty_padded_bytes(rng),
            generate_bytes(rng),
        ),
        Input::message_coin_signed(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen()),
        Input::message_coin_predicate(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            generate_nonempty_padded_bytes(rng),
            generate_bytes(rng),
        ),
    ]);
}

#[test]
fn output() {
    let rng = &mut StdRng::seed_from_u64(8586);

    assert_encoding_correct(&[
        Output::coin(rng.gen(), rng.next_u64(), rng.gen()),
        Output::contract(rng.gen(), rng.gen(), rng.gen()),
        Output::change(rng.gen(), rng.next_u64(), rng.gen()),
        Output::variable(rng.gen(), rng.next_u64(), rng.gen()),
        Output::contract_created(rng.gen(), rng.gen()),
    ]);
}

#[test]
fn receipt() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let mut receipts = vec![
        Receipt::call(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::ret(rng.gen(), rng.gen(), rng.gen(), rng.gen()),
        Receipt::return_data(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            vec![rng.gen(), rng.gen()],
        ),
        Receipt::revert(rng.gen(), rng.gen(), rng.gen(), rng.gen()),
        Receipt::log(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::log_data(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            vec![rng.gen(), rng.gen()],
        ),
        Receipt::transfer(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::transfer_out(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            PanicInstruction::error(
                PanicReason::ContractNotInInputs,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        )
        .with_panic_contract_id(Some(rng.gen())),
        Receipt::script_result(ScriptExecutionResult::Success, rng.gen()),
        Receipt::script_result(ScriptExecutionResult::Panic, rng.gen()),
        Receipt::script_result(ScriptExecutionResult::Revert, rng.gen()),
        Receipt::script_result(
            ScriptExecutionResult::GenericFailure(rng.gen()),
            rng.gen(),
        ),
        Receipt::message_out(
            &rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            vec![rng.gen()],
        ),
        Receipt::mint(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen()),
        Receipt::burn(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen()),
    ];

    for panic_reason in PanicReason::iter() {
        receipts.push(Receipt::panic(
            rng.gen(),
            PanicInstruction::error(
                panic_reason,
                op::ji(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ));
    }

    assert_encoding_correct(&receipts);
}

#[test]
fn transaction() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let i = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen());
    let o = Output::coin(rng.gen(), rng.next_u64(), rng.gen());
    let w = rng.gen::<Witness>();
    let s = rng.gen::<StorageSlot>();

    assert_encoding_correct(&[
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.gen(),
            rng.gen::<Witness>().into_inner(),
            rng.gen::<Witness>().into_inner(),
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.gen(),
            vec![],
            generate_bytes(rng),
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.gen(),
            rng.gen::<Witness>().into_inner(),
            vec![],
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.gen(),
            vec![],
            vec![],
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.gen(),
            vec![],
            vec![],
            vec![],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.gen(),
            vec![],
            vec![],
            vec![],
            vec![],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.gen(),
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ),
    ]);
    assert_encoding_correct(&[
        Transaction::create(
            rng.next_u64(),
            TxParameters::DEFAULT.max_gas_per_tx,
            rng.gen(),
            rng.gen(),
            rng.gen(),
            vec![s.clone()],
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::create(
            rng.next_u64(),
            TxParameters::DEFAULT.max_gas_per_tx,
            rng.gen(),
            rng.gen(),
            rng.gen(),
            vec![s],
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::create(
            rng.next_u64(),
            TxParameters::DEFAULT.max_gas_per_tx,
            rng.gen(),
            rng.gen(),
            rng.gen(),
            vec![],
            vec![i],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::create(
            rng.next_u64(),
            TxParameters::DEFAULT.max_gas_per_tx,
            rng.gen(),
            rng.gen(),
            rng.gen(),
            vec![],
            vec![],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::create(
            rng.next_u64(),
            TxParameters::DEFAULT.max_gas_per_tx,
            rng.gen(),
            rng.gen(),
            rng.gen(),
            vec![],
            vec![],
            vec![],
            vec![w],
        ),
        Transaction::create(
            rng.next_u64(),
            TxParameters::DEFAULT.max_gas_per_tx,
            rng.gen(),
            rng.gen(),
            rng.gen(),
            vec![],
            vec![],
            vec![],
            vec![],
        ),
    ]);
    assert_encoding_correct(&[Transaction::mint(
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
    )]);
}

#[test]
fn create_input_data_offset() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let gas_price = 100;
    let gas_limit = 1000;
    let maturity = 10.into();
    let bytecode_witness_index = 0x00;
    let salt = rng.gen();

    let storage_slots: Vec<Vec<StorageSlot>> =
        vec![vec![], vec![rng.gen()], vec![rng.gen(), rng.gen()]];
    let inputs: Vec<Vec<Input>> = vec![
        vec![],
        vec![Input::contract(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
        )],
        vec![Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen()); 2],
    ];
    let outputs: Vec<Vec<Output>> = vec![
        vec![],
        vec![Output::coin(rng.gen(), rng.next_u64(), rng.gen())],
        vec![Output::contract(rng.gen(), rng.gen(), rng.gen())],
    ];
    let witnesses: Vec<Vec<Witness>> = vec![
        vec![],
        vec![generate_bytes(rng).into()],
        vec![generate_bytes(rng).into(), generate_bytes(rng).into()],
    ];

    let predicate = generate_nonempty_padded_bytes(rng);
    let predicate_data = generate_bytes(rng);
    let predicate_gas_used: u64 = rng.gen();

    let owner = (*Contract::root_from_code(&predicate)).into();

    let input_coin = Input::coin_predicate(
        rng.gen(),
        owner,
        rng.next_u64(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        predicate_gas_used,
        predicate.clone(),
        predicate_data.clone(),
    );

    let data = generate_bytes(rng);
    let input_message = Input::message_data_predicate(
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        predicate_gas_used,
        data,
        predicate.clone(),
        predicate_data,
    );

    for storage_slot in storage_slots.iter() {
        for inputs in inputs.iter() {
            for outputs in outputs.iter() {
                for witnesses in witnesses.iter() {
                    let mut inputs = inputs.clone();

                    let input_coin_idx = inputs.len();
                    inputs.push(input_coin.clone());

                    let input_message_idx = inputs.len();
                    inputs.push(input_message.clone());

                    let tx = Transaction::create(
                        gas_price,
                        gas_limit,
                        maturity,
                        bytecode_witness_index,
                        salt,
                        storage_slot.clone(),
                        inputs,
                        outputs.clone(),
                        witnesses.clone(),
                    );

                    let tx_p = tx.clone();

                    let bytes = tx.to_bytes();

                    let (offset, len) = tx
                        .inputs_predicate_offset_at(input_coin_idx)
                        .expect("Failed to fetch offset");

                    let (offset_p, _) =
                        tx_p.inputs_predicate_offset_at(input_coin_idx).expect(
                            "Failed to fetch offset from tx with precomputed metadata!",
                        );

                    assert_eq!(offset, offset_p);
                    assert_eq!(
                        predicate.as_slice(),
                        &bytes[offset..offset + len][..predicate.len()]
                    );

                    let (offset, len) = tx
                        .inputs_predicate_offset_at(input_message_idx)
                        .expect("Failed to fetch offset");

                    let (offset_p, _) =
                        tx_p.inputs_predicate_offset_at(input_message_idx).expect(
                            "Failed to fetch offset from tx with precomputed metadata!",
                        );

                    assert_eq!(offset, offset_p);
                    assert_eq!(
                        predicate.as_slice(),
                        &bytes[offset..offset + len][..predicate.len()]
                    );
                }
            }
        }
    }
}

#[test]
fn script_input_coin_data_offset() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let gas_price = 100;
    let gas_limit = 1000;
    let maturity = 10.into();

    let script: Vec<Vec<u8>> = vec![vec![], generate_bytes(rng)];
    let script_data: Vec<Vec<u8>> = vec![vec![], generate_bytes(rng)];

    let inputs: Vec<Vec<Input>> = vec![
        vec![],
        vec![Input::contract(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
        )],
        vec![
            Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen()),
            Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen()),
        ],
    ];
    let outputs: Vec<Vec<Output>> = vec![
        vec![],
        vec![Output::coin(rng.gen(), rng.next_u64(), rng.gen())],
        vec![Output::contract(rng.gen(), rng.gen(), rng.gen())],
    ];
    let witnesses: Vec<Vec<Witness>> = vec![
        vec![],
        vec![generate_bytes(rng).into()],
        vec![generate_bytes(rng).into(), generate_bytes(rng).into()],
    ];

    let mut predicate = generate_nonempty_padded_bytes(rng);

    // force word-unaligned predicate
    if predicate.len() % 2 == 0 {
        predicate.push(0xff);
    }

    let predicate_data = generate_bytes(rng);
    let predicate_gas_used = rng.gen();

    let owner = (*Contract::root_from_code(&predicate)).into();

    let input_coin = Input::coin_predicate(
        rng.gen(),
        owner,
        rng.next_u64(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        predicate_gas_used,
        predicate.clone(),
        predicate_data,
    );

    for script in script.iter() {
        for script_data in script_data.iter() {
            for inputs in inputs.iter() {
                for outputs in outputs.iter() {
                    for witnesses in witnesses.iter() {
                        let mut inputs = inputs.clone();
                        let offset = inputs.len();
                        inputs.push(input_coin.clone());

                        let tx = Transaction::script(
                            gas_price,
                            gas_limit,
                            maturity,
                            script.clone(),
                            script_data.clone(),
                            inputs,
                            outputs.clone(),
                            witnesses.clone(),
                        );

                        let mut tx_p = tx.clone();
                        tx_p.precompute(&Default::default())
                            .expect("Should be able to calculate cache");

                        let bytes = tx.to_bytes();

                        let script_offset = tx.script_offset();
                        assert_eq!(
                            script.as_slice(),
                            &bytes[script_offset..script_offset + script.len()]
                        );

                        let script_data_offset = tx.script_data_offset();

                        let script_data_offset_p = tx_p.script_data_offset();

                        assert_eq!(script_data_offset, script_data_offset_p);
                        assert_eq!(
                            script_data.as_slice(),
                            &bytes[script_data_offset
                                ..script_data_offset + script_data.len()]
                        );

                        let (offset, len) = tx
                            .inputs_predicate_offset_at(offset)
                            .expect("Failed to fetch offset");

                        assert_ne!(bytes::padded_len(&predicate), predicate.len());
                        assert_eq!(bytes::padded_len(&predicate), len);

                        assert_eq!(
                            predicate.as_slice(),
                            &bytes[offset..offset + predicate.len()]
                        );
                    }
                }
            }
        }
    }
}
