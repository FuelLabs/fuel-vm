#![cfg(feature = "std")]
#![allow(non_snake_case)]

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

use fuel_types::canonical::Serialize;

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
    consensus_parameters::gas::GasCostsValuesV5,
    field::Inputs,
    ConsensusParameters,
};

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
    execute_predicate_with_input(dummy_inputs, input, rng).await
}

pub struct DataCoinInputBuilder {
    pub utxo_id: UtxoId,
    pub owner: Address,
    pub amount: Word,
    pub asset_id: AssetId,
    pub tx_pointer: TxPointer,
    pub predicate_gas_used: Word,
    pub predicate: Vec<u8>,
    pub predicate_data: Vec<u8>,
    pub data: Vec<u8>,
}

impl DataCoinInputBuilder {
    pub fn new() -> Self {
        let rng = &mut StdRng::seed_from_u64(2322u64);
        let _predicate = vec![op::ret(0x01)];
        let predicate = _predicate.into_iter().collect::<Vec<u8>>();

        let utxo_id = rng.gen();
        let amount = 0;
        let asset_id = rng.gen();
        let tx_pointer = rng.gen();
        let predicate_gas_used = 0;
        let predicate_data = vec![];
        let data = vec![];

        let owner = Input::predicate_owner(&predicate);
        Self {
            utxo_id,
            owner,
            amount,
            asset_id,
            tx_pointer,
            predicate_gas_used,
            predicate,
            predicate_data,
            data,
        }
    }

    pub fn with_predicate(&mut self, predicate: Vec<Instruction>) -> &Self {
        self.predicate = predicate.into_iter().collect();
        self
    }

    pub fn with_predicate_data(&mut self, predicate_data: &[u8]) -> &Self {
        self.predicate_data = predicate_data.to_vec();
        self
    }

    pub fn into_input(self) -> Input {
        Input::data_coin_predicate(
            self.utxo_id,
            self.owner,
            self.amount,
            self.asset_id,
            self.tx_pointer,
            self.predicate_gas_used,
            self.predicate,
            self.predicate_data,
            self.data,
        )
    }
}

async fn execute_data_coin_predicate_input<P>(
    predicate: P,
    predicate_data: Vec<u8>,
    dummy_inputs: usize,
    data: Vec<u8>,
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
    let predicate_gas_used = 0;

    let owner = Input::predicate_owner(&predicate);
    let input = Input::data_coin_predicate(
        utxo_id,
        owner,
        amount,
        asset_id,
        tx_pointer,
        predicate_gas_used,
        predicate,
        predicate_data,
        data,
    );
    execute_predicate_with_input(dummy_inputs, input, rng).await
}

async fn execute_data_coin_predicate_input_and_output<P>(
    predicate: P,
    predicate_data: Vec<u8>,
    input_data: Vec<u8>,
    output_data: Vec<u8>,
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
    let predicate_gas_used = 0;

    let owner = Input::predicate_owner(&predicate);
    let input = Input::data_coin_predicate(
        utxo_id,
        owner,
        amount,
        asset_id,
        tx_pointer,
        predicate_gas_used,
        predicate,
        predicate_data,
        input_data,
    );

    let different_owner = rng.gen();

    let output = Output::data_coin(different_owner, amount, asset_id, output_data);

    execute_predicate_with_input_and_output(vec![input], vec![output]).await
}

async fn execute_predicate_with_input(
    dummy_inputs: usize,
    input: Input,
    rng: &mut StdRng,
) -> bool {
    let maturity = Default::default();
    let height = Default::default();
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
            dbg!(&p_err);
            assert_eq!(p_err, s_err);
            false
        }
        _ => panic!("Parallel and sequential execution should return the same result"),
    }
}

async fn execute_predicate_with_input_and_output(
    inputs: Vec<Input>,
    outputs: Vec<Output>,
) -> bool {
    let maturity = Default::default();
    let height = Default::default();
    let gas_limit = 1_000_000;
    let script = vec![];
    let script_data = vec![];

    let mut builder = TransactionBuilder::script(script, script_data);
    let params = ConsensusParameters::standard();
    let check_params = params.clone().into();

    builder.script_gas_limit(gas_limit).maturity(maturity);

    for input in inputs {
        builder.add_input(input);
    }
    for output in outputs {
        builder.add_output(output);
    }

    let mut transaction = builder.finalize();
    transaction
        .estimate_predicates(&check_params, MemoryInstance::new(), &EmptyStorage)
        .expect("Should estimate predicate");

    let checked = transaction
        .into_checked_basic(height, &params)
        .expect("Should successfully convert into Checked");

    let res = check_predicates(
        &checked,
        &check_params,
        MemoryInstance::new(),
        &EmptyStorage,
    )
    .map(|checked| checked.gas_used());
    res.is_ok()
}

#[test]
fn estimate_predicate_works_when_predicate_address_incorrect() {
    let mut rng = StdRng::seed_from_u64(2322u64);
    let predicate: Vec<u8> = iter::once(op::ret(0x01)).collect();
    let predicate_data = vec![];

    let mut builder = TransactionBuilder::script(vec![], vec![]);

    // Given
    let predicate_owner = rng.gen();
    let input = Input::coin_predicate(
        rng.gen(),
        predicate_owner,
        rng.gen(),
        rng.gen(),
        rng.gen(),
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
            rng.gen(),
            predicate_owner,
            rng.gen(),
            rng.gen(),
            rng.gen(),
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
    let expected = 0x23 as Word;
    let expected_data = expected.to_be_bytes().to_vec();

    let wrong_data = 0x24 as Word;
    let wrong_data = wrong_data.to_be_bytes().to_vec();

    // A script that will succeed only if the argument is 0x23
    let predicate = [
        // set expected value
        op::movi(0x11, expected as u32),
        // allocate 8 bytes to memory
        op::movi(0x12, 0x08),
        op::aloc(0x12),
        op::move_(0x12, RegId::HP),
        op::sw(0x12, 0x11, 0),
        // set length of the data
        op::movi(0x10, 0x08),
        // get the data from the input
        op::gtf_args(0x11, 0, GTFArgs::InputCoinPredicateData),
        // compare expected value with the gotten data
        op::meq(0x10, 0x11, 0x12, 0x10),
        // return result
        op::ret(0x10),
    ];

    assert!(execute_predicate(predicate.iter().copied(), expected_data, 0).await);
    assert!(!execute_predicate(predicate.iter().copied(), wrong_data, 0).await);
}

#[tokio::test]
async fn gtf_args__input_data_coin_utxo_id() {
    let mut data_coin_builder = DataCoinInputBuilder::new();
    let utxo_id_as_bytes = Serialize::to_bytes(&data_coin_builder.utxo_id);
    data_coin_builder.with_predicate_data(&utxo_id_as_bytes);

    let expected_utxo_id_reg = 0x11;
    let actual_utxo_id_reg = 0x12;
    let utxo_id_size = 8;
    let utxo_id_size_reg = 0x13;
    let res_reg = 0x10;
    let predicate = vec![
        op::movi(utxo_id_size_reg, utxo_id_size),
        op::gtf_args(actual_utxo_id_reg, 0, GTFArgs::InputCoinTxId),
        op::gtf_args(expected_utxo_id_reg, 0, GTFArgs::InputCoinPredicateData),
        op::meq(
            res_reg,
            expected_utxo_id_reg,
            actual_utxo_id_reg,
            utxo_id_size_reg,
        ),
        op::ret(res_reg),
    ];
    data_coin_builder.with_predicate(predicate);
    let data_coin_input = data_coin_builder.into_input();
    assert!(
        execute_predicate_with_input(
            0,
            data_coin_input,
            &mut StdRng::seed_from_u64(2322u64)
        )
        .await
    );
}

#[tokio::test]
async fn gtf_args__input_data_coin_address() {
    let mut data_coin_builder = DataCoinInputBuilder::new();
    let expected_address = data_coin_builder.owner;
    let expected_address_bytes = expected_address.to_bytes();
    data_coin_builder.with_predicate_data(&expected_address_bytes);

    let expected_address_reg = 0x11;
    let actual_address_reg = 0x12;
    let address_size = 20;
    let address_size_reg = 0x13;
    let res_reg = 0x10;
    let predicate = vec![
        op::movi(address_size_reg, address_size),
        op::gtf_args(actual_address_reg, 0, GTFArgs::InputCoinOwner),
        op::gtf_args(expected_address_reg, 0, GTFArgs::InputCoinPredicateData),
        op::meq(
            res_reg,
            expected_address_reg,
            actual_address_reg,
            address_size_reg,
        ),
        op::ret(res_reg),
    ];
    data_coin_builder.with_predicate(predicate);
    let data_coin_input = data_coin_builder.into_input();
    assert!(
        execute_predicate_with_input(
            0,
            data_coin_input,
            &mut StdRng::seed_from_u64(2322u64)
        )
        .await
    );
}

#[tokio::test]
async fn gtf_args__input_data_coin_amount() {
    let mut data_coin_builder = DataCoinInputBuilder::new();
    let amount = data_coin_builder.amount;

    let expected_amount_reg = 0x11;
    let actual_amount_reg = 0x12;
    let res_reg = 0x10;
    let predicate = vec![
        op::movi(expected_amount_reg, amount as u32),
        op::gtf_args(actual_amount_reg, 0, GTFArgs::InputCoinAmount),
        op::eq(res_reg, expected_amount_reg, actual_amount_reg),
        op::ret(res_reg),
    ];
    data_coin_builder.with_predicate(predicate);
    let data_coin_input = data_coin_builder.into_input();
    assert!(
        execute_predicate_with_input(
            0,
            data_coin_input,
            &mut StdRng::seed_from_u64(2322u64)
        )
        .await
    );
}

#[tokio::test]
async fn gtf_args__input_data_coin_asset_id() {
    let mut data_coin_builder = DataCoinInputBuilder::new();
    let asset_id = data_coin_builder.asset_id;
    let predicate_data = asset_id.to_bytes();
    let size = predicate_data.len();
    data_coin_builder.with_predicate_data(&predicate_data);

    let expected_asset_id_reg = 0x11;
    let actual_asset_id_reg = 0x12;
    let res_reg = 0x10;
    let size_reg = 0x13;
    let predicate = vec![
        op::movi(size_reg, size as u32),
        op::gtf_args(expected_asset_id_reg, 0, GTFArgs::InputCoinPredicateData),
        op::gtf_args(actual_asset_id_reg, 0, GTFArgs::InputCoinAssetId),
        op::meq(
            res_reg,
            expected_asset_id_reg,
            actual_asset_id_reg,
            size_reg,
        ),
        op::ret(res_reg),
    ];
    data_coin_builder.with_predicate(predicate);
    let data_coin_input = data_coin_builder.into_input();
    assert!(
        execute_predicate_with_input(
            0,
            data_coin_input,
            &mut StdRng::seed_from_u64(2322u64)
        )
        .await
    );
}

#[tokio::test]
async fn gtf_args__input_data_coin_tx_pointer_is_always_default() {
    let mut data_coin_builder = DataCoinInputBuilder::new();
    let default_tx_pointer = TxPointer::default();
    let predicate_data = Serialize::to_bytes(&default_tx_pointer);
    let len = predicate_data.len() as u32;
    data_coin_builder.with_predicate_data(&predicate_data);

    let expected_tx_pointer_reg = 0x11;
    let actual_tx_pointer_reg = 0x12;
    let res_reg = 0x10;
    let size_reg = 0x13;
    let predicate = vec![
        op::movi(size_reg, len),
        op::gtf_args(expected_tx_pointer_reg, 0, GTFArgs::InputCoinPredicateData),
        op::gtf_args(actual_tx_pointer_reg, 0, GTFArgs::InputCoinTxPointer),
        op::meq(
            res_reg,
            expected_tx_pointer_reg,
            actual_tx_pointer_reg,
            size_reg,
        ),
        op::ret(res_reg),
    ];
    data_coin_builder.with_predicate(predicate);
    let data_coin_input = data_coin_builder.into_input();
    assert!(
        execute_predicate_with_input(
            0,
            data_coin_input,
            &mut StdRng::seed_from_u64(2322u64)
        )
        .await
    );
}

#[tokio::test]
async fn gtf_args__input_data_coin_predicate_gas_used() {
    let mut data_coin_builder = DataCoinInputBuilder::new();
    let predicate_gas_used = data_coin_builder.predicate_gas_used;

    let expected_predicate_gas_used_reg = 0x11;
    let actual_predicate_gas_used_reg = 0x12;
    let res_reg = 0x10;
    let size_reg = 0x13;
    let predicate = vec![
        op::movi(expected_predicate_gas_used_reg, predicate_gas_used as u32),
        op::gtf_args(
            actual_predicate_gas_used_reg,
            0,
            GTFArgs::InputCoinPredicateGasUsed,
        ),
        op::meq(
            res_reg,
            expected_predicate_gas_used_reg,
            actual_predicate_gas_used_reg,
            size_reg,
        ),
        op::ret(res_reg),
    ];
    data_coin_builder.with_predicate(predicate);
    let data_coin_input = data_coin_builder.into_input();
    assert!(
        execute_predicate_with_input(
            0,
            data_coin_input,
            &mut StdRng::seed_from_u64(2322u64)
        )
        .await
    );
}

#[tokio::test]
async fn gtf_args__input_data_coin_predicate_data() {
    let expected = 0x23 as Word;
    let expected_data = expected.to_be_bytes().to_vec();

    let data = vec![];

    // A script that will succeed only if the argument is 0x23
    let expected_data_reg = 0x11;
    let gtf_data_reg = 0x13;
    let res_reg = 0x10;
    let compare_bytes_len = 8;
    let input_index = 0;
    let predicate = [
        op::movi(expected_data_reg, expected as u32),
        op::gtf_args(gtf_data_reg, input_index, GTFArgs::InputCoinPredicateData),
        op::meq(res_reg, gtf_data_reg, expected_data_reg, compare_bytes_len),
        op::ret(res_reg),
    ];

    assert!(
        execute_data_coin_predicate_input(
            predicate.iter().copied(),
            expected_data,
            0,
            data
        )
        .await
    );
}

#[tokio::test]
async fn gtf_args__input_data_coin_data_length() {
    let predicate_data = vec![];

    let data = vec![1, 2, 3, 4, 5, 6, 7];
    let expected = data.len() as u32;

    // A script that will succeed only if the argument is 0x23
    let expected_len_reg = 0x11;
    let actual_len_reg = 0x13;
    let res_reg = 0x10;
    let compare_bytes_reg = 0x15;
    let compare_bytes = 8;
    let input_index = 0;
    let predicate = [
        op::movi(expected_len_reg, expected),
        op::movi(compare_bytes_reg, compare_bytes),
        op::gtf_args(
            actual_len_reg,
            input_index,
            GTFArgs::InputDataCoinDataLength,
        ),
        op::meq(res_reg, actual_len_reg, expected_len_reg, compare_bytes_reg),
        op::ret(res_reg),
    ];

    assert!(
        execute_data_coin_predicate_input(
            predicate.iter().copied(),
            predicate_data,
            0,
            data
        )
        .await
    );
}

#[tokio::test]
async fn gtf_args__input_data_coin_data() {
    let data = vec![5; 100];
    let predicate_data = data.clone();

    let len_reg = 0x13;
    let res_reg = 0x10;
    let input_index = 0;
    let expected_data_mem_location = 0x22;
    let actual_data_mem_location = 0x23;
    let predicate = [
        op::gtf_args(len_reg, input_index, GTFArgs::InputDataCoinDataLength),
        // get expected data from predicate data
        op::gtf_args(
            expected_data_mem_location,
            input_index,
            GTFArgs::InputCoinPredicateData,
        ),
        // get actual data
        op::gtf_args(
            actual_data_mem_location,
            input_index,
            GTFArgs::InputDataCoinData,
        ),
        // compare
        op::meq(
            res_reg,
            expected_data_mem_location,
            actual_data_mem_location,
            len_reg,
        ),
        op::ret(res_reg),
    ];

    assert!(
        execute_data_coin_predicate_input(
            predicate.iter().copied(),
            predicate_data,
            0,
            data
        )
        .await
    );
}

#[tokio::test]
async fn gtf_args__output_data_coin_data_len__matches_expected_value() {
    // given
    let predicate_data = vec![1, 2, 3, 4, 5, 6, 7];
    let data = vec![5; 100];

    // A script that will succeed only if the argument is the length of the data
    let expected_len = data.len() as u32;

    let expected_len_reg = 0x13;
    let actual_len_reg = 0x14;
    let res_reg = 0x10;
    let output_index = 0;
    let predicate = [
        op::movi(expected_len_reg, expected_len),
        op::gtf_args(
            actual_len_reg,
            output_index,
            GTFArgs::OutputDataCoinDataLength,
        ),
        op::eq(res_reg, actual_len_reg, expected_len_reg),
        op::ret(res_reg),
    ];

    // when
    let success = execute_data_coin_predicate_input_and_output(
        predicate.iter().copied(),
        predicate_data,
        data.clone(),
        data,
    )
    .await;

    // then
    assert!(success);
}

#[tokio::test]
async fn gtf_args__output_data_coin_data__succeeds_if_input_data_matches_output_data() {
    // given
    let data = vec![5; 100];
    let predicate_data = vec![1, 2, 3, 4, 5, 6, 7];

    // A script that will succeed only if the argument is the length of the data
    let expected_len = data.len() as u32;

    let len_reg = 0x13;
    let res_reg = 0x10;
    let output_index = 0;
    let input_index = 0;
    let output_data_reg = 0x14;
    let input_data_reg = 0x15;
    let predicate = vec![
        op::movi(len_reg, expected_len),
        // get output data
        op::gtf_args(output_data_reg, output_index, GTFArgs::OutputDataCoinData),
        // get other data
        // op::gtf_args(input_data_reg, input_index, GTFArgs::InputCoinPredicateData),
        op::gtf_args(input_data_reg, input_index, GTFArgs::InputDataCoinData),
        // compare
        op::meq(res_reg, output_data_reg, input_data_reg, len_reg),
        // op::meq(res_reg, input_data_reg, output_data_reg, len_reg),
        op::ret(res_reg),
    ];

    // when
    let success = execute_data_coin_predicate_input_and_output(
        predicate,
        predicate_data,
        data.clone(),
        data,
    )
    .await;

    // then
    assert!(success);
}

#[tokio::test]
async fn gtf_args__output_data_coin_to() {
    // given
    let mut rng = StdRng::seed_from_u64(2322u64);

    // Create a random address for the output
    let output_owner: Address = rng.gen();

    // Store the expected owner in predicate data
    let predicate_data = output_owner.to_bytes().to_vec();

    let expected_owner_reg = 0x11;
    let actual_owner_reg = 0x12;
    let address_size = output_owner.size(); // Address is 20 bytes
    let address_size_reg = 0x13;
    let res_reg = 0x10;
    let output_index = 0;
    let input_index = 0;

    let predicate = [
        op::movi(address_size_reg, address_size as u32),
        op::gtf_args(
            expected_owner_reg,
            input_index,
            GTFArgs::InputCoinPredicateData,
        ),
        op::gtf_args(actual_owner_reg, output_index, GTFArgs::OutputCoinTo),
        op::meq(
            res_reg,
            expected_owner_reg,
            actual_owner_reg,
            address_size_reg,
        ),
        op::ret(res_reg),
    ];

    let amount = rng.gen();
    let asset_id = rng.gen();
    // Create the output with the expected owner and some data
    let output_data = vec![1, 2, 3, 4, 5];
    let output = Output::data_coin(
        output_owner,
        amount,   // amount
        asset_id, // asset_id
        output_data,
    );

    // Create a dummy input
    let predicate_bytes = predicate
        .iter()
        .copied()
        .flat_map(|op| u32::from(op).to_be_bytes())
        .collect::<Vec<u8>>();
    let input_owner = Input::predicate_owner(&predicate_bytes);
    let utxo_id = rng.gen();
    let tx_pointer = rng.gen();
    let predicate_gas_used = 0;

    let input = Input::coin_predicate(
        utxo_id,
        input_owner,
        amount,
        asset_id,
        tx_pointer,
        predicate_gas_used,
        predicate_bytes,
        predicate_data,
    );

    // when
    let success =
        execute_predicate_with_input_and_output(vec![input], vec![output]).await;

    // then
    assert!(success);
}

#[tokio::test]
async fn gtf_args__output_data_coin_amount() {
    // given
    let mut rng = StdRng::seed_from_u64(2322u64);

    let expected_amount_reg = 0x11;
    let actual_amount_reg = 0x12;
    let res_reg = 0x10;
    let output_index = 0;
    let amount: Word = 123;

    let predicate = [
        op::movi(expected_amount_reg, amount as u32),
        op::gtf_args(actual_amount_reg, output_index, GTFArgs::OutputCoinAmount),
        op::eq(res_reg, expected_amount_reg, actual_amount_reg),
        op::ret(res_reg),
    ];

    // Create a dummy input
    let predicate_bytes = predicate
        .iter()
        .copied()
        .flat_map(|op| u32::from(op).to_be_bytes())
        .collect::<Vec<u8>>();
    let input_owner = Input::predicate_owner(&predicate_bytes);
    let utxo_id = rng.gen();
    let asset_id = rng.gen();
    let tx_pointer = rng.gen();
    let predicate_gas_used = 0;

    // Store the expected amount in predicate data
    let predicate_data = amount.to_be_bytes().to_vec();

    // Create the output with the expected amount and some data
    let output_data = vec![1, 2, 3, 4, 5];
    let output = Output::data_coin(
        rng.gen(), // to
        amount,
        asset_id, // asset_id
        output_data,
    );

    let input = Input::coin_predicate(
        utxo_id,
        input_owner,
        amount,
        asset_id,
        tx_pointer,
        predicate_gas_used,
        predicate_bytes,
        predicate_data,
    );

    // when
    let success =
        execute_predicate_with_input_and_output(vec![input], vec![output]).await;

    // then
    assert!(success);
}

#[tokio::test]
async fn gtf_args__output_data_coin_asset_id() {
    // given
    let mut rng = StdRng::seed_from_u64(2322u64);

    let asset_id: AssetId = rng.gen();

    // Store the expected asset ID in predicate data
    let predicate_data = asset_id.to_bytes();

    let expected_asset_id_reg = 0x11;
    let actual_asset_id_reg = 0x12;
    let res_reg = 0x10;
    let output_index = 0;
    let asset_id_size = predicate_data.len() as u32;
    let asset_id_size_reg = 0x13;

    let predicate = [
        op::movi(asset_id_size_reg, asset_id_size),
        op::gtf_args(
            expected_asset_id_reg,
            output_index,
            GTFArgs::OutputCoinAssetId,
        ),
        op::gtf_args(actual_asset_id_reg, 0, GTFArgs::InputCoinPredicateData),
        op::meq(
            res_reg,
            expected_asset_id_reg,
            actual_asset_id_reg,
            asset_id_size_reg,
        ),
        op::ret(res_reg),
    ];

    // Create the output with the expected asset ID and some data
    let output_data = vec![1, 2, 3, 4, 5];
    let amount = rng.gen();
    let output = Output::data_coin(
        rng.gen(), // to
        amount,
        asset_id,
        output_data,
    );

    // Create a dummy input
    let predicate_bytes = predicate
        .iter()
        .copied()
        .flat_map(|op| u32::from(op).to_be_bytes())
        .collect::<Vec<u8>>();
    let input_owner = Input::predicate_owner(&predicate_bytes);
    let utxo_id = rng.gen();
    let tx_pointer = rng.gen();
    let predicate_gas_used = 0;

    let input = Input::coin_predicate(
        utxo_id,
        input_owner,
        amount,
        asset_id,
        tx_pointer,
        predicate_gas_used,
        predicate_bytes,
        predicate_data,
    );

    // when
    let success =
        execute_predicate_with_input_and_output(vec![input], vec![output]).await;

    // then
    assert!(success);
}

fn check_read_only_unverified_coin_utxo_id_predicate(
    expected: UtxoId,
    utxo_id_bytes: &[u8],
) -> Vec<u8> {
    let utxo_id_size = utxo_id_bytes.len();
    let utxo_id_size_reg = 0x13;
    let actual_utxo_id_reg = 0x12;
    let expected_utxo_id_reg = 0x11;
    let res_reg = 0x10;
    let read_only_index = 0;
    let predicate_input_index = 1;
    vec![
        op::movi(utxo_id_size_reg, utxo_id_size as u32),
        op::gtf_args(actual_utxo_id_reg, read_only_index, GTFArgs::InputCoinTxId),
        op::gtf_args(
            expected_utxo_id_reg,
            predicate_input_index,
            GTFArgs::InputCoinPredicateData,
        ),
        op::meq(
            res_reg,
            expected_utxo_id_reg,
            actual_utxo_id_reg,
            utxo_id_size_reg,
        ),
        op::ret(res_reg),
    ]
    .into_iter()
    .collect()
}

#[tokio::test]
async fn gtf_args__read_only_unverified_coin_utxo_id() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    // given
    let utxo_id: UtxoId = rng.gen();
    let owner = rng.gen();
    let amount = 123;
    let asset_id = rng.gen();
    let tx_pointer = rng.gen();
    let predicate_data = utxo_id.to_bytes();
    let predicate =
        check_read_only_unverified_coin_utxo_id_predicate(utxo_id, &predicate_data);
    let read_input =
        Input::unverified_read_only_coin(utxo_id, owner, amount, asset_id, tx_pointer);
    let predicate_input = Input::coin_predicate(
        rng.gen(),
        Input::predicate_owner(&predicate),
        234,
        asset_id,
        rng.gen(),
        0,
        predicate.clone(),
        predicate_data,
    );
    let output = Output::change(rng.gen(), 0, asset_id);

    // when
    let success = execute_predicate_with_input_and_output(
        vec![read_input, predicate_input],
        vec![output],
    )
    .await;

    // then
    assert!(success);
}

// do owner
fn check_read_only_unverified_coin_owner_predicate(owner: Address) -> Vec<u8> {
    let owner_size = owner.size();
    let owner_size_reg = 0x13;
    let actual_owner_reg = 0x12;
    let expected_owner_reg = 0x11;
    let res_reg = 0x10;
    let read_only_index = 0;
    let predicate_input_index = 1;
    vec![
        op::movi(owner_size_reg, owner_size as u32),
        op::gtf_args(actual_owner_reg, read_only_index, GTFArgs::InputCoinOwner),
        op::gtf_args(
            expected_owner_reg,
            predicate_input_index,
            GTFArgs::InputCoinPredicateData,
        ),
        op::meq(
            res_reg,
            expected_owner_reg,
            actual_owner_reg,
            owner_size_reg,
        ),
        op::ret(res_reg),
    ]
    .into_iter()
    .collect()
}

#[tokio::test]
async fn gtf_args__read_only_unverified_coin_owner() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    // given
    let utxo_id: UtxoId = rng.gen();
    let owner: Address = rng.gen();
    let amount = 123;
    let asset_id = rng.gen();
    let tx_pointer = rng.gen();
    let predicate_data = owner.to_bytes();
    let predicate = check_read_only_unverified_coin_owner_predicate(owner);
    let read_input =
        Input::unverified_read_only_coin(utxo_id, owner, amount, asset_id, tx_pointer);
    let predicate_input = Input::coin_predicate(
        rng.gen(),
        Input::predicate_owner(&predicate),
        234,
        asset_id,
        rng.gen(),
        0,
        predicate.clone(),
        predicate_data,
    );
    let output = Output::change(rng.gen(), 0, asset_id);

    // when
    let success = execute_predicate_with_input_and_output(
        vec![read_input, predicate_input],
        vec![output],
    )
    .await;

    // then
    assert!(success);
}

fn check_read_only_unverified_coin_amount_predicate(amount: Word) -> Vec<u8> {
    let expected_amount_reg = 0x13;
    let actual_amount_reg = 0x12;
    let res_reg = 0x10;
    let read_only_index = 0;
    let predicate_input_index = 1;
    vec![
        op::movi(expected_amount_reg, amount as u32),
        op::gtf_args(actual_amount_reg, read_only_index, GTFArgs::InputCoinAmount),
        op::eq(res_reg, expected_amount_reg, actual_amount_reg),
        op::ret(res_reg),
    ]
    .into_iter()
    .collect()
}

#[tokio::test]
async fn gtf_args__read_only_unverified_coin_amount() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    // given
    let utxo_id: UtxoId = rng.gen();
    let owner = rng.gen();
    let amount = 123;
    let asset_id = rng.gen();
    let tx_pointer = rng.gen();
    let predicate_data = utxo_id.to_bytes();
    let predicate = check_read_only_unverified_coin_amount_predicate(amount);
    let read_input =
        Input::unverified_read_only_coin(utxo_id, owner, amount, asset_id, tx_pointer);
    let predicate_input = Input::coin_predicate(
        rng.gen(),
        Input::predicate_owner(&predicate),
        234,
        asset_id,
        rng.gen(),
        0,
        predicate.clone(),
        predicate_data,
    );
    let output = Output::change(rng.gen(), 0, asset_id);

    // when
    let success = execute_predicate_with_input_and_output(
        vec![read_input, predicate_input],
        vec![output],
    )
    .await;

    // then
    assert!(success);
}

fn check_read_only_unverified_coin_asset_id_predicate(
    asset_id: AssetId,
    asset_id_size: usize,
) -> Vec<u8> {
    let asset_id_size_reg = 0x13;
    let actual_utxo_id_reg = 0x12;
    let expected_utxo_id_reg = 0x11;
    let res_reg = 0x10;
    let read_only_index = 0;
    let predicate_input_index = 1;
    vec![
        op::movi(asset_id_size_reg, asset_id_size as u32),
        op::gtf_args(
            actual_utxo_id_reg,
            read_only_index,
            GTFArgs::InputCoinAssetId,
        ),
        op::gtf_args(
            expected_utxo_id_reg,
            predicate_input_index,
            GTFArgs::InputCoinPredicateData,
        ),
        op::meq(
            res_reg,
            expected_utxo_id_reg,
            actual_utxo_id_reg,
            asset_id_size_reg,
        ),
        op::ret(res_reg),
    ]
    .into_iter()
    .collect()
}

#[tokio::test]
async fn gtf_args__read_only_unverified_coin_asset_id() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    // given
    let utxo_id: UtxoId = rng.gen();
    let owner = rng.gen();
    let amount = 123;
    let asset_id: AssetId = rng.gen();
    let tx_pointer = rng.gen();
    let predicate_data = asset_id.to_bytes();
    let predicate = check_read_only_unverified_coin_asset_id_predicate(
        asset_id,
        predicate_data.len(),
    );
    let read_input =
        Input::unverified_read_only_coin(utxo_id, owner, amount, asset_id, tx_pointer);
    let predicate_input = Input::coin_predicate(
        rng.gen(),
        Input::predicate_owner(&predicate),
        234,
        asset_id,
        rng.gen(),
        0,
        predicate.clone(),
        predicate_data,
    );
    let output = Output::change(rng.gen(), 0, asset_id);

    // when
    let success = execute_predicate_with_input_and_output(
        vec![read_input, predicate_input],
        vec![output],
    )
    .await;

    // then
    assert!(success);
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
    let result =
        transaction.estimate_predicates(&params, MemoryInstance::new(), &EmptyStorage);

    // Then
    assert_eq!(Ok(()), result);
}
