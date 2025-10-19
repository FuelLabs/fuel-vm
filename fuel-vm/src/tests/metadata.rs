use alloc::{
    borrow::ToOwned,
    vec,
    vec::Vec,
};
use consensus_parameters::gas::GasCostsValuesV5;

use crate::{
    checked_transaction::EstimatePredicates,
    consts::*,
    interpreter::{
        InterpreterParams,
        NotSupportedEcal,
    },
    storage::{
        UploadedBytecode,
        predicate::EmptyStorage,
    },
};
use fuel_asm::{
    GMArgs,
    GTFArgs,
    RegId,
    op,
};
use fuel_crypto::Hasher;
use fuel_tx::{
    ConsensusParameters,
    Finalizable,
    Receipt,
    Script,
    TransactionBuilder,
    field::{
        Inputs,
        Outputs,
        ReceiptsRoot,
        Script as ScriptField,
        Witnesses,
    },
    policies::PoliciesBits,
};
use fuel_types::{
    BlockHeight,
    ChainId,
    bytes,
    canonical::Serialize,
};
use rand::{
    Rng,
    SeedableRng,
    rngs::StdRng,
};

use crate::prelude::{
    GasCosts,
    *,
};

/// Allocates a byte array from heap and initializes it. Then points `reg` to it.
fn alloc_bytearray<const S: usize>(reg: u8, v: [u8; S]) -> Vec<Instruction> {
    let mut ops = vec![op::movi(reg, S as u32), op::aloc(reg)];
    for (i, b) in v.iter().enumerate() {
        if *b != 0 {
            ops.push(op::movi(reg, *b as u32));
            ops.push(op::sb(RegId::HP, reg, i as u16));
        }
    }
    ops.push(op::move_(reg, RegId::HP));
    ops
}

#[test]
fn metadata() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = Default::default();
    let height = Default::default();

    let consensus_params = ConsensusParameters::standard();

    #[rustfmt::skip]
    let routine_metadata_is_caller_external = vec![
        op::gm_args(0x10, GMArgs::IsCallerExternal),
        op::gm_args(0x11, GMArgs::GetCaller),
        op::log(0x10, 0x00, 0x00, 0x00),
        op::movi(0x20, ContractId::LEN as Immediate18),
        op::logd(0x00, 0x00, 0x11, 0x20),
        op::ret(RegId::ONE),
    ];

    let salt: Salt = rng.r#gen();
    let program: Witness = routine_metadata_is_caller_external
        .into_iter()
        .collect::<Vec<u8>>()
        .into();

    let contract_root = Contract::root_from_code(program.as_ref());
    let state_root = Contract::default_state_root();
    let contract_metadata = Contract::id(&salt, &contract_root, &state_root);

    let tx = TransactionBuilder::create(program, salt, vec![])
        .maturity(maturity)
        .add_fee_input()
        .add_contract_created()
        .finalize()
        .into_checked(height, &consensus_params)
        .expect("failed to check tx");

    let interpreter_params = InterpreterParams::new(gas_price, &consensus_params);

    // Deploy the contract into the blockchain
    assert!(
        Transactor::<_, _, _>::new(
            MemoryInstance::new(),
            &mut storage,
            interpreter_params.clone()
        )
        .transact(tx)
        .is_success()
    );

    let mut routine_call_metadata_contract = vec![
        op::gm_args(0x10, GMArgs::IsCallerExternal),
        op::log(0x10, 0x00, 0x00, 0x00),
        op::movi(0x10, (Bytes32::LEN + 2 * Bytes8::LEN) as Immediate18),
        op::aloc(0x10),
        op::move_(0x10, RegId::HP),
    ];

    contract_metadata
        .as_ref()
        .iter()
        .enumerate()
        .for_each(|(i, b)| {
            routine_call_metadata_contract.push(op::movi(0x11, *b as Immediate18));
            routine_call_metadata_contract.push(op::sb(0x10, 0x11, i as Immediate12));
        });

    routine_call_metadata_contract.push(op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS));
    routine_call_metadata_contract.push(op::ret(RegId::ONE));

    let salt: Salt = rng.r#gen();
    let program: Witness = routine_call_metadata_contract
        .into_iter()
        .collect::<Vec<u8>>()
        .into();

    let contract_root = Contract::root_from_code(program.as_ref());
    let state_root = Contract::default_state_root();
    let contract_id = Contract::id(&salt, &contract_root, &state_root);
    let tx = TransactionBuilder::create(program, salt, vec![])
        .maturity(maturity)
        .add_fee_input()
        .add_contract_created()
        .finalize()
        .into_checked(height, &consensus_params)
        .expect("failed to check tx");

    assert!(
        Transactor::<_, _, _>::new(
            MemoryInstance::new(),
            &mut storage,
            interpreter_params.clone()
        )
        .transact(tx)
        .is_success()
    );

    let mut inputs = vec![];
    let mut outputs = vec![];

    inputs.push(Input::contract(
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        contract_id,
    ));
    outputs.push(Output::contract(0, rng.r#gen(), rng.r#gen()));

    inputs.push(Input::contract(
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        rng.r#gen(),
        contract_metadata,
    ));
    outputs.push(Output::contract(1, rng.r#gen(), rng.r#gen()));

    let mut script = vec![
        op::movi(0x10, (1 + Bytes32::LEN + 2 * Bytes8::LEN) as Immediate18),
        op::aloc(0x10),
        op::move_(0x10, RegId::HP),
    ];

    contract_id.as_ref().iter().enumerate().for_each(|(i, b)| {
        script.push(op::movi(0x11, *b as Immediate18));
        script.push(op::sb(0x10, 0x11, i as Immediate12));
    });

    script.push(op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS));
    script.push(op::ret(RegId::ONE));

    #[allow(clippy::iter_cloned_collect)]
    // collection is also perfomring a type conversion
    let script = script.iter().copied().collect::<Vec<u8>>();

    let tx = TransactionBuilder::script(script, vec![])
        .script_gas_limit(gas_limit)
        .maturity(maturity)
        .add_input(inputs[0].clone())
        .add_input(inputs[1].clone())
        .add_output(outputs[0].clone())
        .add_output(outputs[1].clone())
        .add_fee_input()
        .finalize()
        .into_checked(height, &consensus_params)
        .expect("failed to check tx");

    let receipts = Transactor::<_, _, _>::new(
        MemoryInstance::new(),
        &mut storage,
        interpreter_params,
    )
    .transact(tx)
    .receipts()
    .expect("Failed to transact")
    .to_owned();

    let ra = receipts[1]
        .ra()
        .expect("IsCallerExternal should set $rA as boolean flag");
    assert_eq!(1, ra);

    let ra = receipts[3]
        .ra()
        .expect("IsCallerExternal should set $rA as boolean flag");
    assert_eq!(0, ra);

    let contract_call = Hasher::hash(contract_id.as_ref());
    let digest = receipts[4]
        .digest()
        .expect("GetCaller should return contract Id");
    assert_eq!(&contract_call, digest);
}

#[test]
fn get_metadata_chain_id() {
    let rng = &mut StdRng::seed_from_u64(2322u64);
    let gas_limit = 1_000_000;
    let height = BlockHeight::default();

    let chain_id: ChainId = rng.r#gen();

    let interpreter_params = InterpreterParams {
        chain_id,
        ..Default::default()
    };

    let mut client = MemoryClient::<_, NotSupportedEcal>::new(
        MemoryInstance::new(),
        Default::default(),
        interpreter_params,
    );

    #[rustfmt::skip]
    let get_chain_id = vec![
        op::gm_args(0x10, GMArgs::GetChainId),
        op::ret(0x10),
    ];

    let consensus_params = ConsensusParameters::standard_with_id(chain_id);

    let script = TransactionBuilder::script(get_chain_id.into_iter().collect(), vec![])
        .script_gas_limit(gas_limit)
        .with_chain_id(chain_id)
        .add_fee_input()
        .finalize()
        .into_checked(height, &consensus_params)
        .unwrap();

    let receipts = client.transact(script);

    if let Receipt::Return { val, .. } = receipts[0].clone() {
        assert_eq!(val, *chain_id);
    } else {
        panic!("expected return receipt, instead of {:?}", receipts[0])
    }
}

#[test]
fn get_metadata_base_asset_id() {
    let gas_limit = 1_000_000;
    let height = BlockHeight::default();
    let mut storage = MemoryStorage::default();

    let mut params = ConsensusParameters::standard();
    params.set_base_asset_id(AssetId::from([5; 32]));

    let script = TransactionBuilder::script(
        vec![
            op::gm_args(0x20, GMArgs::BaseAssetId),
            op::movi(0x21, AssetId::LEN.try_into().unwrap()),
            op::logd(RegId::ZERO, RegId::ZERO, 0x20, 0x21),
            op::ret(RegId::ONE),
        ]
        .into_iter()
        .collect(),
        vec![],
    )
    .script_gas_limit(gas_limit)
    .add_fee_input()
    .finalize()
    .into_checked(height, &params)
    .unwrap();

    let receipts = Transactor::<_, _, _>::new(
        MemoryInstance::new(),
        &mut storage,
        InterpreterParams {
            base_asset_id: *params.base_asset_id(),
            ..Default::default()
        },
    )
    .transact(script)
    .receipts()
    .expect("Failed to transact")
    .to_owned();

    if let Receipt::LogData { data, .. } = receipts[0].clone() {
        assert_eq!(data.unwrap(), params.base_asset_id().to_bytes().into());
    } else {
        panic!("expected LogData receipt, instead of {:?}", receipts[0]);
    }
}

#[test]
fn get_metadata_tx_start() {
    let gas_limit = 1_000_000;
    let height = BlockHeight::default();
    let mut storage = MemoryStorage::default();

    let script = TransactionBuilder::script(
        vec![op::gm_args(0x20, GMArgs::TxStart), op::ret(0x20)]
            .into_iter()
            .collect(),
        vec![],
    )
    .script_gas_limit(gas_limit)
    .add_fee_input()
    .finalize()
    .into_checked(height, &ConsensusParameters::default())
    .unwrap();

    let receipts = Transactor::<_, _, _>::new(
        MemoryInstance::new(),
        &mut storage,
        InterpreterParams::default(),
    )
    .transact(script)
    .receipts()
    .expect("Failed to transact")
    .to_owned();

    if let Receipt::Return { val, .. } = receipts[0].clone() {
        assert_eq!(val, TxParameters::DEFAULT.tx_offset() as Word);
    } else {
        panic!("expected return receipt, instead of {:?}", receipts[0])
    }
}

#[test]
fn get_metadata__gas_price__script() {
    // given
    let gas_limit = 1_000_000;
    let height = BlockHeight::default();
    let mut storage = MemoryStorage::default();

    let gas_price = 123;

    let script = TransactionBuilder::script(
        vec![op::gm_args(0x20, GMArgs::GetGasPrice), op::ret(0x20)]
            .into_iter()
            .collect(),
        vec![],
    )
    .script_gas_limit(gas_limit)
    .add_fee_input()
    .add_max_fee_limit(10)
    .finalize()
    .into_checked(height, &ConsensusParameters::default())
    .unwrap();

    let interpreter_params = InterpreterParams {
        gas_price,
        ..Default::default()
    };

    // when
    let receipts = Transactor::<_, _, _>::new(
        MemoryInstance::new(),
        &mut storage,
        interpreter_params,
    )
    .transact(script)
    .receipts()
    .expect("Failed to transact")
    .to_owned();

    // then
    if let Receipt::Return { val, .. } = receipts[0].clone() {
        assert_eq!(val, gas_price);
    } else {
        panic!("expected return receipt, instead of {:?}", receipts[0])
    }
}

#[test]
fn get_metadata__gas_price__contract() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    // given
    let mut storage = MemoryStorage::default();

    let gas_price = 123;
    let gas_limit = 1_000_000;
    let maturity = Default::default();
    let height = Default::default();

    let consensus_params = ConsensusParameters::standard();

    // Create a contract that gets the gas price and returns it
    #[rustfmt::skip]
    let contract_bytecode = vec![
        op::gm_args(0x10, GMArgs::GetGasPrice),
        op::ret(0x10),
    ];

    let salt: Salt = rng.r#gen();
    let program: Witness = contract_bytecode.into_iter().collect::<Vec<u8>>().into();

    let contract_root = Contract::root_from_code(program.as_ref());
    let state_root = Contract::default_state_root();
    let contract_id = Contract::id(&salt, &contract_root, &state_root);

    // Deploy the contract
    let tx = TransactionBuilder::create(program, salt, vec![])
        .maturity(maturity)
        .add_fee_input()
        .add_contract_created()
        .add_max_fee_limit(10)
        .finalize()
        .into_checked(height, &consensus_params)
        .expect("failed to check tx");

    let interpreter_params = InterpreterParams {
        gas_price,
        ..Default::default()
    };

    // Deploy the contract into the blockchain
    assert!(
        Transactor::<_, _, _>::new(
            MemoryInstance::new(),
            &mut storage,
            interpreter_params.clone()
        )
        .transact(tx)
        .is_success()
    );

    let mut script = vec![
        op::movi(0x10, (1 + Bytes32::LEN + 2 * Bytes8::LEN) as Immediate18),
        op::aloc(0x10),
        op::move_(0x10, RegId::HP),
    ];

    // Copy contract ID bytes into the allocated memory
    contract_id.as_ref().iter().enumerate().for_each(|(i, b)| {
        script.push(op::movi(0x11, *b as Immediate18));
        script.push(op::sb(0x10, 0x11, i as Immediate12));
    });

    // Call the contract and forward the returned value
    script.push(op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS));
    script.push(op::ret(RegId::ONE));

    let tx = TransactionBuilder::script(script.into_iter().collect(), vec![])
        .script_gas_limit(gas_limit)
        .add_input(Input::contract(
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
            contract_id,
        ))
        .add_output(Output::contract(0, rng.r#gen(), rng.r#gen()))
        .add_fee_input()
        .add_max_fee_limit(10)
        .finalize()
        .into_checked(height, &consensus_params)
        .expect("failed to check tx");

    // when
    let receipts = Transactor::<_, _, _>::new(
        MemoryInstance::new(),
        &mut storage,
        interpreter_params,
    )
    .transact(tx)
    .receipts()
    .expect("Failed to transact")
    .to_owned();

    // then
    let receipt = find_matching_return_receipt_for_contract_id(&receipts, &contract_id)
        .expect("There should be a return receipt for contract");

    if let Receipt::Return { val, .. } = receipt {
        assert_eq!(val, gas_price);
    } else {
        panic!("expected return receipt, instead of {:?}", receipt)
    }
}

fn find_matching_return_receipt_for_contract_id(
    receipts: &[Receipt],
    contract_id: &ContractId,
) -> Option<Receipt> {
    receipts
        .iter()
        .find(|receipt| {
            if let Receipt::Return { id, .. } = receipt {
                contract_id == id
            } else {
                false
            }
        })
        .cloned()
}

#[allow(deprecated)]
#[test]
fn get_transaction_fields() {
    let rng = &mut StdRng::seed_from_u64(2322u64);
    let gas_costs = GasCosts::default();

    let mut client = MemoryClient::default();

    let witness_limit = 1234;
    let max_fee_limit = 4321;
    let tip = 4321;
    let gas_limit = 10_000_000;
    let maturity = 50.into();
    let height = 122.into();
    let expiration = 123.into();
    let owner_idx = 1;
    let input = 10_000_000;

    let tx_params = TxParameters::default();

    let contract: Witness = vec![op::ret(0x01)].into_iter().collect::<Vec<u8>>().into();
    let salt = rng.r#gen();
    let code_root = Contract::root_from_code(contract.as_ref());
    let storage_slots = vec![];
    let state_root = Contract::initial_state_root(storage_slots.iter());
    let contract_id = Contract::id(&salt, &code_root, &state_root);

    let tx = TransactionBuilder::create(contract, salt, storage_slots)
        .add_unsigned_coin_input(
            SecretKey::random(rng),
            rng.r#gen(),
            max_fee_limit,
            AssetId::zeroed(),
            rng.r#gen(),
        )
        .add_contract_created()
        .finalize_checked(height);

    client.deploy(tx).unwrap();

    let predicate = vec![op::ret(RegId::ONE)].into_iter().collect::<Vec<u8>>();
    let mut predicate_data = vec![0u8; 512];

    rng.fill(predicate_data.as_mut_slice());

    let owner = Input::predicate_owner(&predicate);
    let input_coin_predicate = Input::coin_predicate(
        rng.r#gen(),
        owner,
        1_500,
        rng.r#gen(),
        rng.r#gen(),
        gas_costs.ret(),
        predicate.clone(),
        predicate_data.clone(),
    );

    let contract_input_index = 2;

    let message_amount = 5_500;
    let mut message_data = vec![0u8; 256];
    rng.fill(message_data.as_mut_slice());

    let mut m_data = vec![0u8; 64];
    let m_predicate = vec![op::ret(RegId::ONE)].into_iter().collect::<Vec<u8>>();
    let mut m_predicate_data = vec![0u8; 512];

    rng.fill(m_data.as_mut_slice());
    rng.fill(m_predicate_data.as_mut_slice());

    let owner = Input::predicate_owner(&m_predicate);
    let message_predicate = Input::message_data_predicate(
        rng.r#gen(),
        owner,
        7_500,
        rng.r#gen(),
        gas_costs.ret(),
        m_data.clone(),
        m_predicate.clone(),
        m_predicate_data.clone(),
    );

    let asset = rng.r#gen();
    let asset_amt = 27;

    let tx = TransactionBuilder::script(vec![], vec![])
        .maturity(maturity)
        .expiration(expiration)
        .owner(owner_idx)
        .with_gas_costs(gas_costs)
        .script_gas_limit(gas_limit)
        .add_unsigned_coin_input(
            SecretKey::random(rng),
            rng.r#gen(),
            input,
            AssetId::zeroed(),
            rng.r#gen(),
        )
        .add_input(input_coin_predicate)
        .add_input(Input::contract(
            rng.r#gen(),
            rng.r#gen(),
            state_root,
            rng.r#gen(),
            contract_id,
        ))
        .add_output(Output::variable(rng.r#gen(), rng.r#gen(), rng.r#gen()))
        .add_output(Output::contract(
            contract_input_index,
            rng.r#gen(),
            state_root,
        ))
        .add_witness(Witness::from(b"some-data".to_vec()))
        .add_unsigned_message_input(
            SecretKey::random(rng),
            rng.r#gen(),
            rng.r#gen(),
            message_amount,
            message_data.clone(),
        )
        .add_input(message_predicate)
        .add_unsigned_coin_input(
            SecretKey::random(rng),
            rng.r#gen(),
            asset_amt,
            asset,
            rng.r#gen(),
        )
        .add_output(Output::coin(rng.r#gen(), asset_amt, asset))
        .add_output(Output::change(rng.r#gen(), rng.gen_range(10..1000), asset))
        .finalize_checked(height);

    let inputs = tx.as_ref().inputs();
    let outputs = tx.as_ref().outputs();
    let witnesses = tx.as_ref().witnesses();

    let inputs_bytes: Vec<Vec<u8>> = inputs
        .iter()
        .map(|v| {
            let mut v = v.clone();
            v.prepare_sign();
            v.clone().to_bytes()
        })
        .collect();
    let outputs_bytes: Vec<Vec<u8>> = outputs
        .iter()
        .map(|v| {
            let mut v = v.clone();
            v.prepare_init_execute();
            v.clone().to_bytes()
        })
        .collect();
    let witnesses_bytes: Vec<Vec<u8>> =
        witnesses.iter().map(|w| w.clone().to_bytes()).collect();

    let receipts_root = tx.as_ref().receipts_root();

    let base_asset_id = AssetId::BASE;

    #[rustfmt::skip]
    let cases = vec![
        inputs_bytes[0].clone(), // 0 - ScriptInputAtIndex
        outputs_bytes[0].clone(), // 1 - ScriptOutputAtIndex
        witnesses_bytes[1].clone(), // 2 - ScriptWitnessAtIndex
        receipts_root.to_vec(), // 3 - ScriptReceiptsRoot
        inputs[0].utxo_id().unwrap().clone().to_bytes(), // 4- InputCoinTxId
        inputs[0].input_owner().unwrap().to_vec(), // 5 - InputCoinOwner
        inputs[0].asset_id(&base_asset_id).unwrap().to_vec(), // 6 - InputCoinAssetId
        predicate.clone(), // 7 - InputCoinPredicate
        predicate_data.clone(), // 8 - InputCoinPredicateData
        inputs[2].utxo_id().unwrap().clone().to_bytes(), // 9 - InputContractTxId
        inputs[2].balance_root().unwrap().to_vec(), // 10 - InputContractBalanceRoot
        inputs[2].state_root().unwrap().to_vec(), // 11 - InputContractStateRoot
        inputs[2].contract_id().unwrap().to_vec(), // 12 - InputContractId
        inputs[3].sender().unwrap().to_vec(), // 13 - InputMessageSender
        inputs[3].recipient().unwrap().to_vec(), // 14 - InputMessageRecipient
        inputs[3].nonce().unwrap().to_vec(), // 15 - InputMessageNonce
        m_data.clone(), // 16 - InputMessageData
        m_predicate.clone(), // 17 - InputMessagePredicate
        m_predicate_data.clone(), // 18 - InputMessagePredicateData
        outputs[2].to().unwrap().to_vec(), // 19 - OutputCoinTo
        outputs[2].asset_id().unwrap().to_vec(), // 20 - OutputCoinAssetId
        outputs[1].balance_root().unwrap().to_vec(), // 21 - OutputContractBalanceRoot
        outputs[1].state_root().unwrap().to_vec(), // 22 - OutputContractStateRoot
        witnesses[1].as_ref().to_vec(), // 23 - WitnessData
        outputs[3].asset_id().unwrap().to_vec(), // 24 - OutputCoinAssetId
        outputs[3].to().unwrap().to_vec(), // 25 - OutputCoinTo
        inputs[1].tx_pointer().unwrap().clone().to_bytes(), // 26 - InputCoinTxPointer
        inputs[2].tx_pointer().unwrap().clone().to_bytes() // 27 - InputContractTxPointer
    ];

    // hardcoded metadata of script len so it can be checked at runtime
    let script_reserved_words = 300 * WORD_SIZE;
    let script_offset = tx_params.tx_offset() + Script::script_offset_static();
    let script_data_offset = script_offset.saturating_add(
        bytes::padded_len_usize(script_reserved_words).unwrap_or(usize::MAX),
    );
    let script_data: Vec<u8> = cases.iter().flat_map(|c| c.iter()).copied().collect();

    #[rustfmt::skip]
    let mut script: Vec<u8> = vec![
        op::movi(0x20, 0x01),
        op::gtf_args(0x30, 0x19, GTFArgs::ScriptData),

        op::movi(0x19, 0x00),
        op::movi(0x11, TransactionRepr::Script as Immediate18),
        op::gtf_args(0x10, 0x19, GTFArgs::Type),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::gtf_args(0x10, RegId::ZERO, GTFArgs::TxLength),
        op::movi(0x11, 100), // Tx lenght is too complicated to backpatch
        op::gt(0x10, 0x10, 0x11), // so just make sure it's over some arbitrary number
        op::and(0x20, 0x20, 0x10),
        op::movi(0x11, 10_000), // and
        op::lt(0x10, 0x10, 0x11), // below some arbitrary number
        op::and(0x20, 0x20, 0x10),

        op::movi(0x11, tip as Immediate18),
        op::movi(0x19, 0x00),
        op::gtf_args(0x10, 0x19, GTFArgs::PolicyTip),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x00),
        op::movi(0x11, (gas_limit & 0x3ffff) as Immediate18),
        op::movi(0x12, (gas_limit >> 18) as Immediate18),
        op::slli(0x12, 0x12, 18),
        op::or(0x11, 0x11, 0x12),
        op::gtf_args(0x10, 0x19, GTFArgs::ScriptGasLimit),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x00),
        op::movi(0x11, *maturity as Immediate18),
        op::gtf_args(0x10, 0x19, GTFArgs::PolicyMaturity),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x00),
        op::movi(0x11, *expiration as Immediate18),
        op::gtf_args(0x10, 0x19, GTFArgs::PolicyExpiration),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x00),
        op::movi(0x11, max_fee_limit as Immediate18),
        op::gtf_args(0x10, 0x19, GTFArgs::PolicyMaxFee),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x00),
        op::movi(0x11, witness_limit as Immediate18),
        op::gtf_args(0x10, 0x19, GTFArgs::PolicyWitnessLimit),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x00),
        op::movi(0x11, owner_idx as Immediate18),
        op::gtf_args(0x10, 0x19, GTFArgs::PolicyOwner),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x00),
        op::movi(0x11, PoliciesBits::all().bits() as Immediate18),
        op::gtf_args(0x10, 0x19, GTFArgs::PolicyTypes),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x00),
        op::movi(0x11, inputs.len() as Immediate18),
        op::gtf_args(0x10, 0x19, GTFArgs::ScriptInputsCount),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x00),
        op::movi(0x11, outputs.len() as Immediate18),
        op::gtf_args(0x10, 0x19, GTFArgs::ScriptOutputsCount),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x00),
        op::movi(0x11, witnesses.len() as Immediate18),
        op::gtf_args(0x10, 0x19, GTFArgs::ScriptWitnessesCount),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x00),
        op::gtf_args(0x10, 0x19, GTFArgs::ScriptInputAtIndex),
        op::movi(0x11, cases[0].len() as Immediate18),
        op::meq(0x10, 0x10, 0x30, 0x11),
        op::add(0x30, 0x30, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x00),
        op::gtf_args(0x10, 0x19, GTFArgs::ScriptOutputAtIndex),
        op::movi(0x11, cases[1].len() as Immediate18),
        op::meq(0x10, 0x10, 0x30, 0x11),
        op::add(0x30, 0x30, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x01),
        op::gtf_args(0x10, 0x19, GTFArgs::ScriptWitnessAtIndex),
        op::movi(0x11, cases[2].len() as Immediate18),
        op::meq(0x10, 0x10, 0x30, 0x11),
        op::add(0x30, 0x30, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x11, script_reserved_words as Immediate18),
        op::movi(0x19, 0x00),
        op::gtf_args(0x10, 0x19, GTFArgs::ScriptLength),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x11, script_data.len() as Immediate18),
        op::movi(0x19, 0x00),
        op::gtf_args(0x10, 0x19, GTFArgs::ScriptDataLength),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        // Skip over ReceiptsRoot which is always zero
        op::addi(0x30, 0x30, cases[3].len().try_into().unwrap()),

        op::movi(0x11, script_offset as Immediate18),
        op::movi(0x19, 0x00),
        op::gtf_args(0x10, 0x19, GTFArgs::Script),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x11, script_data_offset as Immediate18),
        op::movi(0x19, 0x00),
        op::gtf_args(0x10, 0x19, GTFArgs::ScriptData),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x11, InputRepr::Coin as Immediate18),
        op::movi(0x19, 0x00),
        op::gtf_args(0x10, 0x19, GTFArgs::InputType),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        // Skip over CoinTxId which is always zero
        op::addi(0x30, 0x30, cases[4].len().try_into().unwrap()),

        op::movi(0x11, inputs[0].utxo_id().unwrap().output_index() as Immediate18),
        op::movi(0x19, 0x00),
        op::gtf_args(0x10, 0x19, GTFArgs::InputCoinOutputIndex),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x00),
        op::gtf_args(0x10, 0x19, GTFArgs::InputCoinOwner),
        op::movi(0x11, cases[5].len() as Immediate18),
        op::meq(0x10, 0x10, 0x30, 0x11),
        op::add(0x30, 0x30, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x11, (inputs[0].amount().unwrap() & 0x3ffff) as Immediate18),
        op::movi(0x12, (inputs[0].amount().unwrap() >> 18) as Immediate18),
        op::slli(0x12, 0x12, 18),
        op::or(0x11, 0x11, 0x12),
        op::movi(0x19, 0x00),
        op::gtf_args(0x10, 0x19, GTFArgs::InputCoinAmount),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x00),
        op::gtf_args(0x10, 0x19, GTFArgs::InputCoinAssetId),
        op::movi(0x11, cases[6].len() as Immediate18),
        op::meq(0x10, 0x10, 0x30, 0x11),
        op::add(0x30, 0x30, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x11, inputs[0].witness_index().unwrap() as Immediate18),
        op::movi(0x19, 0x00),
        op::gtf_args(0x10, 0x19, GTFArgs::InputCoinWitnessIndex),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x11, predicate.len() as Immediate18),
        op::movi(0x19, 0x01),
        op::gtf_args(0x10, 0x19, GTFArgs::InputCoinPredicateLength),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x11, predicate_data.len() as Immediate18),
        op::movi(0x19, 0x01),
        op::gtf_args(0x10, 0x19, GTFArgs::InputCoinPredicateDataLength),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x01),
        op::gtf_args(0x10, 0x19, GTFArgs::InputCoinPredicate),
        op::movi(0x11, cases[7].len() as Immediate18),
        op::meq(0x10, 0x10, 0x30, 0x11),
        op::add(0x30, 0x30, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x01),
        op::gtf_args(0x10, 0x19, GTFArgs::InputCoinPredicateData),
        op::movi(0x11, cases[8].len() as Immediate18),
        op::meq(0x10, 0x10, 0x30, 0x11),
        op::add(0x30, 0x30, 0x11),
        op::and(0x20, 0x20, 0x10),



        // Skip over always-zero InputContract fields
        op::addi(0x30, 0x30, cases[9].len().try_into().unwrap()),
        op::addi(0x30, 0x30, cases[10].len().try_into().unwrap()),
        op::addi(0x30, 0x30, cases[11].len().try_into().unwrap()),

        op::movi(0x19, contract_input_index as Immediate18),
        op::gtf_args(0x10, 0x19, GTFArgs::InputContractId),
        op::movi(0x11, cases[12].len() as Immediate18),
        op::meq(0x10, 0x10, 0x30, 0x11),
        op::add(0x30, 0x30, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x03),
        op::gtf_args(0x10, 0x19, GTFArgs::InputMessageSender),
        op::movi(0x11, cases[13].len() as Immediate18),
        op::meq(0x10, 0x10, 0x30, 0x11),
        op::add(0x30, 0x30, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x03),
        op::gtf_args(0x10, 0x19, GTFArgs::InputMessageRecipient),
        op::movi(0x11, cases[14].len() as Immediate18),
        op::meq(0x10, 0x10, 0x30, 0x11),
        op::add(0x30, 0x30, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x11, message_amount as Immediate18),
        op::movi(0x19, 0x03),
        op::gtf_args(0x10, 0x19, GTFArgs::InputMessageAmount),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x03),
        op::gtf_args(0x10, 0x19, GTFArgs::InputMessageNonce),
        op::movi(0x11, cases[15].len() as Immediate18),
        op::meq(0x10, 0x10, 0x30, 0x11),
        op::add(0x30, 0x30, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x11, 0x02),
        op::movi(0x19, 0x03),
        op::gtf_args(0x10, 0x19, GTFArgs::InputMessageWitnessIndex),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x11, message_data.len() as Immediate18),
        op::movi(0x19, 0x03),
        op::gtf_args(0x10, 0x19, GTFArgs::InputMessageDataLength),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x11, m_predicate.len() as Immediate18),
        op::movi(0x19, 0x04),
        op::gtf_args(0x10, 0x19, GTFArgs::InputMessagePredicateLength),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x11, m_predicate_data.len() as Immediate18),
        op::movi(0x19, 0x04),
        op::gtf_args(0x10, 0x19, GTFArgs::InputMessagePredicateDataLength),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x04),
        op::gtf_args(0x10, 0x19, GTFArgs::InputMessageData),
        op::movi(0x11, cases[16].len() as Immediate18),
        op::meq(0x10, 0x10, 0x30, 0x11),
        op::add(0x30, 0x30, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x04),
        op::gtf_args(0x10, 0x19, GTFArgs::InputMessagePredicate),
        op::movi(0x11, cases[17].len() as Immediate18),
        op::meq(0x10, 0x10, 0x30, 0x11),
        op::add(0x30, 0x30, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x04),
        op::gtf_args(0x10, 0x19, GTFArgs::InputMessagePredicateData),
        op::movi(0x11, cases[18].len() as Immediate18),
        op::meq(0x10, 0x10, 0x30, 0x11),
        op::add(0x30, 0x30, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x04),
        op::gtf_args(0x10, 0x19, GTFArgs::InputMessagePredicateGasUsed),
        op::movi(0x11, 0 as Immediate18),
        op::meq(0x10, 0x10, 0x30, 0x11),
        op::add(0x30, 0x30, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x11, OutputRepr::Contract as Immediate18),
        op::movi(0x19, 0x01),
        op::gtf_args(0x10, 0x19, GTFArgs::OutputType),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x02),
        op::gtf_args(0x10, 0x19, GTFArgs::OutputCoinTo),
        op::movi(0x11, cases[19].len() as Immediate18),
        op::meq(0x10, 0x10, 0x30, 0x11),
        op::add(0x30, 0x30, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x11, asset_amt as Immediate18),
        op::movi(0x19, 0x02),
        op::gtf_args(0x10, 0x19, GTFArgs::OutputCoinAmount),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x02),
        op::gtf_args(0x10, 0x19, GTFArgs::OutputCoinAssetId),
        op::movi(0x11, cases[20].len() as Immediate18),
        op::meq(0x10, 0x10, 0x30, 0x11),
        op::add(0x30, 0x30, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x11, asset_amt as Immediate18),
        op::movi(0x19, 0x02),
        op::gtf_args(0x10, 0x19, GTFArgs::OutputCoinAmount),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x11, contract_input_index as Immediate18),
        op::movi(0x19, 0x01),
        op::gtf_args(0x10, 0x19, GTFArgs::OutputContractInputIndex),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        // Skip over always-zero OutputContract fields
        op::addi(0x30, 0x30, cases[21].len().try_into().unwrap()),
        op::addi(0x30, 0x30, cases[22].len().try_into().unwrap()),

        op::movi(0x11, witnesses[1].as_ref().len() as Immediate18),
        op::movi(0x19, 0x01),
        op::gtf_args(0x10, 0x19, GTFArgs::WitnessDataLength),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x01),
        op::gtf_args(0x10, 0x19, GTFArgs::WitnessData),
        op::movi(0x11, cases[23].len() as Immediate18),
        op::meq(0x10, 0x10, 0x30, 0x11),
        op::add(0x30, 0x30, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x03),
        op::gtf_args(0x10, 0x19, GTFArgs::OutputCoinAssetId),
        op::movi(0x11, cases[24].len() as Immediate18),
        op::meq(0x10, 0x10, 0x30, 0x11),
        op::add(0x30, 0x30, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x03),
        op::gtf_args(0x10, 0x19, GTFArgs::OutputCoinTo),
        op::movi(0x11, cases[25].len() as Immediate18),
        op::meq(0x10, 0x10, 0x30, 0x11),
        op::add(0x30, 0x30, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x00),
        op::movi(0x11, inputs.len() as Immediate18),
        op::gtf_args(0x10, 0x19, GTFArgs::TxInputsCount),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x00),
        op::movi(0x11, outputs.len() as Immediate18),
        op::gtf_args(0x10, 0x19, GTFArgs::TxOutputsCount),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x00),
        op::movi(0x11, witnesses.len() as Immediate18),
        op::gtf_args(0x10, 0x19, GTFArgs::TxWitnessesCount),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::gtf_args(0x30, 0x19, GTFArgs::ScriptData),
        op::movi(0x19, 0x00),
        op::gtf_args(0x10, 0x19, GTFArgs::TxInputAtIndex),
        op::movi(0x11, cases[0].len() as Immediate18),
        op::meq(0x10, 0x10, 0x30, 0x11),
        op::add(0x30, 0x30, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x00),
        op::gtf_args(0x10, 0x19, GTFArgs::TxOutputAtIndex),
        op::movi(0x11, cases[1].len() as Immediate18),
        op::meq(0x10, 0x10, 0x30, 0x11),
        op::add(0x30, 0x30, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x01),
        op::gtf_args(0x10, 0x19, GTFArgs::TxWitnessAtIndex),
        op::movi(0x11, cases[2].len() as Immediate18),
        op::meq(0x10, 0x10, 0x30, 0x11),
        op::add(0x30, 0x30, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::movi(0x19, 0x01),
        op::gtf_args(0x10, 0x19, GTFArgs::InputCoinTxPointer),
        op::movi(0x11, cases[26].len() as Immediate18),
        op::meq(0x10, 0x10, 0x30, 0x11),
        op::add(0x30, 0x30, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::log(0x20, 0x00, 0x00, 0x00),
        op::ret(0x00)
    ].into_iter().collect();

    while script.len() < script_reserved_words {
        script.extend(op::noop().to_bytes());
    }

    assert_eq!(script.len(), script_reserved_words);

    let mut builder = TransactionBuilder::script(script, script_data);

    tx.as_ref().inputs().iter().for_each(|i| {
        builder.add_input(i.clone());
    });

    tx.as_ref().outputs().iter().for_each(|o| {
        builder.add_output(o.clone());
    });

    tx.as_ref().witnesses().iter().for_each(|w| {
        builder.add_witness(w.clone());
    });

    let tx = builder
        .tip(tip)
        .maturity(maturity)
        .expiration(expiration)
        .owner(owner_idx)
        .script_gas_limit(gas_limit)
        .witness_limit(witness_limit)
        .max_fee_limit(max_fee_limit)
        .finalize_checked_basic(height);

    let receipts = client.transact(tx);
    let success = receipts
        .iter()
        .any(|r| matches!(r, Receipt::Log{ ra, .. } if ra == &1));

    assert!(success);
}

#[ignore]
#[test]
fn gtf_args__data_coin_data_length() {
    let rng = &mut StdRng::seed_from_u64(2322u64);
    let gas_costs = GasCosts::default();

    let mut client = MemoryClient::default();

    let witness_limit = 1234;
    // let max_fee_limit = 4321;
    let max_fee_limit = 0;
    let tip = 4321;
    let gas_limit = 10_000_000;
    let maturity = 50.into();
    let height = 122.into();
    let expiration = 123.into();

    let predicate = vec![op::ret(RegId::ONE)].into_iter().collect::<Vec<u8>>();
    let mut predicate_data = vec![0u8; 512];

    rng.fill(predicate_data.as_mut_slice());

    let owner = Input::predicate_owner(&predicate);
    let length = 64;
    let mut data = vec![0u8; length];
    rng.fill(data.as_mut_slice());
    let input_coin_predicate = Input::data_coin_predicate(
        rng.r#gen(),
        owner,
        1_500,
        rng.r#gen(),
        rng.r#gen(),
        gas_costs.ret(),
        predicate.clone(),
        predicate_data.clone(),
        data,
    );

    // let tx = TransactionBuilder::script(vec![], vec![])
    //     .maturity(maturity)
    //     .expiration(expiration)
    //     .with_gas_costs(gas_costs)
    //     .script_gas_limit(gas_limit)
    //     .add_input(input_coin_predicate)
    //     .finalize_checked(height);

    // let script_data: Vec<u8> = cases.iter().flat_map(|c| c.iter()).copied().collect();
    let script_data = vec![];

    #[rustfmt::skip]
    let mut script: Vec<_> = vec![
        op::movi(0x20, 0x01),

        // op::movi(0x19, 0x01),
        // op::gtf_args(0x10, 0x19, GTFArgs::InputCoinPredicateData),
        // op::movi(0x11, predicate_data.len() as Immediate18),
        // op::meq(0x20, 0x10, 0x30, 0x11),
        // // op::add(0x30, 0x30, 0x11),
        // // op::and(0x20, 0x20, 0x10),
        // op::log(0x20, 0x00, 0x00, 0x00),
        // op::ret(0x00),
        // op::movi(0x11, predicate_data.len() as Immediate18),
        // op::movi(0x19, 0x01),
        // op::gtf_args(0x10, 0x19, GTFArgs::InputCoinPredicateDataLength),
        // op::eq(0x10, 0x10, 0x11),
        // op::and(0x20, 0x20, 0x10),

        // op::movi(0x19, 0x01),
        // op::gtf_args(0x10, 0x19, GTFArgs::InputCoinPredicateData),
        // op::movi(0x11, predicate_data.len() as Immediate18),
        // op::meq(0x10, 0x10, 0x30, 0x11),
        // op::add(0x30, 0x30, 0x11),
        // op::and(0x20, 0x20, 0x10),
        op::log(0x20, 0x00, 0x00, 0x00),
        op::ret(0x00),
    ]
    .into_iter()
    .collect();

    const SCRIPT_RESERVED_WORDS: usize = 300 * WORD_SIZE;

    while script.len() < SCRIPT_RESERVED_WORDS {
        script.extend(op::noop().to_bytes());
    }

    assert_eq!(script.len(), SCRIPT_RESERVED_WORDS);

    let mut builder = TransactionBuilder::script(script.clone(), script_data);

    builder.add_input(input_coin_predicate);

    let tx = builder
        .tip(tip)
        .maturity(maturity)
        .expiration(expiration)
        .script_gas_limit(gas_limit)
        .witness_limit(witness_limit)
        .max_fee_limit(max_fee_limit)
        .finalize_checked_basic(height);

    let receipts = client.transact(tx);
    dbg!(&receipts);
    let success = receipts
        .iter()
        .any(|r| matches!(r, Receipt::Log{ ra, .. } if ra == &1));

    assert!(success);
}

#[test]
fn get_owner_metadata__two_different_owners__policy_set() {
    let rng = &mut StdRng::seed_from_u64(2322u64);
    let gas_costs = GasCosts::default();

    let mut client = MemoryClient::default();

    let predicate_1 = vec![op::ret(RegId::ONE)].into_iter().collect::<Vec<u8>>();

    let owner_1 = Input::predicate_owner(&predicate_1);
    let input_coin_predicate_1 = Input::coin_predicate(
        rng.r#gen(),
        owner_1,
        100_000_000,
        AssetId::BASE,
        rng.r#gen(),
        gas_costs.ret(),
        predicate_1.clone(),
        vec![],
    );

    let predicate_2 = vec![op::ret(RegId::ONE), op::ret(RegId::ONE)]
        .into_iter()
        .collect::<Vec<u8>>();

    let owner_2 = Input::predicate_owner(&predicate_2);
    let input_coin_predicate_2 = Input::coin_predicate(
        rng.r#gen(),
        owner_2,
        100_000_000,
        AssetId::BASE,
        rng.r#gen(),
        gas_costs.ret(),
        predicate_2.clone(),
        vec![],
    );
    // Set second predicate as an owner
    let owner_idx = 1;

    assert_ne!(predicate_2, predicate_1);

    #[rustfmt::skip]
    let script: Vec<u8> = vec![
        op::movi(0x20, 0x01),
        op::gm_args(0x10, GMArgs::GetOwner),

        op::movi(0x19, owner_idx as u32),
        op::gtf_args(0x11, 0x19, GTFArgs::InputCoinOwner),

        op::movi(0x13, 32),
        op::meq(0x12, 0x10, 0x11, 0x13),

        op::log(0x12, 0x00, 0x00, 0x00),
        op::ret(0x00)
    ].into_iter().collect();

    let tx = TransactionBuilder::script(script, vec![])
        // Given
        .owner(owner_idx)
        .max_fee_limit(1_000_000)
        .script_gas_limit(1_000_000)
        .with_gas_costs(gas_costs)
        .add_input(input_coin_predicate_1)
        .add_input(input_coin_predicate_2)
        .finalize_checked(1u32.into());

    // When
    let receipts = client.transact(tx);

    // Then
    let success = receipts
        .iter()
        .any(|r| matches!(r, Receipt::Log{ ra, .. } if ra == &1));
    assert!(success);
}

#[test]
fn get_owner_metadata__two_different_owners__policy_not_set_causes_panic() {
    let rng = &mut StdRng::seed_from_u64(2322u64);
    let gas_costs = GasCosts::default();

    let mut client = MemoryClient::default();

    let predicate_1 = vec![op::ret(RegId::ONE)].into_iter().collect::<Vec<u8>>();

    let owner_1 = Input::predicate_owner(&predicate_1);
    let input_coin_predicate_1 = Input::coin_predicate(
        rng.r#gen(),
        owner_1,
        100_000_000,
        AssetId::BASE,
        rng.r#gen(),
        gas_costs.ret(),
        predicate_1.clone(),
        vec![],
    );

    let predicate_2 = vec![op::ret(RegId::ONE), op::ret(RegId::ONE)]
        .into_iter()
        .collect::<Vec<u8>>();

    let owner_2 = Input::predicate_owner(&predicate_2);
    let input_coin_predicate_2 = Input::coin_predicate(
        rng.r#gen(),
        owner_2,
        100_000_000,
        AssetId::BASE,
        rng.r#gen(),
        gas_costs.ret(),
        predicate_2.clone(),
        vec![],
    );
    // Set second predicate as an owner
    let owner_idx = 1u32;

    assert_ne!(predicate_2, predicate_1);

    #[rustfmt::skip]
    let script: Vec<u8> = vec![
        op::movi(0x20, 0x01),
        op::gm_args(0x10, GMArgs::GetOwner),

        op::movi(0x19, owner_idx),
        op::gtf_args(0x11, 0x19, GTFArgs::InputCoinOwner),

        op::movi(0x13, 32),
        op::meq(0x12, 0x10, 0x11, 0x13),

        op::log(0x12, 0x00, 0x00, 0x00),
        op::ret(0x00)
    ].into_iter().collect();

    // Given
    let tx = TransactionBuilder::script(script, vec![])
        .max_fee_limit(1_000_000)
        .script_gas_limit(1_000_000)
        .with_gas_costs(gas_costs)
        .add_input(input_coin_predicate_1)
        .add_input(input_coin_predicate_2)
        .finalize_checked(1u32.into());

    // When
    let receipts = client.transact(tx);

    // Then
    let panic = receipts.iter().any(|r| matches!(r, Receipt::Panic { .. }));
    assert!(panic);
}

#[test]
fn get_owner_metadata__two_same_owners__policy_not_set_returns_owner() {
    let rng = &mut StdRng::seed_from_u64(2322u64);
    let gas_costs = GasCosts::default();

    let mut client = MemoryClient::default();

    let predicate_1 = vec![op::ret(RegId::ONE)].into_iter().collect::<Vec<u8>>();

    let owner_1 = Input::predicate_owner(&predicate_1);
    let input_coin_predicate_1 = Input::coin_predicate(
        rng.r#gen(),
        owner_1,
        100_000_000,
        AssetId::BASE,
        rng.r#gen(),
        gas_costs.ret(),
        predicate_1.clone(),
        vec![],
    );

    let input_coin_predicate_2 = Input::coin_predicate(
        rng.r#gen(),
        owner_1,
        100_000_000,
        AssetId::BASE,
        rng.r#gen(),
        gas_costs.ret(),
        predicate_1.clone(),
        vec![],
    );
    let owner_idx = 0u32;

    #[rustfmt::skip]
    let script: Vec<u8> = vec![
        op::movi(0x20, 0x01),
        op::gm_args(0x10, GMArgs::GetOwner),

        op::movi(0x19, owner_idx),
        op::gtf_args(0x11, 0x19, GTFArgs::InputCoinOwner),

        op::movi(0x13, 32),
        op::meq(0x12, 0x10, 0x11, 0x13),

        op::log(0x12, 0x00, 0x00, 0x00),
        op::ret(0x00)
    ].into_iter().collect();

    // Given
    let tx = TransactionBuilder::script(script, vec![])
        .max_fee_limit(1_000_000)
        .script_gas_limit(1_000_000)
        .with_gas_costs(gas_costs)
        .add_input(input_coin_predicate_1)
        .add_input(input_coin_predicate_2)
        .finalize_checked(1u32.into());

    // When
    let receipts = client.transact(tx);

    // Then
    let success = receipts
        .iter()
        .any(|r| matches!(r, Receipt::Log{ ra, .. } if ra == &1));
    assert!(success);
}

#[test]
fn get__create_specific_transaction_fields__success() {
    const PREDICATE_COUNT: u64 = 1;
    const MAX_PREDICATE_GAS: u64 = 1_000_000;
    const MAX_TX_GAS: u64 = MAX_PREDICATE_GAS * PREDICATE_COUNT;
    let rng = &mut StdRng::seed_from_u64(8586);
    let mut client = MemoryClient::default();

    // Given
    let salt = Salt::new([1; 32]);
    let storage_slots = vec![
        StorageSlot::new(Bytes32::new([0; 32]), Bytes32::new([0; 32])),
        StorageSlot::new(Bytes32::new([2; 32]), Bytes32::new([3; 32])),
    ];
    // Write the elements we want to check into the bytecode
    // so that they are available in the predicate memory
    // doesn't work for the contract_id because changing the bytecode would change the
    // contract_id
    let mut bytecode = Vec::new();
    bytecode.extend(salt.to_bytes());
    bytecode.extend(storage_slots[1].to_bytes());
    bytecode.extend(Contract::initial_state_root(storage_slots.iter()).iter());
    let mut tx = TransactionBuilder::create(bytecode.into(), salt, storage_slots);
    tx.add_fee_input();
    tx.add_contract_created();

    let instructions_contract_id = alloc_bytearray::<32>(
        0x11,
        hex::decode("54dcc5dbe7dc3ff267b6a9147c358a38f8b7ac61160769458fdcf53f751be37f")
            .unwrap()
            .try_into()
            .unwrap(),
    );
    // When
    #[rustfmt::skip]
    let mut predicate_code = vec![
        op::gtf_args(0x10, 0x00, GTFArgs::CreateBytecodeWitnessIndex),
        op::movi(0x11, 0x00),
        op::eq(0x20, 0x10, 0x11),

        // Store the bytecode pointer for later use
        op::gtf_args(0x13, 0x10, GTFArgs::TxWitnessAtIndex),
        // Skip the length of the bytecode
        op::addi(0x13, 0x13, 0x08),

        op::gtf_args(0x10, 0x00, GTFArgs::CreateStorageSlotsCount),
        op::movi(0x11, 0x02),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::gtf_args(0x10, 0x00, GTFArgs::CreateSalt),
        op::movi(0x11, 0x20),
        // Salt is at the start of the bytecode which start at value stored in 0x13
        op::meq(0x10, 0x10, 0x13, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::gtf_args(0x10, 0x01, GTFArgs::CreateStorageSlotAtIndex),
        op::movi(0x11, StorageSlot::SLOT_SIZE as Immediate18),
        // Increase bytecode pointer by 32 bytes to pass the salt
        op::addi(0x13, 0x13, 0x20),
        op::meq(0x10, 0x10, 0x13, 0x11),
        op::and(0x20, 0x20, 0x10),

        op::gtf_args(0x10, 0x00, GTFArgs::OutputContractCreatedStateRoot),
        op::movi(0x11, 0x20),
        // Increase bytecode pointer by SLOT_SIZE bytes to pass the storage slot written
        // in bytecode
        op::addi(0x13, 0x13, StorageSlot::SLOT_SIZE as Immediate12),
        op::meq(0x10, 0x10, 0x13, 0x11),
        op::and(0x20, 0x20, 0x10),

    ];

    predicate_code.extend(instructions_contract_id);

    predicate_code.extend(vec![
        op::gtf_args(0x10, 0x00, GTFArgs::OutputContractCreatedContractId),
        op::movi(0x12, 0x20),
        // instructions_contract_id saved the value in a memory starting at value of
        // `0x11`
        op::meq(0x10, 0x10, 0x11, 0x12),
        op::and(0x20, 0x20, 0x10),
        op::ret(0x20),
    ]);

    let predicate_code = predicate_code.into_iter().collect();

    let predicate_owner: Address = Input::predicate_owner(&predicate_code);
    tx.add_input(Input::coin_predicate(
        rng.r#gen(),
        predicate_owner,
        rng.r#gen(),
        *tx.get_params().base_asset_id(),
        rng.r#gen(),
        0,
        predicate_code,
        vec![],
    ));
    let tx_param = TxParameters::default().with_max_gas_per_tx(MAX_TX_GAS);
    let mut tx = tx.finalize();
    let gas_costs = GasCosts::new(GasCostsValuesV5::free().into());
    let predicate_param =
        PredicateParameters::default().with_max_gas_per_predicate(MAX_PREDICATE_GAS);
    let fee_param = FeeParameters::default().with_gas_per_byte(0);

    let mut consensus_params = ConsensusParameters::standard();
    consensus_params.set_gas_costs(gas_costs.clone());
    consensus_params.set_predicate_params(predicate_param);
    consensus_params.set_tx_params(tx_param);
    consensus_params.set_fee_params(fee_param);
    tx.estimate_predicates(
        &consensus_params.clone().into(),
        MemoryInstance::new(),
        &EmptyStorage,
    )
    .unwrap();

    let tx = tx
        .into_checked(BlockHeight::new(0), &consensus_params)
        .unwrap();

    // Then
    client.deploy(tx).unwrap();
}

#[test]
fn get__upload_specific_transaction_fields__success() {
    const PREDICATE_COUNT: u64 = 1;
    const MAX_PREDICATE_GAS: u64 = 1_000_000;
    const MAX_TX_GAS: u64 = MAX_PREDICATE_GAS * PREDICATE_COUNT;
    let rng = &mut StdRng::seed_from_u64(8586);
    let mut client = MemoryClient::default();

    // Given
    let subsection = UploadSubsection::split_bytecode(&[1; 10], 1).unwrap()[0].clone();
    let mut tx = TransactionBuilder::upload(UploadBody {
        root: subsection.root,
        witness_index: 0,
        subsection_index: subsection.subsection_index,
        subsections_number: subsection.subsections_number,
        proof_set: subsection.proof_set.clone(),
    });
    tx.add_witness(subsection.subsection.into());
    tx.add_fee_input();

    let instructions_root = alloc_bytearray(0x11, subsection.root.into());
    let instructions_proof = alloc_bytearray(0x11, subsection.proof_set[1].into());
    // When
    #[rustfmt::skip]
    let mut predicate_code = vec![
        op::gtf_args(0x10, 0x00, GTFArgs::UploadRoot),
    ];
    predicate_code.extend(instructions_root);
    predicate_code.extend(vec![
        op::meq(0x20, 0x10, 0x11, 0x20),
        op::gtf_args(0x10, 0x00, GTFArgs::UploadWitnessIndex),
        op::movi(0x11, 0x00),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),
        op::gtf_args(0x10, 0x00, GTFArgs::UploadSubsectionIndex),
        op::movi(0x11, subsection.subsection_index as u32),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),
        op::gtf_args(0x10, 0x00, GTFArgs::UploadSubsectionsCount),
        op::movi(0x11, subsection.subsections_number as u32),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),
        op::gtf_args(0x10, 0x00, GTFArgs::UploadProofSetCount),
        op::movi(0x11, subsection.proof_set.len() as u32),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),
        op::gtf_args(0x10, 0x01, GTFArgs::UploadProofSetAtIndex),
    ]);
    predicate_code.extend(instructions_proof);
    predicate_code.extend(vec![op::meq(0x20, 0x10, 0x11, 0x20), op::ret(0x20)]);

    let predicate_code = predicate_code.into_iter().collect();

    let predicate_owner: Address = Input::predicate_owner(&predicate_code);
    tx.add_input(Input::coin_predicate(
        rng.r#gen(),
        predicate_owner,
        rng.r#gen(),
        *tx.get_params().base_asset_id(),
        rng.r#gen(),
        0,
        predicate_code,
        vec![],
    ));
    let tx_param = TxParameters::default().with_max_gas_per_tx(MAX_TX_GAS);
    let mut tx = tx.finalize();
    let gas_costs = GasCosts::new(GasCostsValuesV5::free().into());
    let predicate_param =
        PredicateParameters::default().with_max_gas_per_predicate(MAX_PREDICATE_GAS);
    let fee_param = FeeParameters::default().with_gas_per_byte(0);

    let mut consensus_params = ConsensusParameters::standard();
    consensus_params.set_gas_costs(gas_costs.clone());
    consensus_params.set_predicate_params(predicate_param);
    consensus_params.set_tx_params(tx_param);
    consensus_params.set_fee_params(fee_param);
    tx.estimate_predicates(
        &consensus_params.clone().into(),
        MemoryInstance::new(),
        &EmptyStorage,
    )
    .unwrap();

    let tx = tx
        .into_checked(BlockHeight::new(0), &consensus_params)
        .unwrap();

    // Then
    client.upload(tx).unwrap();
}

#[test]
fn get__blob_specific_transaction_fields__success() {
    const PREDICATE_COUNT: u64 = 1;
    const MAX_PREDICATE_GAS: u64 = 1_000_000;
    const MAX_TX_GAS: u64 = MAX_PREDICATE_GAS * PREDICATE_COUNT;
    let rng = &mut StdRng::seed_from_u64(8586);
    let mut client = MemoryClient::default();

    // Given
    let id = BlobId::compute(&[1; 100]);
    let mut tx = TransactionBuilder::blob(BlobBody {
        id,
        witness_index: 0,
    });
    tx.add_witness(vec![1; 100].into());
    tx.add_fee_input();

    // When
    let blob_instructions = alloc_bytearray(0x11, id.into());
    #[rustfmt::skip]
    let mut predicate_code = vec![
        op::gtf_args(0x10, 0x00, GTFArgs::BlobId),
    ];
    predicate_code.extend(blob_instructions);
    predicate_code.extend(vec![
        op::meq(0x20, 0x10, 0x11, 0x20),
        op::gtf_args(0x10, 0x00, GTFArgs::BlobWitnessIndex),
        op::movi(0x11, 0x00),
        op::eq(0x10, 0x10, 0x11),
        op::and(0x20, 0x20, 0x10),
        op::ret(0x20),
    ]);

    let predicate_code = predicate_code.into_iter().collect();

    let predicate_owner: Address = Input::predicate_owner(&predicate_code);
    tx.add_input(Input::coin_predicate(
        rng.r#gen(),
        predicate_owner,
        rng.r#gen(),
        *tx.get_params().base_asset_id(),
        rng.r#gen(),
        0,
        predicate_code,
        vec![],
    ));
    let tx_param = TxParameters::default().with_max_gas_per_tx(MAX_TX_GAS);
    let mut tx = tx.finalize();
    let gas_costs = GasCosts::new(GasCostsValuesV5::free().into());
    let predicate_param =
        PredicateParameters::default().with_max_gas_per_predicate(MAX_PREDICATE_GAS);
    let fee_param = FeeParameters::default().with_gas_per_byte(0);

    let mut consensus_params = ConsensusParameters::standard();
    consensus_params.set_gas_costs(gas_costs.clone());
    consensus_params.set_predicate_params(predicate_param);
    consensus_params.set_tx_params(tx_param);
    consensus_params.set_fee_params(fee_param);
    tx.estimate_predicates(
        &consensus_params.clone().into(),
        MemoryInstance::new(),
        &EmptyStorage,
    )
    .unwrap();

    let tx = tx
        .into_checked(BlockHeight::new(0), &consensus_params)
        .unwrap();

    // Then
    client.blob(tx).unwrap();
}

fn valid_storage(hash: Bytes32, bytecode: Vec<u8>) -> MemoryStorage {
    let mut storage = MemoryStorage::default();
    storage.set_state_transition_version(123);
    storage
        .state_transition_bytecodes_mut()
        .insert(hash, UploadedBytecode::Completed(bytecode));

    storage
}

#[test]
fn get__upgrade_specific_transaction_fields__success() {
    const PREDICATE_COUNT: u64 = 1;
    const MAX_PREDICATE_GAS: u64 = 1_000_000;
    const MAX_TX_GAS: u64 = MAX_PREDICATE_GAS * PREDICATE_COUNT;
    let rng = &mut StdRng::seed_from_u64(8586);

    // Given
    let root = Bytes32::from([1; 32]);
    let mut client: MemoryClient<MemoryInstance> = MemoryClient::new(
        MemoryInstance::default(),
        valid_storage(root, vec![]),
        InterpreterParams::default(),
    );
    let mut tx = TransactionBuilder::upgrade(UpgradePurpose::StateTransition { root });
    tx.add_fee_input();

    // When
    let state_transition_instructions = alloc_bytearray(0x11, root.into());
    #[rustfmt::skip]
    let mut predicate_code = vec![
        op::gtf_args(0x10, 0x00, GTFArgs::UpgradePurpose),
    ];
    predicate_code.extend(state_transition_instructions);
    predicate_code.extend(vec![op::meq(0x20, 0x10, 0x11, 0x20), op::ret(0x20)]);

    let predicate_code = predicate_code.into_iter().collect();

    let predicate_owner: Address = Input::predicate_owner(&predicate_code);
    tx.add_input(Input::coin_predicate(
        rng.r#gen(),
        predicate_owner,
        rng.r#gen(),
        *tx.get_params().base_asset_id(),
        rng.r#gen(),
        0,
        predicate_code,
        vec![],
    ));
    let tx_param = TxParameters::default().with_max_gas_per_tx(MAX_TX_GAS);
    let mut tx = tx.finalize();
    let gas_costs = GasCosts::new(GasCostsValuesV5::free().into());
    let predicate_param =
        PredicateParameters::default().with_max_gas_per_predicate(MAX_PREDICATE_GAS);
    let fee_param = FeeParameters::default().with_gas_per_byte(0);

    let mut consensus_params = ConsensusParameters::standard();
    consensus_params.set_gas_costs(gas_costs.clone());
    consensus_params.set_predicate_params(predicate_param);
    consensus_params.set_tx_params(tx_param);
    consensus_params.set_fee_params(fee_param);
    consensus_params.set_privileged_address(predicate_owner);
    tx.estimate_predicates(
        &consensus_params.clone().into(),
        MemoryInstance::new(),
        &EmptyStorage,
    )
    .unwrap();

    let tx = tx
        .into_checked(BlockHeight::new(0), &consensus_params)
        .unwrap();

    // Then
    client.upgrade(tx).unwrap();
}
