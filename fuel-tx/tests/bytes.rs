use fuel_asm::Opcode;
use fuel_tx::*;
use fuel_tx_test_helpers::{generate_bytes, generate_nonempty_bytes};
use fuel_types::{bytes, Immediate24};
use rand::rngs::StdRng;
use rand::{Rng, RngCore, SeedableRng};

use std::fmt;
use std::io::{self, Read, Write};

pub fn assert_encoding_correct<'a, T>(data: &[T])
where
    T: Read
        + Write
        + fmt::Debug
        + Clone
        + PartialEq
        + bytes::SizedBytes
        + bytes::SerializableVec
        + bytes::Deserializable
        + serde::Serialize
        + serde::Deserialize<'a>,
{
    let mut buffer;

    for data in data.iter() {
        let d_s = bincode::serialize(&data).expect("Failed to serialize data");
        // Safety: bincode/serde fails to understand the elision so this is a cheap way to convince it
        let d_s: T = bincode::deserialize(unsafe { std::mem::transmute(d_s.as_slice()) })
            .expect("Failed to deserialize data");

        assert_eq!(&d_s, data);

        let mut d = data.clone();

        let d_bytes = data.clone().to_bytes();
        let d_p = T::from_bytes(d_bytes.as_slice()).expect("Failed to deserialize T");

        assert_eq!(d, d_p);

        let mut d_p = data.clone();

        buffer = vec![0u8; 2048];
        let read_size = d.read(buffer.as_mut_slice()).expect("Failed to read");
        let write_size = d_p.write(buffer.as_slice()).expect("Failed to write");

        // Simple RW assertion
        assert_eq!(d, d_p);
        assert_eq!(read_size, write_size);

        buffer = vec![0u8; read_size];

        // Minimum size buffer assertion
        let _ = d.read(buffer.as_mut_slice()).expect("Failed to read");
        let _ = d_p.write(buffer.as_slice()).expect("Failed to write");
        assert_eq!(d, d_p);
        assert_eq!(d_bytes.as_slice(), buffer.as_slice());

        // No panic assertion
        loop {
            buffer.pop();

            let err = d
                .read(buffer.as_mut_slice())
                .err()
                .expect("Insufficient buffer should fail!");
            assert_eq!(io::ErrorKind::UnexpectedEof, err.kind());

            let err = d_p
                .write(buffer.as_slice())
                .err()
                .expect("Insufficient buffer should fail!");
            assert_eq!(io::ErrorKind::UnexpectedEof, err.kind());

            if buffer.is_empty() {
                break;
            }
        }
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
            rng.next_u64(),
        ),
        Input::coin_predicate(
            rng.gen(),
            rng.gen(),
            rng.next_u64(),
            rng.gen(),
            rng.gen(),
            generate_nonempty_bytes(rng),
            generate_bytes(rng),
        ),
        Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen()),
    ]);
}

#[test]
fn output() {
    let rng = &mut StdRng::seed_from_u64(8586);

    assert_encoding_correct(&[
        Output::coin(rng.gen(), rng.next_u64(), rng.gen()),
        Output::contract(rng.gen(), rng.gen(), rng.gen()),
        Output::withdrawal(rng.gen(), rng.next_u64(), rng.gen()),
        Output::change(rng.gen(), rng.next_u64(), rng.gen()),
        Output::variable(rng.gen(), rng.next_u64(), rng.gen()),
        Output::contract_created(rng.gen(), rng.gen()),
    ]);
}

#[test]
fn receipt() {
    let rng = &mut StdRng::seed_from_u64(8586);

    assert_encoding_correct(&[
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
            rng.gen(),
            rng.gen(),
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
            rng.gen(),
            rng.gen(),
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
            InstructionResult::success(),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::Revert,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::OutOfGas,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::TransactionValidity,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::MemoryOverflow,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::ArithmeticOverflow,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::ContractNotFound,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::MemoryOwnership,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::NotEnoughBalance,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::ExpectedInternalContext,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::AssetIdNotFound,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::InputNotFound,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::OutputNotFound,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::WitnessNotFound,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::TransactionMaturity,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::InvalidMetadataIdentifier,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::MalformedCallStructure,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::ReservedRegisterNotWritable,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::ErrorFlag,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::InvalidImmediateValue,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::ExpectedCoinInput,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::MaxMemoryAccess,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::MemoryWriteOverlap,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::ContractNotInInputs,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::InternalBalanceOverflow,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::ContractMaxSize,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::ExpectedUnallocatedStack,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::TransferAmountCannotBeZero,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::ExpectedOutputVariable,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::panic(
            rng.gen(),
            InstructionResult::error(
                PanicReason::ExpectedParentInternalContext,
                Opcode::JI(rng.gen::<Immediate24>() & 0xffffff).into(),
            ),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::script_result(ScriptExecutionResult::Success, rng.gen()),
        Receipt::script_result(ScriptExecutionResult::Panic, rng.gen()),
        Receipt::script_result(ScriptExecutionResult::Revert, rng.gen()),
        Receipt::script_result(ScriptExecutionResult::GenericFailure(rng.gen()), rng.gen()),
    ]);
}

#[test]
fn transaction() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let i = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen());
    let o = Output::coin(rng.gen(), rng.next_u64(), rng.gen());
    let w = rng.gen::<Witness>();
    let s = rng.gen::<StorageSlot>();

    assert_encoding_correct(&[
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.gen::<Witness>().into_inner(),
            rng.gen::<Witness>().into_inner(),
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            vec![],
            generate_bytes(rng),
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.gen::<Witness>().into_inner(),
            vec![],
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            vec![],
            vec![],
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            vec![],
            vec![],
            vec![],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            vec![],
            vec![],
            vec![],
            vec![],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ),
        Transaction::create(
            rng.next_u64(),
            ConsensusParameters::DEFAULT.max_gas_per_tx,
            rng.next_u64(),
            rng.next_u64(),
            rng.gen(),
            rng.gen(),
            vec![s.clone()],
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::create(
            rng.next_u64(),
            ConsensusParameters::DEFAULT.max_gas_per_tx,
            rng.next_u64(),
            rng.next_u64(),
            rng.gen(),
            rng.gen(),
            vec![s],
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::create(
            rng.next_u64(),
            ConsensusParameters::DEFAULT.max_gas_per_tx,
            rng.next_u64(),
            rng.next_u64(),
            rng.gen(),
            rng.gen(),
            vec![],
            vec![i],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::create(
            rng.next_u64(),
            ConsensusParameters::DEFAULT.max_gas_per_tx,
            rng.next_u64(),
            rng.next_u64(),
            rng.gen(),
            rng.gen(),
            vec![],
            vec![],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::create(
            rng.next_u64(),
            ConsensusParameters::DEFAULT.max_gas_per_tx,
            rng.next_u64(),
            rng.next_u64(),
            rng.gen(),
            rng.gen(),
            vec![],
            vec![],
            vec![],
            vec![w],
        ),
        Transaction::create(
            rng.next_u64(),
            ConsensusParameters::DEFAULT.max_gas_per_tx,
            rng.next_u64(),
            rng.next_u64(),
            rng.gen(),
            rng.gen(),
            vec![],
            vec![],
            vec![],
            vec![],
        ),
    ]);
}

#[test]
fn create_input_coin_data_offset() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let gas_price = 100;
    let gas_limit = 1000;
    let byte_price = 20;
    let maturity = 10;
    let bytecode_witness_index = 0x00;
    let salt = rng.gen();

    let storage_slots: Vec<Vec<StorageSlot>> =
        vec![vec![], vec![rng.gen()], vec![rng.gen(), rng.gen()]];
    let inputs: Vec<Vec<Input>> = vec![
        vec![],
        vec![Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen())],
        vec![Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen()); 2],
    ];
    let outputs: Vec<Vec<Output>> = vec![
        vec![],
        vec![Output::coin(rng.gen(), rng.next_u64(), rng.gen())],
        vec![
            Output::contract(rng.gen(), rng.gen(), rng.gen()),
            Output::withdrawal(rng.gen(), rng.next_u64(), rng.gen()),
        ],
    ];
    let witnesses: Vec<Vec<Witness>> = vec![
        vec![],
        vec![generate_bytes(rng).into()],
        vec![generate_bytes(rng).into(), generate_bytes(rng).into()],
    ];

    let predicate = generate_nonempty_bytes(rng);
    let predicate_data = generate_bytes(rng);

    let owner = (*Contract::root_from_code(&predicate)).into();

    let input_coin = Input::coin_predicate(
        rng.gen(),
        owner,
        rng.next_u64(),
        rng.gen(),
        rng.gen(),
        predicate.clone(),
        predicate_data,
    );

    let mut buffer = vec![0u8; 4096];
    for storage_slot in storage_slots.iter() {
        for inputs in inputs.iter() {
            for outputs in outputs.iter() {
                for witnesses in witnesses.iter() {
                    let mut inputs = inputs.clone();
                    let last_input = inputs.len();
                    inputs.push(input_coin.clone());

                    dbg!(&inputs);

                    let mut tx = Transaction::create(
                        gas_price,
                        gas_limit,
                        byte_price,
                        maturity,
                        bytecode_witness_index,
                        salt,
                        storage_slot.clone(),
                        inputs,
                        outputs.clone(),
                        witnesses.clone(),
                    );

                    let mut tx_p = tx.clone();
                    tx_p.precompute_metadata();

                    dbg!(&tx_p.metadata());

                    buffer.iter_mut().for_each(|b| *b = 0x00);
                    let _ = tx
                        .read(buffer.as_mut_slice())
                        .expect("Failed to serialize input");

                    let (offset, len) = tx
                        .input_coin_predicate_offset(last_input)
                        .expect("Failed to fetch offset");

                    let (offset_p, _) = tx_p
                        .input_coin_predicate_offset(last_input)
                        .expect("Failed to fetch offset from tx with precomputed metadata!");

                    assert_eq!(offset, offset_p);
                    assert_eq!(
                        predicate.as_slice(),
                        &buffer[offset..offset + len][..predicate.len()]
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
    let byte_price = 20;
    let maturity = 10;

    let script: Vec<Vec<u8>> = vec![vec![], generate_bytes(rng)];
    let script_data: Vec<Vec<u8>> = vec![vec![], generate_bytes(rng)];

    let inputs: Vec<Vec<Input>> = vec![
        vec![],
        vec![Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen())],
        vec![
            Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen()),
            Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen()),
        ],
    ];
    let outputs: Vec<Vec<Output>> = vec![
        vec![],
        vec![Output::coin(rng.gen(), rng.next_u64(), rng.gen())],
        vec![
            Output::contract(rng.gen(), rng.gen(), rng.gen()),
            Output::withdrawal(rng.gen(), rng.next_u64(), rng.gen()),
        ],
    ];
    let witnesses: Vec<Vec<Witness>> = vec![
        vec![],
        vec![generate_bytes(rng).into()],
        vec![generate_bytes(rng).into(), generate_bytes(rng).into()],
    ];

    let mut predicate = generate_nonempty_bytes(rng);

    // force word-unaligned predicate
    if predicate.len() % 2 == 0 {
        predicate.push(0xff);
    }

    let predicate_data = generate_bytes(rng);

    let owner = (*Contract::root_from_code(&predicate)).into();

    let input_coin = Input::coin_predicate(
        rng.gen(),
        owner,
        rng.next_u64(),
        rng.gen(),
        rng.gen(),
        predicate.clone(),
        predicate_data,
    );

    let mut buffer = vec![0u8; 4096];
    for script in script.iter() {
        for script_data in script_data.iter() {
            for inputs in inputs.iter() {
                for outputs in outputs.iter() {
                    for witnesses in witnesses.iter() {
                        let mut inputs = inputs.clone();
                        let offset = inputs.len();
                        inputs.push(input_coin.clone());

                        let mut tx = Transaction::script(
                            gas_price,
                            gas_limit,
                            byte_price,
                            maturity,
                            script.clone(),
                            script_data.clone(),
                            inputs,
                            outputs.clone(),
                            witnesses.clone(),
                        );

                        let mut tx_p = tx.clone();
                        tx_p.precompute_metadata();

                        buffer.iter_mut().for_each(|b| *b = 0x00);

                        let _ = tx
                            .read(buffer.as_mut_slice())
                            .expect("Failed to serialize input");

                        let script_offset = Transaction::script_offset();
                        assert_eq!(
                            script.as_slice(),
                            &buffer[script_offset..script_offset + script.len()]
                        );

                        let script_data_offset = tx
                            .script_data_offset()
                            .expect("Transaction is Script and should return data offset");

                        let script_data_offset_p = tx_p.script_data_offset().expect(
                            "Transaction metadata script data offset failed after precompute!",
                        );

                        assert_eq!(script_data_offset, script_data_offset_p);
                        assert_eq!(
                            script_data.as_slice(),
                            &buffer[script_data_offset..script_data_offset + script_data.len()]
                        );

                        let (offset, len) = tx
                            .input_coin_predicate_offset(offset)
                            .expect("Failed to fetch offset");

                        assert_ne!(predicate.len(), len);
                        assert_eq!(bytes::padded_len(&predicate), len);

                        assert_eq!(
                            predicate.as_slice(),
                            &buffer[offset..offset + predicate.len()]
                        );
                    }
                }
            }
        }
    }
}
