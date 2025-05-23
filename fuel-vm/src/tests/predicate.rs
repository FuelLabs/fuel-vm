#![cfg(feature = "std")]
#![allow(non_snake_case)]

use fuel_asm::{
    GMArgs,
    GTFArgs,
    Instruction,
    RegId,
    op,
};
use fuel_tx::TransactionBuilder;
use rand::{
    Rng,
    SeedableRng,
    rngs::StdRng,
};
use tokio_rayon::AsyncRayonHandle;

use crate::{
    error::PredicateVerificationFailed,
    pool::DummyPool,
    prelude::*,
};

use crate::{
    checked_transaction::{
        CheckError,
        CheckPredicateParams,
        CheckPredicates,
        EstimatePredicates,
        ParallelExecutor,
    },
    prelude::predicates::{
        check_predicates,
        check_predicates_async,
    },
    storage::predicate::EmptyStorage,
};
use core::iter;
use fuel_tx::{
    ConsensusParameters,
    consensus_parameters::gas::GasCostsValuesV5,
    field::{
        ChargeableBody,
        Inputs,
        Outputs,
        Policies,
        Script,
        ScriptData,
        Witnesses,
    },
};

pub struct TokioWithRayon;

#[async_trait::async_trait]
impl ParallelExecutor for TokioWithRayon {
    type Task = AsyncRayonHandle<(usize, Result<Word, PredicateVerificationFailed>)>;

    fn create_task<F>(func: F) -> Self::Task
    where
        F: FnOnce() -> (usize, Result<Word, PredicateVerificationFailed>)
            + Send
            + 'static,
    {
        tokio_rayon::spawn(func)
    }

    async fn execute_tasks(
        futures: Vec<Self::Task>,
    ) -> Vec<(usize, Result<Word, PredicateVerificationFailed>)> {
        futures::future::join_all(futures).await
    }
}

async fn execute_predicate<P>(
    predicate: P,
    predicate_data: Vec<u8>,
    dummy_inputs: usize,
) -> bool
where
    P: IntoIterator<Item = Instruction>,
{
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let predicate: Vec<u8> = predicate
        .into_iter()
        .flat_map(|op| u32::from(op).to_be_bytes())
        .collect();

    let utxo_id = rng.r#gen();
    let amount = 0;
    let asset_id = rng.r#gen();
    let tx_pointer = rng.r#gen();
    let maturity = Default::default();
    let height = Default::default();
    let predicate_gas_used = 0;

    let owner = Input::predicate_owner(&predicate);
    let input = Input::coin_predicate(
        utxo_id,
        owner,
        amount,
        asset_id,
        tx_pointer,
        predicate_gas_used,
        predicate,
        predicate_data,
    );

    let gas_limit = 1_000_000;
    let script = vec![];
    let script_data = vec![];

    let mut builder = TransactionBuilder::script(script, script_data);
    let params = ConsensusParameters::standard();
    let check_params = params.clone().into();

    builder.script_gas_limit(gas_limit).maturity(maturity);

    (0..dummy_inputs).for_each(|_| {
        builder.add_unsigned_coin_input(
            SecretKey::random(rng),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        );
    });

    builder.add_input(input);

    let mut transaction = builder.finalize();
    transaction
        .estimate_predicates(&check_params, MemoryInstance::new(), &EmptyStorage)
        .expect("Should estimate predicate");

    let checked = transaction
        .into_checked_basic(height, &params)
        .expect("Should successfully convert into Checked");

    let parallel_execution = {
        check_predicates_async::<_, TokioWithRayon>(
            &checked,
            &check_params,
            &DummyPool,
            &EmptyStorage,
        )
        .await
        .map(|checked| checked.gas_used())
    };

    let seq_execution = check_predicates(
        &checked,
        &check_params,
        MemoryInstance::new(),
        &EmptyStorage,
    )
    .map(|checked| checked.gas_used());

    match (parallel_execution, seq_execution) {
        (Ok(p_gas_used), Ok(s_gas_used)) => {
            assert_eq!(p_gas_used, s_gas_used);
            true
        }
        (Err(p_err), Err(s_err)) => {
            assert_eq!(p_err, s_err);
            false
        }
        _ => panic!("Parallel and sequential execution should return the same result"),
    }
}

#[test]
fn estimate_predicate_works_when_predicate_address_incorrect() {
    let mut rng = StdRng::seed_from_u64(2322u64);
    let predicate: Vec<u8> = iter::once(op::ret(0x01)).collect();
    let predicate_data = vec![];

    let mut builder = TransactionBuilder::script(vec![], vec![]);

    // Given
    let predicate_owner = rng.r#gen();
    let input = Input::coin_predicate(
        rng.r#gen(),
        predicate_owner,
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        0,
        predicate.clone(),
        predicate_data.clone(),
    );
    builder.add_input(input);
    let mut script = builder.finalize();

    // When
    assert_eq!(script.inputs()[0].predicate_gas_used(), Some(0));
    let result = script.estimate_predicates(
        &ConsensusParameters::standard().into(),
        MemoryInstance::new(),
        &EmptyStorage,
    );

    // Then
    result.expect("Should estimate predicate");
    assert_ne!(script.inputs()[0].predicate_gas_used(), Some(0));
}

#[test]
fn estimate_predicate_works_when_max_gas_per_predicate_less_than_tx_gas__10_inputs() {
    let mut rng = StdRng::seed_from_u64(2322u64);
    let predicate: Vec<u8> = iter::once(op::ret(0x01)).collect();
    let predicate_data = vec![];
    let predicate_owner = Input::predicate_owner(&predicate);

    let mut builder = TransactionBuilder::script(vec![], vec![]);

    const PREDICATE_COUNT: u64 = 10;
    const MAX_PREDICATE_GAS: u64 = 1_000_000;
    const MAX_TX_GAS: u64 = MAX_PREDICATE_GAS * PREDICATE_COUNT;

    for _ in 0..PREDICATE_COUNT {
        let input = Input::coin_predicate(
            rng.r#gen(),
            predicate_owner,
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            0,
            predicate.clone(),
            predicate_data.clone(),
        );

        builder.add_input(input.clone());
    }

    let mut script = builder.finalize();

    // Given
    let gas_costs = GasCostsValuesV5 {
        ret: MAX_PREDICATE_GAS,
        ..GasCostsValuesV5::free()
    };
    let gas_costs = GasCosts::new(gas_costs.into());
    let predicate_param =
        PredicateParameters::default().with_max_gas_per_predicate(MAX_PREDICATE_GAS);
    let tx_param = TxParameters::default().with_max_gas_per_tx(MAX_TX_GAS);
    let fee_param = FeeParameters::default().with_gas_per_byte(0);

    let mut consensus_params = ConsensusParameters::standard();
    consensus_params.set_gas_costs(gas_costs.clone());
    consensus_params.set_predicate_params(predicate_param);
    consensus_params.set_tx_params(tx_param);
    consensus_params.set_fee_params(fee_param);
    let gas_before_estimation = script.max_gas(&gas_costs, &fee_param);

    // When
    script
        .estimate_predicates(
            &consensus_params.into(),
            MemoryInstance::new(),
            &EmptyStorage,
        )
        .unwrap();

    // Then
    let gas_after_estimation = script.max_gas(&gas_costs, &fee_param);
    assert_eq!(gas_before_estimation, 0);
    assert_eq!(gas_after_estimation, MAX_TX_GAS);
}

#[tokio::test]
async fn predicate_minimal() {
    let predicate = iter::once(op::ret(0x01));
    let data = vec![];

    assert!(execute_predicate(predicate, data, 7).await);
}

#[tokio::test]
async fn predicate() {
    let expected_data = 0x23 as Word;
    let expected_data = expected_data.to_be_bytes().to_vec();

    let wrong_data = 0x24 as Word;
    let wrong_data = wrong_data.to_be_bytes().to_vec();

    // A script that will succeed only if the argument is 0x23
    let predicate = [
        op::movi(0x10, 0x11),
        op::addi(0x11, 0x10, 0x12),
        op::movi(0x12, 0x08),
        op::aloc(0x12),
        op::move_(0x12, RegId::HP),
        op::sw(0x12, 0x11, 0),
        op::movi(0x10, 0x08),
        op::gtf_args(0x11, 0, GTFArgs::InputCoinPredicateData),
        op::meq(0x10, 0x11, 0x12, 0x10),
        op::ret(0x10),
    ];

    assert!(execute_predicate(predicate.iter().copied(), expected_data, 0).await);
    assert!(!execute_predicate(predicate.iter().copied(), wrong_data, 0).await);
}

#[cfg(feature = "chargeable-tx-v2")]
#[tokio::test]
async fn script_v2__estimate_predicate_happy_path() {
    let mut rng = StdRng::seed_from_u64(2322u64);

    // given
    let script = vec![];
    let script_data = vec![];
    let predicate_index = 0;
    let predicate_data_index = 1;
    let asset_id = rng.r#gen();
    let amount = rng.r#gen();
    let input = Input::coin_predicate_v2(
        rng.r#gen(),
        rng.r#gen(),
        amount,
        asset_id,
        rng.r#gen(),
        predicate_index,
        predicate_data_index,
    );
    let predicate_bytes: Vec<_> = vec![op::ret(RegId::ONE)].into_iter().collect();

    let predicate_static_witness = Witness::from(predicate_bytes);
    let predicate_data_static_witness = Witness::from(vec![]);

    let mut tx = TransactionBuilder::script_v2(script, script_data)
        .add_input(input)
        .add_output(Output::coin(rng.r#gen(), amount, asset_id))
        .add_static_witness(predicate_static_witness)
        .add_static_witness(predicate_data_static_witness)
        .finalize();

    assert_eq!(tx.inputs()[0].predicate_gas_used(), Some(0));
    let result = tx.estimate_predicates(
        &ConsensusParameters::standard().into(),
        MemoryInstance::new(),
        &EmptyStorage,
    );

    // Then
    result.expect("Should estimate predicate");
    assert_ne!(tx.inputs()[0].predicate_gas_used(), Some(0));
}

#[cfg(feature = "chargeable-tx-v2")]
#[tokio::test]
async fn test_serialization() {
    use fuel_types::canonical::Serialize;

    let mut rng = StdRng::seed_from_u64(2322u64);
    let predicate_index = 0;
    let predicate_data_index = 1;
    let asset_id = rng.r#gen();
    let amount = rng.r#gen();
    let input = Input::coin_predicate_v2(
        rng.r#gen(),
        rng.r#gen(),
        amount,
        asset_id,
        rng.r#gen(),
        predicate_index,
        predicate_data_index,
    );

    let script = vec![1; 4];
    let script_data = vec![9; 15];

    let predicate_bytes: Vec<_> = vec![op::ret(RegId::ONE)].into_iter().collect();
    let predicate_static_witness = Witness::from(predicate_bytes);
    let predicate_data_static_witness = Witness::from(vec![]);

    let mut tx_1 = TransactionBuilder::script(script.clone(), script_data.clone())
        .add_input(input.clone())
        .add_output(Output::coin(rng.r#gen(), amount, asset_id))
        .finalize();
    let mut tx_2 = TransactionBuilder::script_v2(script, script_data)
        .add_input(input)
        .add_output(Output::coin(rng.r#gen(), amount, asset_id))
        .add_static_witness(predicate_static_witness)
        .add_static_witness(predicate_data_static_witness)
        .finalize();
    let tx_1_bytes = tx_1.to_bytes();
    let tx_2_bytes = tx_2.to_bytes();

    let script_offset = tx_1.script_offset();
    let script_offset_v2 = tx_2.script_offset();
    let script_data_offset = tx_1.script_data_offset();
    let script_data_offset_v2 = tx_2.script_data_offset();
    let body_offset_end = tx_1.body_offset_end();
    let body_offset_end_v2 = tx_2.body_offset_end();
    let policies_offset = tx_1.policies_offset();
    let policies_offset_v2 = tx_2.policies_offset();
    let input_offset = tx_1.inputs_offset();
    let input_offset_v2 = tx_2.inputs_offset();
    let output_offset = tx_1.outputs_offset();
    let output_offset_v2 = tx_2.outputs_offset();
    let witness_offset = tx_1.witnesses_offset();
    let witness_offset_v2 = tx_2.witnesses_offset();

    let range = 0..script_offset;
    let range_v2 = 0..script_offset_v2;
    println!(
        "range v1 len: {}, range v2 len: {}",
        range.len(),
        range_v2.len()
    );
    println!(
        "front \ntx_1: {:?}, \ntx_2: {:?}",
        &tx_1_bytes[range], &tx_2_bytes[range_v2]
    );
    let script_bytes_range = script_offset..script_data_offset;
    let script_bytes_range_v2 = script_offset_v2..script_data_offset_v2;
    println!(
        "script bytes range v1 len: {}, script bytes range v2 len: {}",
        script_bytes_range.len(),
        script_bytes_range_v2.len()
    );

    println!(
        "script \ntx_1: {:?}, \ntx_2: {:?}",
        &tx_1_bytes[script_bytes_range], &tx_2_bytes[script_bytes_range_v2]
    );

    let script_data_bytes_range = script_data_offset..body_offset_end;
    let script_data_bytes_range_v2 = script_data_offset_v2..body_offset_end_v2;
    println!(
        "script data bytes range v1 len: {}, script data bytes range v2 len: {}",
        script_data_bytes_range.len(),
        script_data_bytes_range_v2.len()
    );
    println!(
        "script_data \ntx_1: {:?}, \ntx_2: {:?}",
        &tx_1_bytes[script_data_bytes_range], &tx_2_bytes[script_data_bytes_range_v2]
    );
    let body_bytes_range = body_offset_end..policies_offset;
    let body_bytes_range_v2 = body_offset_end_v2..policies_offset_v2;
    println!(
        "body bytes range v1 len: {}, body bytes range v2 len: {}",
        body_bytes_range.len(),
        body_bytes_range_v2.len()
    );
    println!(
        "body \ntx_1: {:?}, \ntx_2: {:?}",
        &tx_1_bytes[body_bytes_range], &tx_2_bytes[body_bytes_range_v2]
    );
    let policies_bytes_range = policies_offset..input_offset;
    let policies_bytes_range_v2 = policies_offset_v2..input_offset_v2;
    println!(
        "policies bytes range v1 len: {}, policies bytes range v2 len: {}",
        policies_bytes_range.len(),
        policies_bytes_range_v2.len()
    );
    println!(
        "policies \ntx_1: {:?}, \ntx_2: {:?}",
        &tx_1_bytes[policies_bytes_range], &tx_2_bytes[policies_bytes_range_v2]
    );
    let input_bytes_range = input_offset..output_offset;
    let input_bytes_range_v2 = input_offset_v2..output_offset_v2;
    println!(
        "input bytes range v1 len: {}, input bytes range v2 len: {}",
        input_bytes_range.len(),
        input_bytes_range_v2.len()
    );
    println!(
        "input \ntx_1: {:?}, \ntx_2: {:?}",
        &tx_1_bytes[input_bytes_range], &tx_2_bytes[input_bytes_range_v2]
    );
    let output_bytes_range = output_offset..witness_offset;
    let output_bytes_range_v2 = output_offset_v2..witness_offset_v2;
    println!(
        "output bytes range v1 len: {}, output bytes range v2 len: {}",
        output_bytes_range.len(),
        output_bytes_range_v2.len()
    );
    println!(
        "output \ntx_1: {:?}, \ntx_2: {:?}",
        &tx_1_bytes[output_bytes_range.clone()],
        &tx_2_bytes[output_bytes_range_v2.clone()]
    );
    let witness_bytes_range = witness_offset..tx_1_bytes.len();
    let witness_bytes_range_v2 = witness_offset_v2..tx_2_bytes.len();
    println!(
        "witness bytes range v1 len: {}, witness bytes range v2 len: {}",
        witness_bytes_range.len(),
        witness_bytes_range_v2.len()
    );
    println!(
        "witness \ntx_1: {:?}, \ntx_2: {:?}",
        &tx_1_bytes[witness_bytes_range], &tx_2_bytes[witness_bytes_range_v2]
    );

    let static_witness_1_offset = tx_2.static_witnesses_offset_at(0).unwrap();
    let static_witness_2_offset = tx_2.static_witnesses_offset_at(1).unwrap();
    let static_witness_1_bytes_range = static_witness_1_offset..static_witness_2_offset;
    let static_witness_2_bytes_range = static_witness_2_offset..tx_2_bytes.len();
    println!(
        "static witness bytes range v1 len: {}, static witness bytes range v2 len: {}",
        static_witness_1_bytes_range.len(),
        static_witness_2_bytes_range.len()
    );
    println!(
        "static witness 1: {:?}",
        &tx_2_bytes[static_witness_1_bytes_range]
    );
    println!(
        "static witness 2: {:?}",
        &tx_2_bytes[static_witness_2_bytes_range]
    );
}

#[tokio::test]
async fn get_verifying_predicate() {
    let indices = vec![0, 4, 5, 7, 11];

    for idx in indices {
        #[rustfmt::skip]
        let predicate = vec![
            op::gm_args(0x10, GMArgs::GetVerifyingPredicate),
            op::movi(0x11, idx),
            op::eq(0x10, 0x10, 0x11),
            op::ret(0x10),
        ];

        assert!(execute_predicate(predicate, vec![], idx as usize).await);
    }
}

/// Returns the amount of gas used if verification succeeds
async fn execute_gas_metered_predicates(
    predicates: Vec<Vec<Instruction>>,
) -> Result<u64, ()> {
    const GAS_LIMIT: Word = 100_000;
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let arb_max_fee = 2_000;
    let script = vec![];
    let script_data = vec![];

    let mut builder = TransactionBuilder::script(script, script_data);
    builder
        .max_fee_limit(arb_max_fee)
        .maturity(Default::default());

    let coin_amount = 10_000_000;
    let params = CheckPredicateParams {
        max_gas_per_predicate: GAS_LIMIT,
        ..Default::default()
    };

    if predicates.is_empty() {
        builder.add_unsigned_coin_input(
            SecretKey::random(rng),
            rng.r#gen(),
            arb_max_fee,
            params.base_asset_id,
            rng.r#gen(),
        );
    }

    for predicate in predicates.iter() {
        let predicate: Vec<u8> = predicate
            .clone()
            .into_iter()
            .flat_map(|op| u32::from(op).to_be_bytes())
            .collect();

        let owner = Input::predicate_owner(&predicate);
        let input = Input::coin_predicate(
            rng.r#gen(),
            owner,
            coin_amount,
            AssetId::default(),
            rng.r#gen(),
            0,
            predicate,
            vec![],
        );

        builder.add_input(input);
    }

    let mut transaction = builder.finalize();

    let params = CheckPredicateParams {
        max_gas_per_predicate: GAS_LIMIT,
        ..Default::default()
    };

    // parallel version
    let parallel_gas_used = {
        let mut async_tx = transaction.clone();
        async_tx
            .estimate_predicates_async::<TokioWithRayon>(
                &params,
                &DummyPool,
                &EmptyStorage,
            )
            .await
            .map_err(|_| ())?;

        let tx = async_tx
            .into_checked_basic(Default::default(), &ConsensusParameters::standard())
            .expect("Should successfully create checked tranaction with predicate");

        check_predicates_async::<_, TokioWithRayon>(
            &tx,
            &params,
            &DummyPool,
            &EmptyStorage,
        )
        .await
        .map(|r| r.gas_used())
        .map_err(|_| ())?
    };

    // sequential version
    transaction
        .estimate_predicates(&params, MemoryInstance::new(), &EmptyStorage)
        .map_err(|_| ())?;

    let tx = transaction
        .into_checked_basic(Default::default(), &ConsensusParameters::standard())
        .expect("Should successfully create checked tranaction with predicate");

    let seq_gas_used =
        check_predicates(&tx, &params, MemoryInstance::new(), &EmptyStorage)
            .map(|r| r.gas_used())
            .map_err(|_| ())?;

    assert_eq!(seq_gas_used, parallel_gas_used);

    Ok(seq_gas_used)
}

#[tokio::test]
async fn predicate_gas_metering() {
    // This just succeeds
    assert!(
        execute_gas_metered_predicates(vec![vec![op::ret(RegId::ONE)]])
            .await
            .is_ok()
    );

    // This runs out of gas
    assert!(
        execute_gas_metered_predicates(vec![vec![
            op::ji(0), // Infinite loop
        ]])
        .await
        .is_err()
    );

    // Multiple Predicate Success
    assert!(
        execute_gas_metered_predicates(vec![
            vec![op::ret(RegId::ONE)],
            vec![
                op::movi(0x10, 0x11),
                op::movi(0x10, 0x11),
                op::ret(RegId::ONE),
            ],
        ])
        .await
        .is_ok()
    );

    // Running predicate gas used is combined properly
    let exe = (0..4).map(|n| async move {
        execute_gas_metered_predicates(vec![vec![op::ret(RegId::ONE)]; n])
            .await
            .unwrap()
    });

    let gas_used_by: Vec<_> = futures::future::join_all(exe).await.into_iter().collect();

    assert_eq!(gas_used_by[0], 0);
    assert_ne!(gas_used_by[1], 0);
    assert_eq!(gas_used_by[1] * 2, gas_used_by[2]);
    assert_eq!(gas_used_by[1] * 3, gas_used_by[3]);
}

#[tokio::test]
async fn gas_used_by_predicates_not_causes_out_of_gas_during_script() {
    let rng = &mut StdRng::seed_from_u64(2322u64);
    let params = CheckPredicateParams::default();

    let gas_limit = 1_000_000;
    let script = vec![
        op::addi(0x20, 0x20, 1),
        op::addi(0x20, 0x20, 1),
        op::ret(RegId::ONE),
    ]
    .into_iter()
    .collect::<Vec<u8>>();
    let script_data = vec![];

    let mut builder = TransactionBuilder::script(script, script_data);
    builder
        .script_gas_limit(gas_limit)
        .maturity(Default::default());

    let coin_amount = 10_000_000;

    builder.add_unsigned_coin_input(
        SecretKey::random(rng),
        rng.r#gen(),
        coin_amount,
        AssetId::default(),
        rng.r#gen(),
    );

    // parallel version
    {
        let _ = builder
            .clone()
            .finalize_checked_basic(Default::default())
            .check_predicates_async::<TokioWithRayon>(&params, &DummyPool, &EmptyStorage)
            .await
            .expect("Predicate check failed even if we don't have any predicates");
    }

    let tx_without_predicate = builder
        .finalize_checked_basic(Default::default())
        .check_predicates(&params, MemoryInstance::new(), &EmptyStorage)
        .expect("Predicate check failed even if we don't have any predicates");

    let mut client = MemoryClient::default();

    client.transact(tx_without_predicate);
    let receipts_without_predicate =
        client.receipts().expect("Expected receipts").to_vec();
    let gas_without_predicate = receipts_without_predicate[1]
        .gas_used()
        .expect("Should retrieve gas used");

    builder.script_gas_limit(gas_without_predicate);

    let predicate: Vec<u8> = vec![op::addi(0x20, 0x20, 1), op::ret(RegId::ONE)]
        .into_iter()
        .flat_map(|op| u32::from(op).to_be_bytes())
        .collect();
    let owner = Input::predicate_owner(&predicate);
    let input = Input::coin_predicate(
        rng.r#gen(),
        owner,
        coin_amount,
        AssetId::default(),
        rng.r#gen(),
        0,
        predicate,
        vec![],
    );

    builder.add_input(input);

    let mut transaction = builder.finalize();
    transaction
        .estimate_predicates(&params, MemoryInstance::new(), &EmptyStorage)
        .expect("Predicate estimation failed");

    let checked = transaction
        .into_checked_basic(Default::default(), &ConsensusParameters::standard())
        .expect("Should successfully create checked tranaction with predicate");

    // parallel version
    {
        let tx_with_predicate = checked
            .clone()
            .check_predicates_async::<TokioWithRayon>(&params, &DummyPool, &EmptyStorage)
            .await
            .expect("Predicate check failed");

        client.transact(tx_with_predicate);
        let receipts_with_predicate =
            client.receipts().expect("Expected receipts").to_vec();

        // No panic for transaction without gas limit
        assert_eq!(receipts_without_predicate[1].reason(), None);
        // Don't panic with out of gas for transaction with predicate
        assert_eq!(
            receipts_with_predicate[1].result().unwrap(),
            &ScriptExecutionResult::Success
        );
    }

    let tx_with_predicate = checked
        .check_predicates(&params, MemoryInstance::new(), &EmptyStorage)
        .expect("Predicate check failed");

    client.transact(tx_with_predicate);
    let receipts_with_predicate = client.receipts().expect("Expected receipts").to_vec();

    // No panic for transaction without gas limit
    assert_eq!(receipts_without_predicate[1].reason(), None);
    // Don't panic with out of gas for transaction with predicate
    assert_eq!(
        receipts_with_predicate[1].result().unwrap(),
        &ScriptExecutionResult::Success
    );
}

#[tokio::test]
async fn gas_used_by_predicates_more_than_limit() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let params = CheckPredicateParams::default();

    let gas_limit = 1_000_000;
    let arb_max_fee = 1000;
    let script = vec![
        op::addi(0x20, 0x20, 1),
        op::addi(0x20, 0x20, 1),
        op::ret(RegId::ONE),
    ]
    .into_iter()
    .collect::<Vec<u8>>();
    let script_data = vec![];

    let mut builder = TransactionBuilder::script(script, script_data);
    builder
        .max_fee_limit(arb_max_fee)
        .script_gas_limit(gas_limit)
        .maturity(Default::default());

    let coin_amount = 10_000_000;

    builder.add_unsigned_coin_input(
        SecretKey::random(rng),
        rng.r#gen(),
        coin_amount,
        AssetId::default(),
        rng.r#gen(),
    );

    // parallel version
    {
        let _ = builder
            .clone()
            .finalize_checked_basic(Default::default())
            .check_predicates_async::<TokioWithRayon>(&params, &DummyPool, &EmptyStorage)
            .await
            .expect("Predicate check failed even if we don't have any predicates");
    }

    let tx_without_predicate = builder
        .finalize_checked_basic(Default::default())
        .check_predicates(&params, MemoryInstance::new(), &EmptyStorage)
        .expect("Predicate check failed even if we don't have any predicates");

    let mut client = MemoryClient::default();

    client.transact(tx_without_predicate);
    let receipts_without_predicate =
        client.receipts().expect("Expected receipts").to_vec();
    let gas_without_predicate = receipts_without_predicate[1]
        .gas_used()
        .expect("Should retrieve gas used");

    builder.script_gas_limit(gas_without_predicate);

    let predicate: Vec<u8> = vec![
        op::addi(0x20, 0x20, 1),
        op::addi(0x20, 0x20, 1),
        op::addi(0x20, 0x20, 1),
        op::addi(0x20, 0x20, 1),
        op::ret(RegId::ONE),
    ]
    .into_iter()
    .flat_map(|op| u32::from(op).to_be_bytes())
    .collect();
    let owner = Input::predicate_owner(&predicate);
    let input = Input::coin_predicate(
        rng.r#gen(),
        owner,
        coin_amount,
        AssetId::default(),
        rng.r#gen(),
        gas_limit + 1,
        predicate,
        vec![],
    );

    builder.add_input(input);

    // parallel version
    {
        let tx_with_predicate = builder
            .clone()
            .finalize_checked_basic(Default::default())
            .check_predicates_async::<TokioWithRayon>(&params, &DummyPool, &EmptyStorage)
            .await;

        assert!(matches!(
            tx_with_predicate.unwrap_err(),
            CheckError::PredicateVerificationFailed(_)
        ));
    }

    let tx_with_predicate = builder
        .finalize_checked_basic(Default::default())
        .check_predicates(&params, MemoryInstance::new(), &EmptyStorage);

    assert!(matches!(
        tx_with_predicate.unwrap_err(),
        CheckError::PredicateVerificationFailed(_)
    ));
}

#[test]
#[ntest::timeout(5_000)]
fn synchronous_estimate_predicates_respects_total_tx_gas_limit() {
    let limit = 1_000_000;
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let params = CheckPredicateParams {
        max_gas_per_predicate: limit,
        max_gas_per_tx: limit,
        gas_costs: GasCosts::unit(),
        ..Default::default()
    };

    // Infinite loop
    let predicate = vec![op::noop(), op::jmpb(RegId::ZERO, 0)]
        .into_iter()
        .collect::<Vec<u8>>();
    let predicate_owner = Input::predicate_owner(&predicate);

    let mut builder = TransactionBuilder::script(vec![], vec![]);
    builder.max_fee_limit(1000).maturity(Default::default());

    let coin_amount = 100_000;

    for _ in 0..255 {
        builder.add_input(Input::coin_predicate(
            rng.r#gen(),
            predicate_owner,
            coin_amount,
            AssetId::default(),
            rng.r#gen(),
            0,
            predicate.clone(),
            vec![],
        ));
    }
    let mut transaction = builder.finalize();

    // When
    let result =
        transaction.estimate_predicates(&params, MemoryInstance::new(), &EmptyStorage);

    // Then
    assert_eq!(Ok(()), result);
}
