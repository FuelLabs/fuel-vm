use crate::{
    checked_transaction::{
        CheckPredicateParams,
        EstimatePredicates,
    },
    prelude::{
        MemoryInstance,
        *,
    },
};
use core::str::FromStr;
use fuel_asm::{
    op,
    RegId,
};
use fuel_tx::{
    field::{
        Inputs,
        Outputs,
        ReceiptsRoot,
        StorageSlots,
    },
    input::coin::CoinPredicate,
    ConsensusParameters,
    Input,
    TransactionBuilder,
    UtxoId,
};
use fuel_types::BlockHeight;
use rand::{
    rngs::StdRng,
    Rng,
    SeedableRng,
};

use crate::storage::predicate::EmptyStorage;
#[cfg(feature = "alloc")]
use alloc::vec;

#[test]
fn transaction_can_be_executed_after_maturity() {
    const MATURITY: BlockHeight = BlockHeight::new(1);
    const BLOCK_HEIGHT: BlockHeight = BlockHeight::new(2);

    let arb_max_fee = 1;

    let rng = &mut StdRng::seed_from_u64(2322u64);
    let tx = TransactionBuilder::script(
        Some(op::ret(1)).into_iter().collect(),
        Default::default(),
    )
    .max_fee_limit(arb_max_fee)
    .add_unsigned_coin_input(
        SecretKey::random(rng),
        rng.gen(),
        arb_max_fee,
        Default::default(),
        rng.gen(),
    )
    .script_gas_limit(100)
    .maturity(MATURITY)
    .finalize_checked(BLOCK_HEIGHT);

    let result = TestBuilder::new(2322u64)
        .block_height(BLOCK_HEIGHT)
        .execute_tx(tx);
    assert!(result.is_ok());
}

#[test]
fn transaction__execution__works_before_expiration() {
    let arb_max_fee = 1;

    let rng = &mut StdRng::seed_from_u64(2322u64);

    // Given
    const EXPIRATION: BlockHeight = BlockHeight::new(2);
    const BLOCK_HEIGHT: BlockHeight = BlockHeight::new(1);
    let tx = TransactionBuilder::script(
        Some(op::ret(1)).into_iter().collect(),
        Default::default(),
    )
    .max_fee_limit(arb_max_fee)
    .add_unsigned_coin_input(
        SecretKey::random(rng),
        rng.gen(),
        arb_max_fee,
        Default::default(),
        rng.gen(),
    )
    .script_gas_limit(100)
    .expiration(EXPIRATION)
    .finalize_checked(BLOCK_HEIGHT);

    // When
    let result = TestBuilder::new(2322u64)
        .block_height(BLOCK_HEIGHT)
        .execute_tx(tx);

    // Then
    assert!(result.is_ok());
}

#[test]
fn transaction__execution__works_current_height_expiration() {
    let arb_max_fee = 1;

    let rng = &mut StdRng::seed_from_u64(2322u64);

    // Given
    const EXPIRATION: BlockHeight = BlockHeight::new(1);
    const BLOCK_HEIGHT: BlockHeight = BlockHeight::new(1);
    let tx = TransactionBuilder::script(
        Some(op::ret(1)).into_iter().collect(),
        Default::default(),
    )
    .max_fee_limit(arb_max_fee)
    .add_unsigned_coin_input(
        SecretKey::random(rng),
        rng.gen(),
        arb_max_fee,
        Default::default(),
        rng.gen(),
    )
    .script_gas_limit(100)
    .expiration(EXPIRATION)
    .finalize_checked(BLOCK_HEIGHT);

    // When
    let result = TestBuilder::new(2322u64)
        .block_height(BLOCK_HEIGHT)
        .execute_tx(tx);

    // Then
    assert!(result.is_ok());
}

/// Malleable fields should not affect validity of the create transaction
#[allow(deprecated)]
#[test]
fn malleable_fields_do_not_affect_validity_of_create() {
    let params = ConsensusParameters::default();

    let tx_start_ptr = params.tx_params().tx_offset();
    let tx_size_ptr = tx_start_ptr - 8;
    let witness_count_offset =
        <Create as StorageSlots>::storage_slots_offset_static() - 8;

    let predicate_bytecode = [
        // Load tx size
        op::movi(0x21, tx_size_ptr as u32),
        op::lw(0x21, 0x21, 0),
        // Make heap space for tx bytes, chain_id and computed tx hash
        op::addi(0x22, 0x21, 8 + 32),
        op::aloc(0x22),
        // Construct the chain_id and tx bytes for hashing
        op::addi(0x22, RegId::HP, 32), // Chain id position
        op::gtf_args(0x20, 0x00, GTFArgs::InputCoinPredicateData),
        op::mcpi(0x22, 0x20, 8), // Copy chain id
        op::movi(0x20, tx_start_ptr as u32),
        op::addi(0x23, 0x22, 8),   // Tx bytes position
        op::mcp(0x23, 0x20, 0x21), // Copy tx bytes
        // Assert that there is exactly one witness
        op::gtf_args(0x26, 0x00, GTFArgs::ScriptWitnessesCount),
        op::eq(0x26, 0x26, RegId::ONE),
        op::jnzf(0x26, 0x00, 1),
        op::ret(0),
        // Zero out witness count
        op::addi(0x26, 0x23, witness_count_offset as u16), // Offset to witness count
        op::sw(0x26, RegId::ZERO, 0),                      // Zero out the witness count
        // Actually hash. We use property that contract bytecode has zero size.
        op::s256(RegId::HP, 0x22, 0x21), // Compute tx id hash
        op::movi(0x25, 32),              // Hash size
        // Compare two hashes
        op::meq(0x26, RegId::HP, 0x0, 0x25),
        // Done
        op::ret(0x26),
    ]
    .into_iter()
    .collect();
    let predicate_data = params.chain_id().to_be_bytes().to_vec();
    let predicate_owner = Input::predicate_owner(&predicate_bytecode);

    let mut tx = TransactionBuilder::create(vec![].into(), Salt::zeroed(), vec![])
        .add_input(Input::coin_predicate(
            UtxoId::new([4; 32].into(), 0),
            predicate_owner,
            123456789,
            AssetId::default(),
            Default::default(),
            0,
            predicate_bytecode,
            predicate_data,
        ))
        .add_contract_created()
        .add_output(Output::change(
            Default::default(),
            Default::default(),
            Default::default(),
        ))
        .finalize();
    tx.estimate_predicates(
        &CheckPredicateParams::from(&params),
        MemoryInstance::new(),
        &EmptyStorage,
    )
    .expect("Should estimate predicate");

    let run_tx = |tx: Create| tx.into_checked(0u32.into(), &params).map(|_| ());
    let result = run_tx(tx.clone());
    assert_eq!(result, Ok(()));

    // Check that modifying the input coin doesn't affect validity
    {
        let mut tx = tx.clone();

        match tx.inputs_mut()[0] {
            Input::CoinPredicate(CoinPredicate {
                ref mut tx_pointer, ..
            }) => {
                #[cfg(not(feature = "u32-tx-pointer"))]
                {
                    *tx_pointer = TxPointer::from_str("123456780001").unwrap()
                }
                #[cfg(feature = "u32-tx-pointer")]
                {
                    *tx_pointer = TxPointer::from_str("1234567800000001").unwrap()
                }
            }
            _ => unreachable!(),
        };

        match tx.outputs_mut()[1] {
            Output::Change { ref mut amount, .. } => {
                *amount = 123456789;
            }
            _ => unreachable!(),
        };
        let result = run_tx(tx);
        assert_eq!(result, Ok(()));
    }
}

/// Malleable fields should not affect validity of the script
#[test]
fn malleable_fields_do_not_affect_validity_of_script() {
    let params = ConsensusParameters::default();

    let tx_start_ptr = params.tx_params().tx_offset();
    let tx_size_ptr = tx_start_ptr - 8;

    let predicate_bytecode = [
        // Load tx size
        op::movi(0x21, tx_size_ptr as u32),
        op::lw(0x21, 0x21, 0),
        // Make heap space for tx bytes, chain_id and computed tx hash
        op::addi(0x22, 0x21, 8 + 32),
        op::aloc(0x22),
        // Construct the chain_id and tx bytes for hashing
        op::addi(0x22, RegId::HP, 32), // Chain id position
        op::gtf_args(0x20, 0x00, GTFArgs::InputCoinPredicateData),
        op::mcpi(0x22, 0x20, 8), // Copy chain id
        op::movi(0x20, tx_start_ptr as u32),
        op::addi(0x23, 0x22, 8),   // Tx bytes position
        op::mcp(0x23, 0x20, 0x21), // Copy tx bytes
        // Actually hash
        op::addi(0x24, 0x21, 8),         // Offset ptr
        op::s256(RegId::HP, 0x22, 0x24), // Compute tx id hash
        op::movi(0x25, 32),              // Hash size
        // Compare two hashes
        op::meq(0x26, RegId::HP, 0x0, 0x25),
        // Done
        op::ret(0x26),
    ]
    .into_iter()
    .collect();
    let predicate_data = params.chain_id().to_be_bytes().to_vec();
    let predicate_owner = Input::predicate_owner(&predicate_bytecode);

    let mut tx = TransactionBuilder::script(vec![], vec![])
        .add_input(Input::coin_predicate(
            UtxoId::new([4; 32].into(), 0),
            predicate_owner,
            123456789,
            AssetId::default(),
            Default::default(),
            0,
            predicate_bytecode,
            predicate_data,
        ))
        .add_input(Input::contract(
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Contract::EMPTY_CONTRACT_ID,
        ))
        .add_output(Output::variable(
            Default::default(),
            Default::default(),
            Default::default(),
        ))
        .add_output(Output::change(
            Default::default(),
            Default::default(),
            Default::default(),
        ))
        .add_output(Output::contract(1, Default::default(), Default::default()))
        .script_gas_limit(1_000_000)
        .finalize();
    tx.estimate_predicates(
        &CheckPredicateParams::from(&params),
        MemoryInstance::new(),
        &EmptyStorage,
    )
    .expect("Should estimate predicate");

    let run_tx = |tx: Script| tx.into_checked(0u32.into(), &params).map(|_| ());
    let result = run_tx(tx.clone());
    assert_eq!(result, Ok(()));

    // Check that modifying the input coin doesn't affect validity
    {
        let mut tx = tx.clone();

        *tx.receipts_root_mut() = [1u8; 32].into();

        match tx.inputs_mut()[0] {
            Input::CoinPredicate(CoinPredicate {
                ref mut tx_pointer, ..
            }) => {
                #[cfg(not(feature = "u32-tx-pointer"))]
                {
                    *tx_pointer = TxPointer::from_str("123456780001").unwrap()
                }
                #[cfg(feature = "u32-tx-pointer")]
                {
                    *tx_pointer = TxPointer::from_str("1234567800000001").unwrap()
                }
            }
            _ => unreachable!(),
        };

        match tx.inputs_mut()[1] {
            Input::Contract(input::contract::Contract {
                ref mut utxo_id,
                ref mut balance_root,
                ref mut state_root,
                ref mut tx_pointer,
                ..
            }) => {
                *utxo_id = UtxoId::new([1; 32].into(), 0);
                *balance_root = [2; 32].into();
                *state_root = [3; 32].into();
                #[cfg(not(feature = "u32-tx-pointer"))]
                {
                    *tx_pointer = TxPointer::from_str("123456780001").unwrap();
                }
                #[cfg(feature = "u32-tx-pointer")]
                {
                    *tx_pointer = TxPointer::from_str("1234567800000001").unwrap();
                }
            }
            _ => unreachable!(),
        };

        match tx.outputs_mut()[0] {
            Output::Variable {
                ref mut to,
                ref mut amount,
                ref mut asset_id,
            } => {
                *to = [123; 32].into();
                *amount = 123456789;
                *asset_id = [213; 32].into();
            }
            _ => unreachable!(),
        };

        match tx.outputs_mut()[1] {
            Output::Change { ref mut amount, .. } => {
                *amount = 123456789;
            }
            _ => unreachable!(),
        };

        match tx.outputs_mut()[2] {
            Output::Contract(output::contract::Contract {
                ref mut balance_root,
                ref mut state_root,
                ..
            }) => {
                *balance_root = [7; 32].into();
                *state_root = [8; 32].into();
            }
            _ => unreachable!(),
        };

        let result = run_tx(tx);
        assert_eq!(result, Ok(()));
    }
}
