#![cfg(feature = "std")]

use fuel_asm::{
    op,
    GMArgs,
    GTFArgs,
    Instruction,
    RegId,
};
use fuel_tx::TransactionBuilder;
use rand::{
    rngs::StdRng,
    Rng,
    SeedableRng,
};
use tokio_rayon::AsyncRayonHandle;

use crate::{
    error::PredicateVerificationFailed,
    pool::DummyPool,
    prelude::*,
};

use crate::checked_transaction::{
    CheckError,
    CheckPredicateParams,
    CheckPredicates,
    EstimatePredicates,
    ParallelExecutor,
};
use core::iter;
use fuel_tx::ConsensusParameters;

pub struct TokioWithRayon;

#[async_trait::async_trait]
impl ParallelExecutor for TokioWithRayon {
    type Task = AsyncRayonHandle<Result<(Word, usize), PredicateVerificationFailed>>;

    fn create_task<F>(func: F) -> Self::Task
    where
        F: FnOnce() -> Result<(Word, usize), PredicateVerificationFailed>
            + Send
            + 'static,
    {
        tokio_rayon::spawn(func)
    }

    async fn execute_tasks(
        futures: Vec<Self::Task>,
    ) -> Vec<Result<(Word, usize), PredicateVerificationFailed>> {
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

    let utxo_id = rng.gen();
    let amount = 0;
    let asset_id = rng.gen();
    let tx_pointer = rng.gen();
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
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
        );
    });

    builder.add_input(input);

    let mut transaction = builder.finalize();
    transaction
        .estimate_predicates(&check_params, MemoryInstance::new())
        .expect("Should estimate predicate");

    let checked = transaction
        .into_checked_basic(height, &params)
        .expect("Should successfully convert into Checked");

    let parallel_execution = {
        Interpreter::check_predicates_async::<TokioWithRayon>(
            &checked,
            &check_params,
            &DummyPool,
        )
        .await
        .map(|checked| checked.gas_used())
    };

    let seq_execution =
        Interpreter::check_predicates(&checked, &check_params, MemoryInstance::new())
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
    const GAS_LIMIT: Word = 10000;
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
            rng.gen(),
            arb_max_fee,
            params.base_asset_id,
            rng.gen(),
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

    let mut transaction = builder.finalize();

    let params = CheckPredicateParams {
        max_gas_per_predicate: GAS_LIMIT,
        ..Default::default()
    };

    // parallel version
    let parallel_gas_used = {
        let mut async_tx = transaction.clone();
        async_tx
            .estimate_predicates_async::<TokioWithRayon>(&params, &DummyPool)
            .await
            .map_err(|_| ())?;

        let tx = async_tx
            .into_checked_basic(Default::default(), &ConsensusParameters::standard())
            .expect("Should successfully create checked tranaction with predicate");

        Interpreter::check_predicates_async::<TokioWithRayon>(&tx, &params, &DummyPool)
            .await
            .map(|r| r.gas_used())
            .map_err(|_| ())?
    };

    // sequential version
    transaction
        .estimate_predicates(&params, MemoryInstance::new())
        .map_err(|_| ())?;

    let tx = transaction
        .into_checked_basic(Default::default(), &ConsensusParameters::standard())
        .expect("Should successfully create checked tranaction with predicate");

    let seq_gas_used = Interpreter::check_predicates(&tx, &params, MemoryInstance::new())
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
    assert!(execute_gas_metered_predicates(vec![vec![
        op::ji(0), // Infinite loop
    ]])
    .await
    .is_err());

    // Multiple Predicate Success
    assert!(execute_gas_metered_predicates(vec![
        vec![op::ret(RegId::ONE)],
        vec![
            op::movi(0x10, 0x11),
            op::movi(0x10, 0x11),
            op::ret(RegId::ONE),
        ],
    ])
    .await
    .is_ok());

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
        rng.gen(),
        coin_amount,
        AssetId::default(),
        rng.gen(),
    );

    // parallel version
    {
        let _ = builder
            .clone()
            .finalize_checked_basic(Default::default())
            .check_predicates_async::<TokioWithRayon>(&params, &DummyPool)
            .await
            .expect("Predicate check failed even if we don't have any predicates");
    }

    let tx_without_predicate = builder
        .finalize_checked_basic(Default::default())
        .check_predicates(&params, MemoryInstance::new())
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
        rng.gen(),
        owner,
        coin_amount,
        AssetId::default(),
        rng.gen(),
        rng.gen(),
        predicate,
        vec![],
    );

    builder.add_input(input);

    let mut transaction = builder.finalize();
    transaction
        .estimate_predicates(&params, MemoryInstance::new())
        .expect("Predicate estimation failed");

    let checked = transaction
        .into_checked_basic(Default::default(), &ConsensusParameters::standard())
        .expect("Should successfully create checked tranaction with predicate");

    // parallel version
    {
        let tx_with_predicate = checked
            .clone()
            .check_predicates_async::<TokioWithRayon>(&params, &DummyPool)
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
        .check_predicates(&params, MemoryInstance::new())
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
        rng.gen(),
        coin_amount,
        AssetId::default(),
        rng.gen(),
    );

    // parallel version
    {
        let _ = builder
            .clone()
            .finalize_checked_basic(Default::default())
            .check_predicates_async::<TokioWithRayon>(&params, &DummyPool)
            .await
            .expect("Predicate check failed even if we don't have any predicates");
    }

    let tx_without_predicate = builder
        .finalize_checked_basic(Default::default())
        .check_predicates(&params, MemoryInstance::new())
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
        rng.gen(),
        owner,
        coin_amount,
        AssetId::default(),
        rng.gen(),
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
            .check_predicates_async::<TokioWithRayon>(&params, &DummyPool)
            .await;

        assert!(matches!(
            tx_with_predicate.unwrap_err(),
            CheckError::PredicateVerificationFailed(_)
        ));
    }

    let tx_with_predicate = builder
        .finalize_checked_basic(Default::default())
        .check_predicates(&params, MemoryInstance::new());

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

    let mut params = CheckPredicateParams::default();
    params.max_gas_per_predicate = limit;
    params.max_gas_per_tx = limit;
    params.gas_costs = GasCosts::unit();

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
            rng.gen(),
            predicate_owner,
            coin_amount,
            AssetId::default(),
            rng.gen(),
            0,
            predicate.clone(),
            vec![],
        ));
    }
    let mut transaction = builder.finalize();

    // When
    let result = transaction.estimate_predicates(&params, MemoryInstance::new());

    // Then
    assert_eq!(Ok(()), result);
}
