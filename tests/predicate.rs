use fuel_types::bytes;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use fuel_vm::consts::*;
use fuel_vm::prelude::*;

use core::iter;

fn execute_predicate<P>(predicate: P, predicate_data: Vec<u8>) -> bool
where
    P: IntoIterator<Item = Opcode>,
{
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let predicate: Vec<u8> = predicate
        .into_iter()
        .map(|op| u32::from(op).to_be_bytes())
        .flatten()
        .collect();

    let utxo_id = rng.gen();
    let amount = 0;
    let asset_id = rng.gen();
    let maturity = 0;

    let owner = Input::predicate_owner(&predicate);
    let input = Input::coin_predicate(utxo_id, owner, amount, asset_id, maturity, predicate, predicate_data);

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let script = vec![];
    let script_data = vec![];

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        script,
        script_data,
        vec![input],
        vec![],
        vec![],
    );

    Interpreter::<PredicateStorage>::check_predicates(tx, Default::default())
}

#[test]
fn predicate_minimal() {
    let predicate = iter::once(Opcode::RET(0x01));
    let data = vec![];

    assert!(execute_predicate(predicate, data));
}

#[test]
fn predicate() {
    let expected_data = 0x23 as Word;
    let expected_data = expected_data.to_be_bytes().to_vec();
    let expected_data_len = bytes::padded_len(expected_data.as_slice()) as Immediate12;

    let wrong_data = 0x24 as Word;
    let wrong_data = wrong_data.to_be_bytes().to_vec();

    // A script that will succeed only if the argument is 0x23
    let mut predicate = vec![];

    predicate.push(Opcode::MOVI(0x10, 0x11));
    predicate.push(Opcode::ADDI(0x11, 0x10, 0x12));
    predicate.push(Opcode::MOVI(0x12, 0x08));
    predicate.push(Opcode::ALOC(0x12));
    predicate.push(Opcode::ADDI(0x12, REG_HP, 0x01));
    predicate.push(Opcode::SW(0x12, 0x11, 0));
    predicate.push(Opcode::MOVI(0x10, 0x08));
    predicate.push(Opcode::XIL(0x20, 0));
    predicate.push(Opcode::XIS(0x11, 0));
    predicate.push(Opcode::ADD(0x11, 0x11, 0x20));
    predicate.push(Opcode::SUBI(0x11, 0x11, expected_data_len));
    predicate.push(Opcode::MEQ(0x10, 0x11, 0x12, 0x10));
    predicate.push(Opcode::RET(0x10));

    assert!(execute_predicate(predicate.iter().copied(), expected_data));
    assert!(!execute_predicate(predicate.iter().copied(), wrong_data));
}
