use crate::{
    field::{
        Inputs,
        Script,
        ScriptData,
    },
    policies::Policies,
    test_helper::{
        generate_bytes,
        generate_nonempty_padded_bytes,
    },
    *,
};
use fuel_asm::{
    PanicInstruction,
    PanicReason,
    op,
};
use fuel_types::{
    Immediate24,
    bytes,
    canonical::{
        Deserialize,
        Serialize,
    },
};
use rand::{
    Rng,
    RngCore,
    SeedableRng,
    rngs::StdRng,
};
use std::fmt;

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
            rng.r#gen(),
            rng.r#gen(),
            rng.next_u64(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        ),
        Input::coin_predicate(
            rng.r#gen(),
            rng.r#gen(),
            rng.next_u64(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            generate_nonempty_padded_bytes(rng),
            generate_bytes(rng),
        ),
        Input::contract(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        ),
        Input::message_data_signed(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            generate_bytes(rng),
        ),
        Input::message_data_predicate(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            generate_bytes(rng),
            generate_nonempty_padded_bytes(rng),
            generate_bytes(rng),
        ),
        Input::message_coin_signed(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        ),
        Input::message_coin_predicate(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            generate_nonempty_padded_bytes(rng),
            generate_bytes(rng),
        ),
    ]);
}

#[test]
fn output() {
    let rng = &mut StdRng::seed_from_u64(8586);

    assert_encoding_correct(&[
        Output::coin(rng.r#gen(), rng.next_u64(), rng.r#gen()),
        Output::contract(rng.r#gen(), rng.r#gen(), rng.r#gen()),
        Output::change(rng.r#gen(), rng.next_u64(), rng.r#gen()),
        Output::variable(rng.r#gen(), rng.next_u64(), rng.r#gen()),
        Output::contract_created(rng.r#gen(), rng.r#gen()),
    ]);
}

#[test]
fn receipt() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let receipts = vec![
        Receipt::call(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        ),
        Receipt::ret(rng.r#gen(), rng.r#gen(), rng.r#gen(), rng.r#gen()),
        Receipt::return_data(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            vec![rng.r#gen(), rng.r#gen()],
        ),
        Receipt::revert(rng.r#gen(), rng.r#gen(), rng.r#gen(), rng.r#gen()),
        Receipt::log(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        ),
        Receipt::log_data(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            vec![rng.r#gen(), rng.r#gen()],
        ),
        Receipt::transfer(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        ),
        Receipt::transfer_out(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        ),
        Receipt::panic(
            rng.r#gen(),
            PanicInstruction::error(
                PanicReason::UnknownPanicReason,
                op::ji(rng.r#gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.r#gen(),
            rng.r#gen(),
        )
        .with_panic_contract_id(Some(rng.r#gen())),
        Receipt::script_result(ScriptExecutionResult::Success, rng.r#gen()),
        Receipt::script_result(ScriptExecutionResult::Panic, rng.r#gen()),
        Receipt::script_result(ScriptExecutionResult::Revert, rng.r#gen()),
        Receipt::script_result(
            ScriptExecutionResult::GenericFailure(rng.r#gen()),
            rng.r#gen(),
        ),
        Receipt::message_out(
            &rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            vec![rng.r#gen()],
        ),
        Receipt::mint(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        ),
        Receipt::burn(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        ),
    ];

    assert_encoding_correct(&receipts);
}

#[test]
fn transaction_serde_serialization_deserialization() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let i = Input::contract(
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
    );
    let o = Output::coin(rng.r#gen(), rng.next_u64(), rng.r#gen());
    let w = rng.r#gen::<Witness>();
    let s = rng.r#gen::<StorageSlot>();

    assert_encoding_correct(&[
        Transaction::script(
            rng.next_u64(),
            rng.r#gen::<Witness>().into_inner(),
            rng.r#gen::<Witness>().into_inner(),
            rng.r#gen(),
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            vec![],
            generate_bytes(rng),
            rng.r#gen(),
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.r#gen::<Witness>().into_inner(),
            vec![],
            rng.r#gen(),
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            vec![],
            vec![],
            rng.r#gen(),
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            vec![],
            vec![],
            rng.r#gen(),
            vec![],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            vec![],
            vec![],
            rng.r#gen(),
            vec![],
            vec![],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            vec![],
            vec![],
            rng.r#gen(),
            vec![],
            vec![],
            vec![],
        ),
    ]);
    assert_encoding_correct(&[
        Transaction::create(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            vec![s.clone()],
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::create(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            vec![s],
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::create(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            vec![],
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::create(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            vec![],
            vec![],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::create(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            vec![],
            vec![],
            vec![],
            vec![w.clone()],
        ),
        Transaction::create(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            vec![],
            vec![],
            vec![],
            vec![],
        ),
    ]);
    assert_encoding_correct(&[
        Transaction::upgrade(
            UpgradePurpose::ConsensusParameters {
                witness_index: 0,
                checksum: [0xfa; 32].into(),
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_expiration((u32::MAX >> 2).into())
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::upgrade(
            UpgradePurpose::ConsensusParameters {
                witness_index: 0,
                checksum: [0xfa; 32].into(),
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_expiration((u32::MAX >> 2).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::upgrade(
            UpgradePurpose::ConsensusParameters {
                witness_index: 0,
                checksum: [0xfa; 32].into(),
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_expiration((u32::MAX >> 2).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![],
            vec![w.clone()],
        ),
        Transaction::upgrade(
            UpgradePurpose::ConsensusParameters {
                witness_index: 0,
                checksum: [0xfa; 32].into(),
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_expiration((u32::MAX >> 2).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![],
            vec![],
        ),
        Transaction::upgrade(
            UpgradePurpose::StateTransition {
                root: [0xfa; 32].into(),
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_expiration((u32::MAX >> 2).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::upgrade(
            UpgradePurpose::StateTransition {
                root: [0xfa; 32].into(),
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_expiration((u32::MAX >> 2).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::upgrade(
            UpgradePurpose::StateTransition {
                root: [0xfa; 32].into(),
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_expiration((u32::MAX >> 2).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![],
            vec![w.clone()],
        ),
        Transaction::upgrade(
            UpgradePurpose::StateTransition {
                root: [0xfa; 32].into(),
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_expiration((u32::MAX >> 2).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![],
            vec![],
        ),
    ]);
    assert_encoding_correct(&[
        Transaction::upload(
            UploadBody {
                root: [6; 32].into(),
                witness_index: 0,
                subsection_index: 0x1234,
                subsections_number: 0x4321,
                proof_set: vec![[1; 32].into(), [2; 32].into(), [3; 32].into()],
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_expiration((u32::MAX >> 2).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::upload(
            UploadBody {
                root: [6; 32].into(),
                witness_index: 0,
                subsection_index: 0x1234,
                subsections_number: 0x4321,
                proof_set: vec![[1; 32].into(), [2; 32].into(), [3; 32].into()],
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_expiration((u32::MAX >> 2).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::upload(
            UploadBody {
                root: [6; 32].into(),
                witness_index: 0,
                subsection_index: 0x1234,
                subsections_number: 0x4321,
                proof_set: vec![[1; 32].into(), [2; 32].into(), [3; 32].into()],
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_expiration((u32::MAX >> 2).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![],
            vec![w.clone()],
        ),
        Transaction::upload(
            UploadBody {
                root: [6; 32].into(),
                witness_index: 0,
                subsection_index: 0x1234,
                subsections_number: 0x4321,
                proof_set: vec![[1; 32].into(), [2; 32].into(), [3; 32].into()],
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_expiration((u32::MAX >> 2).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![],
            vec![],
        ),
    ]);
    assert_encoding_correct(&[
        Transaction::upload(
            UploadBody {
                root: [6; 32].into(),
                witness_index: 0,
                subsection_index: 0x1234,
                subsections_number: 0x4321,
                proof_set: vec![[1; 32].into(), [2; 32].into(), [3; 32].into()],
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_expiration((u32::MAX >> 2).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::upload(
            UploadBody {
                root: [6; 32].into(),
                witness_index: 0,
                subsection_index: 0x1234,
                subsections_number: 0x4321,
                proof_set: vec![[1; 32].into(), [2; 32].into(), [3; 32].into()],
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_expiration((u32::MAX >> 2).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::upload(
            UploadBody {
                root: [6; 32].into(),
                witness_index: 0,
                subsection_index: 0x1234,
                subsections_number: 0x4321,
                proof_set: vec![[1; 32].into(), [2; 32].into(), [3; 32].into()],
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_expiration((u32::MAX >> 2).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![],
            vec![w.clone()],
        ),
        Transaction::upload(
            UploadBody {
                root: [6; 32].into(),
                witness_index: 0,
                subsection_index: 0x1234,
                subsections_number: 0x4321,
                proof_set: vec![[1; 32].into(), [2; 32].into(), [3; 32].into()],
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_expiration((u32::MAX >> 2).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![],
            vec![],
        ),
    ]);
    assert_encoding_correct(&[Transaction::mint(
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
    )]);
}

#[test]
fn create_input_data_offset() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 10.into();
    let bytecode_witness_index = 0x00;
    let salt = rng.r#gen();

    let storage_slots: Vec<Vec<StorageSlot>> =
        vec![vec![], vec![rng.r#gen()], vec![rng.r#gen(), rng.r#gen()]];
    let inputs: Vec<Vec<Input>> = vec![
        vec![],
        vec![Input::contract(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        )],
        vec![
            Input::contract(
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen()
            );
            2
        ],
    ];
    let outputs: Vec<Vec<Output>> = vec![
        vec![],
        vec![Output::coin(rng.r#gen(), rng.next_u64(), rng.r#gen())],
        vec![Output::contract(rng.r#gen(), rng.r#gen(), rng.r#gen())],
    ];
    let witnesses: Vec<Vec<Witness>> = vec![
        vec![],
        vec![generate_bytes(rng).into()],
        vec![generate_bytes(rng).into(), generate_bytes(rng).into()],
    ];

    let predicate = generate_nonempty_padded_bytes(rng);
    let predicate_data = generate_bytes(rng);
    let predicate_gas_used: u64 = rng.r#gen();

    let owner = (*Contract::root_from_code(&predicate)).into();

    let input_coin = Input::coin_predicate(
        rng.r#gen(),
        owner,
        rng.next_u64(),
        rng.r#gen(),
        rng.r#gen(),
        predicate_gas_used,
        predicate.clone(),
        predicate_data.clone(),
    );

    let data = generate_bytes(rng);
    let input_message = Input::message_data_predicate(
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
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
                        bytecode_witness_index,
                        Policies::new().with_maturity(maturity),
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

    let gas_limit = 1000;
    let maturity = 10.into();

    let script: Vec<Vec<u8>> = vec![vec![], generate_bytes(rng)];
    let script_data: Vec<Vec<u8>> = vec![vec![], generate_bytes(rng)];

    let inputs: Vec<Vec<Input>> = vec![
        vec![],
        vec![Input::contract(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        )],
        vec![
            Input::contract(
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
            ),
            Input::contract(
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
            ),
        ],
    ];
    let outputs: Vec<Vec<Output>> = vec![
        vec![],
        vec![Output::coin(rng.r#gen(), rng.next_u64(), rng.r#gen())],
        vec![Output::contract(rng.r#gen(), rng.r#gen(), rng.r#gen())],
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
    let predicate_gas_used = rng.r#gen();

    let owner = (*Contract::root_from_code(&predicate)).into();

    let input_coin = Input::coin_predicate(
        rng.r#gen(),
        owner,
        rng.next_u64(),
        rng.r#gen(),
        rng.r#gen(),
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
                            gas_limit,
                            script.clone(),
                            script_data.clone(),
                            Policies::new().with_maturity(maturity),
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

                        assert_ne!(bytes::padded_len(&predicate), Some(predicate.len()));
                        assert_eq!(bytes::padded_len(&predicate), Some(len));

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

#[test]
fn upgrade_input_coin_data_offset() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 10.into();

    let inputs: Vec<Vec<Input>> = vec![
        vec![],
        vec![Input::contract(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        )],
        vec![
            Input::contract(
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
            ),
            Input::contract(
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
            ),
        ],
    ];
    let outputs: Vec<Vec<Output>> = vec![
        vec![],
        vec![Output::coin(rng.r#gen(), rng.next_u64(), rng.r#gen())],
        vec![Output::contract(rng.r#gen(), rng.r#gen(), rng.r#gen())],
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
    let predicate_gas_used = rng.r#gen();

    let owner = (*Contract::root_from_code(&predicate)).into();

    let input_coin = Input::coin_predicate(
        rng.r#gen(),
        owner,
        rng.next_u64(),
        rng.r#gen(),
        rng.r#gen(),
        predicate_gas_used,
        predicate.clone(),
        predicate_data,
    );

    for inputs in inputs.iter() {
        for outputs in outputs.iter() {
            for witnesses in witnesses.iter() {
                let mut inputs = inputs.clone();
                let offset = inputs.len();
                inputs.push(input_coin.clone());

                let tx = Transaction::upgrade_consensus_parameters(
                    &ConsensusParameters::default(),
                    Policies::new().with_maturity(maturity),
                    inputs,
                    outputs.clone(),
                    witnesses.clone(),
                )
                .unwrap();

                let mut tx_p = tx.clone();
                tx_p.precompute(&Default::default())
                    .expect("Should be able to calculate cache");

                let bytes = tx.to_bytes();
                let (offset, len) = tx
                    .inputs_predicate_offset_at(offset)
                    .expect("Failed to fetch offset");

                assert_ne!(bytes::padded_len(&predicate), Some(predicate.len()));
                assert_eq!(bytes::padded_len(&predicate), Some(len));

                assert_eq!(
                    predicate.as_slice(),
                    &bytes[offset..offset + predicate.len()]
                );
            }
        }
    }
}

#[allow(non_snake_case)]
#[test]
fn upload__inputs_predicate_offset_at__returns_offset_to_the_predicate() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let maturity = 10.into();

    let inputs: Vec<Vec<Input>> = vec![
        vec![],
        vec![Input::contract(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        )],
        vec![
            Input::contract(
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
            ),
            Input::contract(
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
                rng.r#gen(),
            ),
        ],
    ];
    let outputs: Vec<Vec<Output>> = vec![
        vec![],
        vec![Output::coin(rng.r#gen(), rng.next_u64(), rng.r#gen())],
        vec![Output::contract(rng.r#gen(), rng.r#gen(), rng.r#gen())],
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
    let predicate_gas_used = rng.r#gen();

    let owner = (*Contract::root_from_code(&predicate)).into();

    let input_coin = Input::coin_predicate(
        rng.r#gen(),
        owner,
        rng.next_u64(),
        rng.r#gen(),
        rng.r#gen(),
        predicate_gas_used,
        predicate.clone(),
        predicate_data,
    );

    for inputs in inputs.iter() {
        for outputs in outputs.iter() {
            for witnesses in witnesses.iter() {
                // Given
                let mut inputs = inputs.clone();
                let offset = inputs.len();
                inputs.push(input_coin.clone());

                let subsections = UploadSubsection::split_bytecode(&[123; 2048], 1023)
                    .expect("Failed to split bytecode");
                let tx = Transaction::upload_from_subsection(
                    subsections[0].clone(),
                    Policies::new().with_maturity(maturity),
                    inputs,
                    outputs.clone(),
                    witnesses.clone(),
                );

                // WHen
                let mut tx_p = tx.clone();
                tx_p.precompute(&Default::default())
                    .expect("Should be able to calculate cache");

                // Then
                let bytes = tx.to_bytes();
                let (offset, len) = tx
                    .inputs_predicate_offset_at(offset)
                    .expect("Failed to fetch offset");

                assert_ne!(bytes::padded_len(&predicate), Some(predicate.len()));
                assert_eq!(bytes::padded_len(&predicate), Some(len));

                assert_eq!(
                    predicate.as_slice(),
                    &bytes[offset..offset + predicate.len()]
                );
            }
        }
    }
}
