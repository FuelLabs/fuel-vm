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
    Interpreter::<PredicateStorage>::check_predicates(tx, Default::default(), Default::default()).is_ok()
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

/// Returns the amount of gas used if verification succeeds
fn execute_gas_metered_predicates(predicates: Vec<Vec<Opcode>>) -> Result<u64, ()> {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let gas_price = 1_000;
    let gas_limit = 1_000_000;
    let script = vec![];
    let script_data = vec![];

    let mut builder = TransactionBuilder::script(script, script_data);
    builder.gas_price(gas_price).gas_limit(gas_limit).maturity(0);

    let coin_amount = 10_000_000;

    if predicates.is_empty() {
        builder.add_unsigned_coin_input(rng.gen(), rng.gen(), coin_amount, AssetId::default(), rng.gen(), 0);
    }

    for predicate in predicates {
        let predicate: Vec<u8> = predicate
            .into_iter()
            .flat_map(|op| u32::from(op).to_be_bytes())
            .collect();

        let owner = Input::predicate_owner(&predicate);
        let input = Input::coin_predicate(
            rng.gen(),
            owner,
            coin_amount,
            AssetId::default(),
            rng.gen(),
            0,
            predicate,
            vec![],
        );

        builder.add_input(input);
    }

    let tx = builder.finalize_checked_basic(0, &ConsensusParameters::default());
    Interpreter::<PredicateStorage>::check_predicates(tx, Default::default(), Default::default()).map(|r| r.gas_used())
}

#[test]
fn predicate_gas_metering() {
    // This just succeeds
    assert!(execute_gas_metered_predicates(vec![vec![Opcode::RET(REG_ONE)]]).is_ok());

    // This runs out of gas
    assert!(execute_gas_metered_predicates(vec![vec![
        Opcode::JI(0), // Infinite loop
    ]])
    .is_err());

    // Multiple Predicate Success
    assert!(execute_gas_metered_predicates(vec![
        vec![Opcode::RET(REG_ONE)],
        vec![Opcode::MOVI(0x10, 0x11), Opcode::MOVI(0x10, 0x11), Opcode::RET(REG_ONE)],
    ])
    .is_ok());

    // Running predicate gas used is combined properly
    let gas_used_by: Vec<_> = (0..4)
        .map(|n| execute_gas_metered_predicates(vec![vec![Opcode::RET(REG_ONE)]; n]).unwrap())
        .collect();
    assert_eq!(gas_used_by[0], 0);
    assert_ne!(gas_used_by[1], 0);
    assert_eq!(gas_used_by[1] * 2, gas_used_by[2]);
    assert_eq!(gas_used_by[1] * 3, gas_used_by[3]);
}
