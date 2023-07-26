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
    prelude::*,
};

use crate::checked_transaction::{
    CheckPredicateParams,
    CheckPredicates,
    EstimatePredicates,
    ParallelExecutor,
};
use core::iter;
use fuel_asm::PanicReason::OutOfGas;
use fuel_tx::ConsensusParameters;
use fuel_types::ChainId;

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

    let owner = Input::predicate_owner(&predicate, &ChainId::default());
    let input = Input::coin_predicate(
        utxo_id,
        owner,
        amount,
        asset_id,
        tx_pointer,
        maturity,
        predicate_gas_used,
        predicate,
        predicate_data,
    );

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let script = vec![];
    let script_data = vec![];

    let mut builder = TransactionBuilder::script(script, script_data);

    builder
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .maturity(maturity);

    (0..dummy_inputs).for_each(|_| {
        builder.add_unsigned_coin_input(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            maturity,
        );
    });

    builder.add_input(input);

    let transaction = builder.finalize();
    let gas_costs = GasCosts::free();

    let checked = transaction
        .into_checked_basic(height, &ConsensusParameters::standard())
        .expect("Should successfully convert into Checked");

    let params = CheckPredicateParams {
        gas_costs,
        ..Default::default()
    };

    let parallel_execution = {
        Interpreter::<PredicateStorage>::check_predicates_async::<_, TokioWithRayon>(
            &checked, &params,
        )
        .await
        .map(|checked| checked.gas_used())
    };

    let seq_execution =
        Interpreter::<PredicateStorage>::check_predicates(&checked, &params)
            .map(|checked| checked.gas_used());

    match (parallel_execution, seq_execution) {
        (Ok(p_gas_used), Ok(s_gas_used)) => {
            assert_eq!(p_gas_used, s_gas_used);
            true
        }
        (Err(_), Err(_)) => false,
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
    let predicate = vec![
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

    let gas_price = 1_000;
    let gas_limit = 1_000_000;
    let script = vec![];
    let script_data = vec![];

    let mut builder = TransactionBuilder::script(script, script_data);
    builder
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .maturity(Default::default());

    let coin_amount = 10_000_000;

    if predicates.is_empty() {
        builder.add_unsigned_coin_input(
            rng.gen(),
            rng.gen(),
            coin_amount,
            AssetId::default(),
            rng.gen(),
            Default::default(),
        );
    }

    for predicate in predicates {
        let predicate: Vec<u8> = predicate
            .into_iter()
            .flat_map(|op| u32::from(op).to_be_bytes())
            .collect();

        let owner = Input::predicate_owner(&predicate, &ChainId::default());
        let input = Input::coin_predicate(
            rng.gen(),
            owner,
            coin_amount,
            AssetId::default(),
            rng.gen(),
            Default::default(),
            GAS_LIMIT,
            predicate,
            vec![],
        );

        builder.add_input(input);
    }

    let mut transaction = builder.finalize();

    let params = CheckPredicateParams {
        max_gas_per_tx: GAS_LIMIT,
        max_gas_per_predicate: GAS_LIMIT,
        ..Default::default()
    };

    // parallel version
    let parallel_gas_used = {
        let mut async_tx = transaction.clone();
        async_tx
            .estimate_predicates_async::<TokioWithRayon>(&params)
            .await
            .map_err(|_| ())?;

        let tx = async_tx
            .into_checked_basic(Default::default(), &ConsensusParameters::standard())
            .expect("Should successfully create checked tranaction with predicate");

        Interpreter::<PredicateStorage>::check_predicates_async::<_, TokioWithRayon>(
            &tx, &params,
        )
        .await
        .map(|r| r.gas_used())
        .map_err(|_| ())?
    };

    // sequential version
    transaction.estimate_predicates(&params).map_err(|_| ())?;

    let tx = transaction
        .into_checked_basic(Default::default(), &ConsensusParameters::standard())
        .expect("Should successfully create checked tranaction with predicate");

    let seq_gas_used = Interpreter::<PredicateStorage>::check_predicates(&tx, &params)
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
            op::ret(RegId::ONE)
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
async fn gas_used_by_predicates_is_deducted_from_script_gas() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let gas_price = 1_000;
    let gas_limit = 1_000_000;
    let script = vec![op::ret(RegId::ONE)].into_iter().collect::<Vec<u8>>();
    let script_data = vec![];
    let params = CheckPredicateParams::default();

    let mut builder = TransactionBuilder::script(script, script_data);
    builder
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .maturity(Default::default());

    let coin_amount = 10_000_000;

    builder.add_unsigned_coin_input(
        rng.gen(),
        rng.gen(),
        coin_amount,
        AssetId::default(),
        rng.gen(),
        Default::default(),
    );

    // parallel version
    let p_tx_without_predicate = {
        builder
            .clone()
            .finalize_checked_basic(Default::default())
            .check_predicates_async::<TokioWithRayon>(&params)
            .await
            .expect("Predicate check failed even if we don't have any predicates")
    };

    let tx_without_predicate = builder
        .finalize_checked_basic(Default::default())
        .check_predicates(&params)
        .expect("Predicate check failed even if we don't have any predicates");

    let predicate: Vec<u8> = vec![
        op::addi(0x20, 0x20, 1),
        op::addi(0x20, 0x20, 1),
        op::addi(0x20, 0x20, 1),
        op::addi(0x20, 0x20, 1),
        op::addi(0x20, 0x20, 1),
        op::ret(RegId::ONE),
    ]
    .into_iter()
    .flat_map(|op| u32::from(op).to_be_bytes())
    .collect();
    let owner = Input::predicate_owner(&predicate, &params.chain_id);
    let input = Input::coin_predicate(
        rng.gen(),
        owner,
        coin_amount,
        AssetId::default(),
        rng.gen(),
        Default::default(),
        rng.gen(),
        predicate,
        vec![],
    );

    // add non-predicate input before and after predicate input
    // to check that predicate verification only handles predicate inputs
    builder.add_unsigned_coin_input(
        rng.gen(),
        rng.gen(),
        rng.gen(),
        AssetId::default(),
        rng.gen(),
        Default::default(),
    );

    builder.add_input(input);

    builder.add_unsigned_coin_input(
        rng.gen(),
        rng.gen(),
        rng.gen(),
        AssetId::default(),
        rng.gen(),
        Default::default(),
    );

    let mut transaction = builder.finalize();
    transaction
        .estimate_predicates(&params)
        .expect("Predicate estimation failed");

    let checked = transaction
        .into_checked_basic(Default::default(), &ConsensusParameters::standard())
        .expect("Should successfully create checked tranaction with predicate");

    // parallel version
    let (p_with_predicate, p_without_predicate) = {
        let tx_with_predicate = checked
            .clone()
            .check_predicates_async::<TokioWithRayon>(&params)
            .await
            .expect("Predicate check failed");

        let mut client = MemoryClient::default();
        client.transact(tx_with_predicate);
        let receipts_with_predicate =
            client.receipts().expect("Expected receipts").to_vec();
        client.transact(p_tx_without_predicate);
        let receipts_without_predicate =
            client.receipts().expect("Expected receipts").to_vec();

        assert!(
            receipts_with_predicate[1].gas_used()
                > receipts_without_predicate[1].gas_used()
        );

        (
            receipts_with_predicate[1].gas_used(),
            receipts_without_predicate[1].gas_used(),
        )
    };

    let tx_with_predicate = checked
        .check_predicates(&params)
        .expect("Predicate check failed");

    let mut client = MemoryClient::default();
    client.transact(tx_with_predicate);
    let receipts_with_predicate = client.receipts().expect("Expected receipts").to_vec();
    client.transact(tx_without_predicate);
    let receipts_without_predicate =
        client.receipts().expect("Expected receipts").to_vec();

    assert!(
        receipts_with_predicate[1].gas_used() > receipts_without_predicate[1].gas_used()
    );

    assert_eq!(p_with_predicate, receipts_with_predicate[1].gas_used());
    assert_eq!(
        p_without_predicate,
        receipts_without_predicate[1].gas_used()
    );
}

#[tokio::test]
async fn gas_used_by_predicates_causes_out_of_gas_during_script() {
    let rng = &mut StdRng::seed_from_u64(2322u64);
    let params = CheckPredicateParams::default();

    let gas_price = 1_000;
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
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .maturity(Default::default());

    let coin_amount = 10_000_000;

    builder.add_unsigned_coin_input(
        rng.gen(),
        rng.gen(),
        coin_amount,
        AssetId::default(),
        rng.gen(),
        Default::default(),
    );

    // parallel version
    {
        let _ = builder
            .clone()
            .finalize_checked_basic(Default::default())
            .check_predicates_async::<TokioWithRayon>(&params)
            .await
            .expect("Predicate check failed even if we don't have any predicates");
    }

    let tx_without_predicate = builder
        .finalize_checked_basic(Default::default())
        .check_predicates(&params)
        .expect("Predicate check failed even if we don't have any predicates");

    let mut client = MemoryClient::default();

    client.transact(tx_without_predicate);
    let receipts_without_predicate =
        client.receipts().expect("Expected receipts").to_vec();
    let gas_without_predicate = receipts_without_predicate[1]
        .gas_used()
        .expect("Should retrieve gas used");

    builder.gas_limit(gas_without_predicate);

    let predicate: Vec<u8> = vec![op::addi(0x20, 0x20, 1), op::ret(RegId::ONE)]
        .into_iter()
        .flat_map(|op| u32::from(op).to_be_bytes())
        .collect();
    let owner = Input::predicate_owner(&predicate, &params.chain_id);
    let input = Input::coin_predicate(
        rng.gen(),
        owner,
        coin_amount,
        AssetId::default(),
        rng.gen(),
        Default::default(),
        rng.gen(),
        predicate,
        vec![],
    );

    builder.add_input(input);

    let mut transaction = builder.finalize();
    transaction
        .estimate_predicates(&params)
        .expect("Predicate estimation failed");

    let checked = transaction
        .into_checked_basic(Default::default(), &ConsensusParameters::standard())
        .expect("Should successfully create checked tranaction with predicate");

    // parallel version
    {
        let tx_with_predicate = checked
            .clone()
            .check_predicates_async::<TokioWithRayon>(&params)
            .await
            .expect("Predicate check failed");

        client.transact(tx_with_predicate);
        let receipts_with_predicate =
            client.receipts().expect("Expected receipts").to_vec();

        // No panic for transaction without gas limit
        assert_eq!(receipts_without_predicate[1].reason(), None);
        // Panic with out of gas for transaction with predicate
        assert_eq!(
            receipts_with_predicate[0].reason().unwrap().reason(),
            &OutOfGas
        );
    }

    let tx_with_predicate = checked
        .check_predicates(&params)
        .expect("Predicate check failed");

    client.transact(tx_with_predicate);
    let receipts_with_predicate = client.receipts().expect("Expected receipts").to_vec();

    // No panic for transaction without gas limit
    assert_eq!(receipts_without_predicate[1].reason(), None);
    // Panic with out of gas for transaction with predicate
    assert_eq!(
        receipts_with_predicate[0].reason().unwrap().reason(),
        &OutOfGas
    );
}

#[tokio::test]
async fn gas_used_by_predicates_more_than_limit() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let params = CheckPredicateParams::default();

    let gas_price = 1_000;
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
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .maturity(Default::default());

    let coin_amount = 10_000_000;

    builder.add_unsigned_coin_input(
        rng.gen(),
        rng.gen(),
        coin_amount,
        AssetId::default(),
        rng.gen(),
        Default::default(),
    );

    // parallel version
    {
        let _ = builder
            .clone()
            .finalize_checked_basic(Default::default())
            .check_predicates_async::<TokioWithRayon>(&params)
            .await
            .expect("Predicate check failed even if we don't have any predicates");
    }

    let tx_without_predicate = builder
        .finalize_checked_basic(Default::default())
        .check_predicates(&params)
        .expect("Predicate check failed even if we don't have any predicates");

    let mut client = MemoryClient::default();

    client.transact(tx_without_predicate);
    let receipts_without_predicate =
        client.receipts().expect("Expected receipts").to_vec();
    let gas_without_predicate = receipts_without_predicate[1]
        .gas_used()
        .expect("Should retrieve gas used");

    builder.gas_limit(gas_without_predicate);

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
    let owner = Input::predicate_owner(&predicate, &params.chain_id);
    let input = Input::coin_predicate(
        rng.gen(),
        owner,
        coin_amount,
        AssetId::default(),
        rng.gen(),
        Default::default(),
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
            .check_predicates_async::<TokioWithRayon>(&params)
            .await;

        assert_eq!(
            tx_with_predicate.unwrap_err(),
            CheckError::PredicateVerificationFailed
        );
    }

    let tx_with_predicate = builder
        .finalize_checked_basic(Default::default())
        .check_predicates(&params);

    assert_eq!(
        tx_with_predicate.unwrap_err(),
        CheckError::PredicateVerificationFailed
    );
}
