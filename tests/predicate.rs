use fuel_tx::TransactionBuilder;
use rand::{rngs::StdRng, Rng, SeedableRng};

use fuel_vm::{consts::*, prelude::*};

use core::iter;

fn execute_predicate<P>(predicate: P, predicate_data: Vec<u8>, dummy_inputs: usize) -> bool
where
    P: IntoIterator<Item = Opcode>,
{
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let predicate: Vec<u8> = predicate
        .into_iter()
        .flat_map(|op| u32::from(op).to_be_bytes())
        .collect();

    let utxo_id = rng.gen();
    let amount = 0;
    let asset_id = rng.gen();
    let tx_pointer = rng.gen();
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::default();

    let owner = Input::predicate_owner(&predicate);
    let input = Input::coin_predicate(
        utxo_id,
        owner,
        amount,
        asset_id,
        tx_pointer,
        maturity,
        predicate,
        predicate_data,
    );

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let script = vec![];
    let script_data = vec![];

    let mut builder = TransactionBuilder::script(script, script_data);

    builder.gas_price(gas_price).gas_limit(gas_limit).maturity(maturity);

    (0..dummy_inputs).for_each(|_| {
        builder.add_unsigned_coin_input(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen(), maturity);
    });

    builder.add_input(input);

    let tx = builder.finalize_checked_basic(height, &params);
    Interpreter::<PredicateStorage>::check_predicates(tx, Default::default())
}

#[test]
fn predicate_minimal() {
    let predicate = iter::once(Opcode::RET(0x01));
    let data = vec![];

    assert!(execute_predicate(predicate, data, 7));
}

#[test]
fn predicate() {
    let expected_data = 0x23 as Word;
    let expected_data = expected_data.to_be_bytes().to_vec();

    let wrong_data = 0x24 as Word;
    let wrong_data = wrong_data.to_be_bytes().to_vec();

    // A script that will succeed only if the argument is 0x23
    let predicate = vec![
        Opcode::MOVI(0x10, 0x11),
        Opcode::ADDI(0x11, 0x10, 0x12),
        Opcode::MOVI(0x12, 0x08),
        Opcode::ALOC(0x12),
        Opcode::ADDI(0x12, REG_HP, 0x01),
        Opcode::SW(0x12, 0x11, 0),
        Opcode::MOVI(0x10, 0x08),
        Opcode::gtf(0x11, 0, GTFArgs::InputCoinPredicateData),
        Opcode::MEQ(0x10, 0x11, 0x12, 0x10),
        Opcode::RET(0x10),
    ];

    assert!(execute_predicate(predicate.iter().copied(), expected_data, 0));
    assert!(!execute_predicate(predicate.iter().copied(), wrong_data, 0));
}

#[test]
fn get_verifying_predicate() {
    let indices = vec![0, 4, 5, 7, 11];

    for idx in indices {
        #[rustfmt::skip]
        let predicate = vec![
            Opcode::gm(0x10, GMArgs::GetVerifyingPredicate),
            Opcode::MOVI(0x11, idx),
            Opcode::EQ(0x10, 0x10, 0x11),
            Opcode::RET(0x10),
        ];

        assert!(execute_predicate(predicate, vec![], idx as usize));
    }
}
