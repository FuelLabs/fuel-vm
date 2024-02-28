use alloc::{
    borrow::ToOwned,
    vec,
    vec::Vec,
};

use crate::{
    consts::*,
    interpreter::{
        InterpreterParams,
        NotSupportedEcal,
    },
};
use fuel_asm::{
    op,
    GMArgs,
    GTFArgs,
    RegId,
};
use fuel_crypto::Hasher;
use fuel_tx::{
    field::{
        Inputs,
        Outputs,
        ReceiptsRoot,
        Script as ScriptField,
        Witnesses,
    },
    policies::PoliciesBits,
    ConsensusParameters,
    Finalizable,
    Receipt,
    Script,
    TransactionBuilder,
};
use fuel_types::{
    bytes,
    canonical::Serialize,
    BlockHeight,
    ChainId,
};
use rand::{
    rngs::StdRng,
    Rng,
    SeedableRng,
};

use crate::prelude::{
    GasCosts,
    *,
};
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
        op::movi(0x20,  ContractId::LEN as Immediate18),
        op::logd(0x00, 0x00, 0x11, 0x20),
        op::ret(RegId::ONE),
    ];

    let salt: Salt = rng.gen();
    let program: Witness = routine_metadata_is_caller_external
        .into_iter()
        .collect::<Vec<u8>>()
        .into();

    let contract = Contract::from(program.as_ref());
    let contract_root = contract.root();
    let state_root = Contract::default_state_root();
    let contract_metadata = contract.id(&salt, &contract_root, &state_root);
    let output = Output::contract_created(contract_metadata, state_root);

    let tx = TransactionBuilder::create(program, salt, vec![])
        .maturity(maturity)
        .add_random_fee_input()
        .add_output(output)
        .finalize()
        .into_checked(height, &consensus_params, gas_price)
        .expect("failed to check tx");

    let interpreter_params = InterpreterParams::new(gas_price, &consensus_params);

    // Deploy the contract into the blockchain
    assert!(
        Transactor::<_, _>::new(&mut storage, interpreter_params.clone())
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

    let salt: Salt = rng.gen();
    let program: Witness = routine_call_metadata_contract
        .into_iter()
        .collect::<Vec<u8>>()
        .into();

    let contract = Contract::from(program.as_ref());
    let contract_root = contract.root();
    let state_root = Contract::default_state_root();
    let contract_call = contract.id(&salt, &contract_root, &state_root);

    let output = Output::contract_created(contract_call, state_root);

    let tx = TransactionBuilder::create(program, salt, vec![])
        .maturity(maturity)
        .add_random_fee_input()
        .add_output(output)
        .finalize()
        .into_checked(height, &consensus_params, gas_price)
        .expect("failed to check tx");

    assert!(
        Transactor::<_, _>::new(&mut storage, interpreter_params.clone())
            .transact(tx)
            .is_success()
    );

    let mut inputs = vec![];
    let mut outputs = vec![];

    inputs.push(Input::contract(
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        contract_call,
    ));
    outputs.push(Output::contract(0, rng.gen(), rng.gen()));

    inputs.push(Input::contract(
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        contract_metadata,
    ));
    outputs.push(Output::contract(1, rng.gen(), rng.gen()));

    let mut script = vec![
        op::movi(0x10, (1 + Bytes32::LEN + 2 * Bytes8::LEN) as Immediate18),
        op::aloc(0x10),
        op::move_(0x10, RegId::HP),
    ];

    contract_call
        .as_ref()
        .iter()
        .enumerate()
        .for_each(|(i, b)| {
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
        .add_output(outputs[0])
        .add_output(outputs[1])
        .add_random_fee_input()
        .finalize()
        .into_checked(height, &consensus_params, gas_price)
        .expect("failed to check tx");

    let receipts = Transactor::<_, _>::new(&mut storage, interpreter_params)
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

    let contract_call = Hasher::hash(contract_call.as_ref());
    let digest = receipts[4]
        .digest()
        .expect("GetCaller should return contract Id");
    assert_eq!(&contract_call, digest);
}

#[test]
fn get_metadata_chain_id() {
    let rng = &mut StdRng::seed_from_u64(2322u64);
    let gas_limit = 1_000_000;
    let zero_gas_price = 0;
    let height = BlockHeight::default();

    let chain_id: ChainId = rng.gen();

    let interpreter_params = InterpreterParams {
        chain_id,
        ..Default::default()
    };

    let mut client =
        MemoryClient::<NotSupportedEcal>::new(Default::default(), interpreter_params);

    #[rustfmt::skip]
        let get_chain_id = vec![
        op::gm_args(0x10, GMArgs::GetChainId),
        op::ret(0x10),
    ];

    let consensus_params = ConsensusParameters::standard_with_id(chain_id);

    let script = TransactionBuilder::script(get_chain_id.into_iter().collect(), vec![])
        .script_gas_limit(gas_limit)
        .with_chain_id(chain_id)
        .add_random_fee_input()
        .finalize()
        .into_checked(height, &consensus_params, zero_gas_price)
        .unwrap();

    let receipts = client.transact(script);

    if let Receipt::Return { val, .. } = receipts[0].clone() {
        assert_eq!(val, *chain_id);
    } else {
        panic!("expected return receipt, instead of {:?}", receipts[0])
    }
}

#[test]
fn get_transaction_fields() {
    let rng = &mut StdRng::seed_from_u64(2322u64);
    let gas_costs = GasCosts::default();

    let mut client = MemoryClient::default();

    let zero_gas_price = 0;
    let witness_limit = 1234;
    let max_fee_limit = 4321;
    let tip = 4321;
    let gas_limit = 10_000_000;
    let maturity = 50.into();
    let height = 122.into();
    let input = 10_000_000;

    let tx_params = TxParameters::default();

    let contract: Witness = vec![op::ret(0x01)].into_iter().collect::<Vec<u8>>().into();
    let salt = rng.gen();
    let code_root = Contract::root_from_code(contract.as_ref());
    let storage_slots = vec![];
    let state_root = Contract::initial_state_root(storage_slots.iter());
    let contract_id =
        Contract::from(contract.as_ref()).id(&salt, &code_root, &state_root);

    let tx = TransactionBuilder::create(contract, salt, storage_slots)
        .add_output(Output::contract_created(contract_id, state_root))
        .add_random_fee_input()
        .finalize_checked(height, zero_gas_price);

    client.deploy(tx);

    let predicate = vec![op::ret(RegId::ONE)].into_iter().collect::<Vec<u8>>();
    let mut predicate_data = vec![0u8; 512];

    rng.fill(predicate_data.as_mut_slice());

    let owner = Input::predicate_owner(&predicate);
    let input_coin_predicate = Input::coin_predicate(
        rng.gen(),
        owner,
        1_500,
        rng.gen(),
        rng.gen(),
        gas_costs.ret,
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
        rng.gen(),
        owner,
        7_500,
        rng.gen(),
        gas_costs.ret,
        m_data.clone(),
        m_predicate.clone(),
        m_predicate_data.clone(),
    );

    let asset = rng.gen();
    let asset_amt = 27;

    let tx = TransactionBuilder::script(vec![], vec![])
        .maturity(maturity)
        .with_gas_costs(gas_costs)
        .script_gas_limit(gas_limit)
        .add_unsigned_coin_input(
            SecretKey::random(rng),
            rng.gen(),
            input,
            AssetId::zeroed(),
            rng.gen(),
        )
        .add_input(input_coin_predicate)
        .add_input(Input::contract(
            rng.gen(),
            rng.gen(),
            state_root,
            rng.gen(),
            contract_id,
        ))
        .add_output(Output::variable(rng.gen(), rng.gen(), rng.gen()))
        .add_output(Output::contract(
            contract_input_index,
            rng.gen(),
            state_root,
        ))
        .add_witness(Witness::from(b"some-data".to_vec()))
        .add_unsigned_message_input(
            SecretKey::random(rng),
            rng.gen(),
            rng.gen(),
            message_amount,
            message_data.clone(),
        )
        .add_input(message_predicate)
        .add_unsigned_coin_input(
            SecretKey::random(rng),
            rng.gen(),
            asset_amt,
            asset,
            rng.gen(),
        )
        .add_output(Output::coin(rng.gen(), asset_amt, asset))
        .add_output(Output::change(rng.gen(), rng.gen_range(10..1000), asset))
        .finalize_checked(height, zero_gas_price);

    let inputs = tx.as_ref().inputs();
    let outputs = tx.as_ref().outputs();
    let witnesses = tx.as_ref().witnesses();

    let inputs_bytes: Vec<Vec<u8>> = inputs
        .iter()
        .map(|v| {
            let mut v = v.clone();
            v.prepare_init_execute();
            v.clone().to_bytes()
        })
        .collect();
    let outputs_bytes: Vec<Vec<u8>> = outputs
        .iter()
        .map(|v| {
            let mut v = *v;
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
    ];

    // hardcoded metadata of script len so it can be checked at runtime
    let script_reserved_words = 300 * WORD_SIZE;
    let script_offset = tx_params.tx_offset() + Script::script_offset_static();
    let script_data_offset =
        script_offset + bytes::padded_len_usize(script_reserved_words);
    let script_data: Vec<u8> = cases.iter().flat_map(|c| c.iter()).copied().collect();

    // Maybe use predicates to check create context?
    // TODO GTFArgs::CreateBytecodeLength
    // TODO GTFArgs::CreateBytecodeWitnessIndex
    // TODO GTFArgs::CreateStorageSlotsCount
    // TODO GTFArgs::CreateSalt
    // TODO GTFArgs::CreateStorageSlotAtIndex
    // TODO GTFArgs::OutputContractCreatedContractId
    // TODO GTFArgs::OutputContractCreatedStateRoot

    // blocked by https://github.com/FuelLabs/fuel-vm/issues/59
    // TODO GTFArgs::InputCoinTxPointer
    // TODO GTFArgs::InputContractTxPointer

    #[rustfmt::skip]
    let mut script: Vec<u8> = vec![
        op::movi(0x20, 0x01),
        op::gtf_args(0x30, 0x19, GTFArgs::ScriptData),

        op::movi(0x19, 0x00),
        op::movi(0x11, TransactionRepr::Script as Immediate18),
        op::gtf_args(0x10, 0x19, GTFArgs::Type),
        op::eq(0x10, 0x10, 0x11),
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
        builder.add_output(*o);
    });

    tx.as_ref().witnesses().iter().for_each(|w| {
        builder.add_witness(w.clone());
    });

    let tx = builder
        .tip(tip)
        .maturity(maturity)
        .script_gas_limit(gas_limit)
        .witness_limit(witness_limit)
        .max_fee_limit(max_fee_limit)
        .finalize_checked_basic(height, zero_gas_price);

    let receipts = client.transact(tx);
    let success = receipts
        .iter()
        .any(|r| matches!(r, Receipt::Log{ ra, .. } if ra == &1));

    assert!(success);
}
