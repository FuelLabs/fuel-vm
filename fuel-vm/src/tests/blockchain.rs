#![allow(non_snake_case)]

use crate::{
    consts::*,
    interpreter::{
        InterpreterParams,
        NotSupportedEcal,
    },
    prelude::*,
    script_with_data_offset,
    storage::ContractsStateData,
    util::test_helpers::{
        check_expected_reason_for_instructions,
        check_expected_reason_for_instructions_with_client,
    },
};
use alloc::{
    vec,
    vec::Vec,
};
use fuel_asm::{
    op,
    Instruction,
    PanicReason::{
        ContractMaxSize,
        ContractNotInInputs,
        ExpectedUnallocatedStack,
        MemoryOverflow,
    },
    RegId,
};
use fuel_crypto::{
    Hasher,
    SecretKey,
};
use fuel_tx::{
    field::{
        Outputs,
        Script as ScriptField,
    },
    ConsensusParameters,
    Finalizable,
    Input,
    Output,
    Receipt,
    TransactionBuilder,
};
use fuel_types::{
    canonical::Serialize,
    AssetId,
    BlockHeight,
    ChainId,
};
use itertools::Itertools;
use rand::{
    rngs::StdRng,
    CryptoRng,
    Rng,
    SeedableRng,
};

fn deploy_contract(
    client: &mut MemoryClient,
    contract: Witness,
    salt: Salt,
    storage_slots: Vec<StorageSlot>,
) {
    let code_root = Contract::root_from_code(contract.as_ref());
    let state_root = Contract::initial_state_root(storage_slots.iter());
    let contract_id =
        Contract::from(contract.as_ref()).id(&salt, &code_root, &state_root);

    let tx_params = TxParameters::default();
    let height = Default::default();
    let contract_deployer = TransactionBuilder::create(contract, salt, storage_slots)
        .with_tx_params(tx_params)
        .add_output(Output::contract_created(contract_id, state_root))
        .add_random_fee_input()
        .finalize_checked(height);

    client
        .deploy(contract_deployer)
        .expect("valid contract deployment");
}

fn write_contract_id(
    script: &mut Vec<Instruction>,
    register: u8,
    contract_id: ContractId,
) {
    const COUNT: Immediate12 = ContractId::LEN as Immediate12;
    script.extend([op::ori(register, register, COUNT), op::aloc(register)]);
    for (i, byte) in contract_id.as_ref().iter().enumerate() {
        let index = i as Immediate12;
        let value = *byte as Immediate12;
        script.extend([
            op::movi(register, value.into()),
            op::sb(RegId::HP, register, index),
        ]);
    }
    script.push(op::move_(register, RegId::HP));
}

const SET_STATUS_REG: u8 = 0x39;
// log2(VM_MAX_MEM) - used to set a pointer to the memory boundary via SHL:
// 1<<log2(VM_MAX_MEM)
const MAX_MEM_SHL: Immediate12 = 26 as Immediate12;

#[test]
fn state_read_write() {
    let mut test_context = TestBuilder::new(2322u64);
    let gas_limit = 1_000_000;

    // Create a program with two main routines
    //
    // 0 - Fetch (key, value) from call[b] and state[key] += value
    //
    // 1 - Fetch (key, value) from call[b], unpack b into 4x16, xor the limbs of
    // state[key] with unpacked b

    #[rustfmt::skip]
    let function_selector = vec![
        op::move_(0x30, RegId::ZERO),
        op::move_(0x31, RegId::ONE),
    ];

    #[rustfmt::skip]
    let call_arguments_parser = vec![
        op::addi(0x10, RegId::FP, CallFrame::a_offset() as Immediate12),
        op::lw(0x10, 0x10, 0),
        op::addi(0x11, RegId::FP, CallFrame::b_offset() as Immediate12),
        op::lw(0x11, 0x11, 0),
    ];

    #[rustfmt::skip]
    let routine_add_word_to_state = vec![
        op::jnei(0x10, 0x30, 13),               // (0, b) Add word to state
        op::lw(0x20, 0x11, 4),                  // r[0x20]      := m[b+32, 8]
        op::srw(0x21, SET_STATUS_REG, 0x11),    // r[0x21]      := s[m[b, 32], 8]
        op::add(0x20, 0x20, 0x21),              // r[0x20]      += r[0x21]
        op::sww(0x11, SET_STATUS_REG, 0x20),    // s[m[b,32]]   := r[0x20]
        op::log(0x20, 0x21, 0x00, 0x00),
        op::ret(RegId::ONE),
    ];

    #[rustfmt::skip]
    let routine_unpack_and_xor_limbs_into_state = vec![
        op::jnei(0x10, 0x31, 45),                               // (1, b) Unpack arg into 4x16 and xor into state
        op::movi(0x20, 32),                                     // r[0x20]      := 32
        op::aloc(0x20),                                         // aloc            0x20
        op::move_(0x20, RegId::HP),                             // r[0x20]      := $hp
        op::srwq(0x20, SET_STATUS_REG, 0x11, RegId::ONE),   // m[0x20,32]       := s[m[b, 32], 32]
        op::lw(0x21, 0x11, 4),                                  // r[0x21]      := m[b+32, 8]
        op::log(0x21, 0x00, 0x00, 0x00),
        op::srli(0x22, 0x21, 48),                               // r[0x22]      := r[0x21] >> 48
        op::srli(0x23, 0x21, 32),                               // r[0x23]      := r[0x21] >> 32
        op::andi(0x23, 0x23, 0xff),                             // r[0x23]      &= 0xffff
        op::srli(0x24, 0x21, 16),                               // r[0x24]      := r[0x21] >> 16
        op::andi(0x24, 0x24, 0xff),                             // r[0x24]      &= 0xffff
        op::andi(0x25, 0x21, 0xff),                             // r[0x25]      := r[0x21] & 0xffff
        op::log(0x22, 0x23, 0x24, 0x25),
        op::lw(0x26, 0x20, 0),                                  // r[0x26]      := m[$fp, 8]
        op::xor(0x26, 0x26, 0x22),                              // r[0x26]      ^= r[0x22]
        op::log(0x26, 0x00, 0x00, 0x00),
        op::sw(0x20, 0x26, 0),                                  // m[0x20,8]    := r[0x26]
        op::lw(0x26, 0x20, 1),                                  // r[0x26]      := m[$fp+8, 8]
        op::xor(0x26, 0x26, 0x22),                              // r[0x26]      ^= r[0x22]
        op::log(0x26, 0x00, 0x00, 0x00),
        op::sw(0x20, 0x26, 1),                                  // m[0x20+8,8]  := r[0x26]
        op::lw(0x26, 0x20, 2),                                  // r[0x26]      := m[$fp+16, 8]
        op::xor(0x26, 0x26, 0x22),                              // r[0x26]      ^= r[0x22]
        op::log(0x26, 0x00, 0x00, 0x00),
        op::sw(0x20, 0x26, 2),                                  // m[0x20+16,8] := r[0x26]
        op::lw(0x26, 0x20, 3),                                  // r[0x26]      := m[$fp+24, 8]
        op::xor(0x26, 0x26, 0x22),                              // r[0x26]      ^= r[0x22]
        op::log(0x26, 0x00, 0x00, 0x00),
        op::sw(0x20, 0x26, 3),                                  // m[0x20+24,8] := r[0x26]
        op::swwq(0x11, SET_STATUS_REG, 0x20, RegId::ONE),   // s[m[b,32],32]:= m[0x20, 32]
        op::ret(RegId::ONE),
    ];

    #[rustfmt::skip]
    let invalid_call = vec![
        op::ret(RegId::ZERO),
    ];

    let program = function_selector
        .into_iter()
        .chain(call_arguments_parser)
        .chain(routine_add_word_to_state)
        .chain(routine_unpack_and_xor_limbs_into_state)
        .chain(invalid_call)
        .collect_vec();

    let contract_id = test_context.setup_contract(program, None, None).contract_id;

    let (script, script_data_offset) = script_with_data_offset!(
        data_offset,
        vec![
            op::movi(0x10, data_offset as Immediate18),
            op::call(0x10, RegId::ZERO, RegId::ZERO, RegId::CGAS),
            op::ret(RegId::ONE),
        ],
        test_context.get_tx_params().tx_offset()
    );

    let mut script_data = vec![];

    // Routine to be called: Add word to state
    let routine: Word = 0;

    // Offset of the script data relative to the call data
    let call_data_offset = script_data_offset as usize + ContractId::LEN + 2 * WORD_SIZE;
    let call_data_offset = call_data_offset as Word;

    // Key and value to be added
    let key = Hasher::hash(b"some key");
    let val: Word = 150;

    // Script data containing the call arguments (contract, a, b) and (key, value)
    script_data.extend(contract_id.as_ref());
    script_data.extend(routine.to_be_bytes());
    script_data.extend(call_data_offset.to_be_bytes());
    script_data.extend(key.as_ref());
    script_data.extend(val.to_be_bytes());

    // Assert the initial state of `key` is empty
    let state = test_context
        .get_storage()
        .contract_state(&contract_id, &key);
    assert_eq!(ContractsStateData::default(), state.into_owned());

    let result = test_context
        .start_script(script.clone(), script_data)
        .script_gas_limit(gas_limit)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute();

    let receipts = result.receipts();
    let state = test_context
        .get_storage()
        .contract_state(&contract_id, &key);

    // Assert the state of `key` is mutated to `val`
    assert_eq!(
        &val.to_be_bytes()[..],
        &state.as_ref().as_ref()[..WORD_SIZE]
    );

    // Expect the correct receipt
    assert_eq!(receipts[1].ra().expect("Register value expected"), val);
    assert_eq!(receipts[1].rb().expect("Register value expected"), 0);

    let mut script_data = vec![];

    // Routine to be called: Unpack and XOR into state
    let routine: Word = 1;

    // Create limbs reference values
    let a = 0x25;
    let b = 0xc1;
    let c = 0xd3;
    let d = 0xaa;

    let val: Word = (a << 48) | (b << 32) | (c << 16) | d;

    // Script data containing the call arguments (contract, a, b) and (key, value)
    script_data.extend(contract_id.as_ref());
    script_data.extend(routine.to_be_bytes());
    script_data.extend(call_data_offset.to_be_bytes());
    script_data.extend(key.as_ref());
    script_data.extend(val.to_be_bytes());

    let result = test_context
        .start_script(script, script_data)
        .script_gas_limit(gas_limit)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute();

    let receipts = result.receipts();

    // Expect the arguments to be received correctly by the VM
    assert_eq!(receipts[1].ra().expect("Register value expected"), val);
    assert_eq!(receipts[2].ra().expect("Register value expected"), a);
    assert_eq!(receipts[2].rb().expect("Register value expected"), b);
    assert_eq!(receipts[2].rc().expect("Register value expected"), c);
    assert_eq!(receipts[2].rd().expect("Register value expected"), d);

    let m = a ^ 0x96;
    let n = a;
    let o = a;
    let p = a;

    // Expect the value to be unpacked correctly into 4x16 limbs + XOR state
    assert_eq!(receipts[3].ra().expect("Register value expected"), m);
    assert_eq!(receipts[4].ra().expect("Register value expected"), n);
    assert_eq!(receipts[5].ra().expect("Register value expected"), o);
    assert_eq!(receipts[6].ra().expect("Register value expected"), p);

    let mut bytes = [0u8; 32];

    // Reconstruct the final state out of the limbs
    bytes[..8].copy_from_slice(&m.to_be_bytes());
    bytes[8..16].copy_from_slice(&n.to_be_bytes());
    bytes[16..24].copy_from_slice(&o.to_be_bytes());
    bytes[24..].copy_from_slice(&p.to_be_bytes());

    // Assert the state is correct
    let data = ContractsStateData::from(bytes.as_ref());
    let state = test_context
        .get_storage()
        .contract_state(&contract_id, &key);
    assert_eq!(data, state.into_owned());
}

#[test]
fn ldc__load_external_contract_code() {
    let rng = &mut StdRng::seed_from_u64(2322u64);
    let salt: Salt = rng.gen();

    let mut client = MemoryClient::default();

    let target_contract = vec![
        op::log(RegId::ONE, RegId::ONE, RegId::ONE, RegId::ONE),
        op::ret(RegId::ONE),
        op::ret(RegId::ZERO), // Pad to make length uneven to test padding
    ];

    let bytes = target_contract.into_iter().collect::<Vec<u8>>();
    let len = bytes.len().try_into().unwrap();
    let offset = 0;
    let program: Witness = bytes.into();

    let receipts = ldc__load_len_of_target_contract(
        &mut client,
        rng,
        salt,
        offset,
        len,
        program.clone(),
        true,
    );

    if let Receipt::LogData { digest, .. } = receipts.first().expect("No receipt") {
        let mut code = program.into_inner();
        code.extend([0; 4]);
        assert_eq!(digest, &Hasher::hash(&code), "Loaded code digest incorrect");
    } else {
        panic!("Script did not return a value");
    }

    if let Receipt::Log { ra, .. } = receipts.get(1).expect("No receipt") {
        assert_eq!(*ra, 1, "Invalid log from loaded code");
    } else {
        panic!("Script did not return a value");
    }
}

#[test]
fn ldc__gas_cost_is_not_dependent_on_rC() {
    let rng = &mut StdRng::seed_from_u64(2322u64);
    let salt: Salt = rng.gen();

    let mut client = MemoryClient::default();

    let gas_costs = client.gas_costs();
    let ldc_cost = gas_costs.ldc();
    let ldc_dep_len = match ldc_cost {
        DependentCost::LightOperation { units_per_gas, .. } => units_per_gas,
        DependentCost::HeavyOperation { gas_per_unit, .. } => gas_per_unit,
    };
    let noop_cost = gas_costs.noop();

    let contract_size = 1000;
    let offset = 0;
    let starting_rC = 0;
    let starting_gas_amount =
        ldc__gas_cost_for_len(&mut client, rng, salt, contract_size, offset, starting_rC);

    for i in 1..10 {
        // Increase by ldc_dep_cost for each attempt
        let len_diff = (i * ldc_dep_len) as u16;
        let len = starting_rC + len_diff;
        // The gas should go up 0 per ldc_dep_cost and 1 per noop_cost every 4
        // bytes (noop is 4 bytes)
        let cost_of_noops = len_diff as u64 / 4 * noop_cost;
        let expected_gas_used = starting_gas_amount + cost_of_noops;

        let actual_gas_used =
            ldc__gas_cost_for_len(&mut client, rng, salt, contract_size, offset, len);
        assert_eq!(actual_gas_used, expected_gas_used);
    }
}

#[test]
fn state_write_charges_for_new_storage() {
    let mut test_context = TestBuilder::new(2322u64);

    let balance = 1000;
    let gas_limit = 1_000_000;

    let prelude = vec![
        op::movi(0x14, 10),                        // The count for swwq
        op::muli(0x15, 0x14, Bytes32::LEN as u16), // Slot space for swwq
        op::aloc(0x15),                            // Allocate slots
    ];

    for operation in [
        op::sww(RegId::HP, 0x12, RegId::ONE),
        op::swwq(RegId::HP, 0x12, RegId::ONE, 0x14),
    ] {
        let [new_asset, existing_asset] = [true, false].map(|create_new_asset| {
            let mut program = prelude.clone();

            // Write before so the asset exists before measuring
            if !create_new_asset {
                program.push(operation);
            }

            // The write we're measuring
            program.extend([
                op::log(RegId::GGAS, RegId::ZERO, RegId::ZERO, RegId::ZERO),
                operation,
                op::log(RegId::GGAS, RegId::ZERO, RegId::ZERO, RegId::ZERO),
                op::ret(RegId::ONE),
            ]);

            let contract_id =
                test_context.setup_contract(program, None, None).contract_id;

            let (script_call, _) = script_with_data_offset!(
                data_offset,
                vec![
                    op::movi(0x10, data_offset as Immediate18),
                    op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS),
                    op::ret(RegId::ONE),
                ],
                test_context.get_tx_params().tx_offset()
            );
            let script_call_data = Call::new(contract_id, 0, balance).to_bytes();

            let result = test_context
                .start_script(script_call.clone(), script_call_data)
                .script_gas_limit(gas_limit)
                .contract_input(contract_id)
                .fee_input()
                .contract_output(&contract_id)
                .execute();

            let mut gas_values = result.receipts().iter().filter_map(|v| match v {
                Receipt::Log { ra, .. } => Some(ra),
                _ => None,
            });

            let gas_before = gas_values.next().expect("Missing log receipt");
            let gas_after = gas_values.next().expect("Missing log receipt");
            gas_before - gas_after
        });

        assert!(new_asset > existing_asset);
    }
}

#[test]
fn ldc__offset_affects_read_code() {
    let rng = &mut StdRng::seed_from_u64(2322u64);
    let salt: Salt = rng.gen();

    let mut client = MemoryClient::default();

    let gas_costs = client.gas_costs();
    let noop_cost = gas_costs.noop();

    let number_of_opcodes = 25;
    let offset = 0;
    let len = 100;
    let starting_gas_amount =
        ldc__gas_cost_for_len(&mut client, rng, salt, number_of_opcodes, offset, len);

    for i in 1..10 {
        let offset = i * 4;
        let expected_gas_used = starting_gas_amount - (i * noop_cost);
        let actual_gas_used = ldc__gas_cost_for_len(
            &mut client,
            rng,
            salt,
            number_of_opcodes,
            offset as u16,
            len,
        );

        assert_eq!(expected_gas_used, actual_gas_used);
    }
}

#[test]
fn ldc__cost_is_proportional_to_total_contracts_size_not_rC() {
    let rng = &mut StdRng::seed_from_u64(2322u64);
    let salt: Salt = rng.gen();

    let mut client = MemoryClient::default();

    let gas_costs = client.gas_costs();
    let ldc_cost = gas_costs.ldc();
    let ldc_dep_len = match ldc_cost {
        DependentCost::LightOperation { units_per_gas, .. } => units_per_gas,
        DependentCost::HeavyOperation { gas_per_unit, .. } => gas_per_unit,
    };
    let contract_size = 0;
    let offset = 0;
    let len = 0;
    let starting_gas_amount =
        ldc__gas_cost_for_len(&mut client, rng, salt, contract_size, offset, len);

    let bytes_per_op = 4;

    for i in 1..10 {
        let number_of_opcodes = contract_size + (i * ldc_dep_len / bytes_per_op) as usize;

        let cost_of_ldc = i;

        // The gas should go up 1 per every ldc_dep_cost even when rC is 0
        let expected_gas_used = starting_gas_amount + cost_of_ldc;

        let actual_gas_used =
            ldc__gas_cost_for_len(&mut client, rng, salt, number_of_opcodes, offset, len);
        assert_eq!(actual_gas_used, expected_gas_used);
    }
}

fn ldc__gas_cost_for_len(
    client: &mut MemoryClient,
    rng: &mut StdRng,
    salt: Salt,
    // in number of opcodes
    number_of_opcodes: usize,
    offset: u16,
    len: u16,
) -> Word {
    let mut target_contract = vec![];
    for _ in 0..number_of_opcodes {
        target_contract.push(op::noop());
    }

    let bytes = target_contract.into_iter().collect::<Vec<u8>>();
    let contract_code: Witness = bytes.into();

    let receipts = ldc__load_len_of_target_contract(
        client,
        rng,
        salt,
        offset,
        len,
        contract_code,
        false,
    );

    let result = receipts.last().expect("No receipt");

    let actual_gas_used = match result {
        Receipt::ScriptResult { gas_used, .. } => gas_used,
        _ => panic!("Should be a `ScriptResult`"),
    };

    *actual_gas_used
}

fn ldc__load_len_of_target_contract<'a>(
    client: &'a mut MemoryClient,
    rng: &mut StdRng,
    salt: Salt,
    offset: u16,
    len: u16,
    target_contract_witness: Witness,
    include_log_d: bool,
) -> &'a [Receipt] {
    let gas_limit = 1_000_000;
    let maturity = Default::default();
    let height = Default::default();

    let contract = Contract::from(target_contract_witness.as_ref());
    let contract_root = contract.root();
    let state_root = Contract::default_state_root();
    let contract_id = contract.id(&salt, &contract_root, &state_root);

    let input0 = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), contract_id);
    let output0 = Output::contract_created(contract_id, state_root);
    let output1 = Output::contract(0, rng.gen(), rng.gen());

    let consensus_params = ConsensusParameters::standard();

    let tx_create_target =
        TransactionBuilder::create(target_contract_witness.clone(), salt, vec![])
            .maturity(maturity)
            .add_random_fee_input()
            .add_output(output0)
            .finalize()
            .into_checked(height, &consensus_params)
            .expect("failed to check tx");

    client.deploy(tx_create_target).unwrap();

    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;
    let reg_b = 0x21;
    let reg_c = 0x22;

    let count = ContractId::LEN as Immediate12;

    let mut load_contract = vec![
        op::ori(reg_a, reg_a, count), // r[a] := r[a] | ContractId::LEN
        op::aloc(reg_a),              // Reserve space for contract id in the heap
    ];

    // Generate code for pushing contract id to heap
    for (i, byte) in contract_id.as_ref().iter().enumerate() {
        let index = i as Immediate12;
        let value = *byte as Immediate12;
        load_contract.extend([
            op::movi(reg_a, value.into()),   // r[a] := r[a] | value
            op::sb(RegId::HP, reg_a, index), // m[$hp+index] := r[a] (=value)
        ]);
    }

    // when
    load_contract.extend([
        op::move_(reg_a, RegId::HP),   // r[a] := $hp
        op::ori(reg_b, reg_b, offset), // r[b] += offset
        op::ori(reg_c, reg_c, len),    // r[b] += len
        op::ldc(reg_a, reg_b, reg_c),  // Load first two words from the contract
    ]);

    if include_log_d {
        let padded_len = pad(len);
        load_contract.extend([
            op::subi(reg_a, RegId::SSP, padded_len), /* r[a] := $ssp - padded_len
                                                      * (start of
                                                      * the loaded
                                                      * code) */
            op::movi(reg_c, padded_len as u32), /* r[b] = padded_len (length of the
                                                 * loaded code) */
            op::logd(RegId::ZERO, RegId::ZERO, reg_a, reg_c), /* Log digest of the
                                                               * loaded code */
        ])
    }

    load_contract.push(op::noop()); // Patched to the jump later

    let tx_deploy_loader = TransactionBuilder::script(
        #[allow(clippy::iter_cloned_collect)]
        load_contract.iter().copied().collect(),
        vec![],
    )
    .script_gas_limit(gas_limit)
    .maturity(maturity)
    .add_input(input0.clone())
    .add_random_fee_input()
    .add_output(output1)
    .finalize()
    .into_checked(height, &consensus_params)
    .expect("failed to check tx");

    // Patch the code with correct jump address
    let transaction_end_addr =
        tx_deploy_loader.transaction().size() - Script::script_offset_static();
    *load_contract.last_mut().unwrap() =
        op::ji((transaction_end_addr / 4) as Immediate24);

    let tx_deploy_loader =
        TransactionBuilder::script(load_contract.into_iter().collect(), vec![])
            .script_gas_limit(gas_limit)
            .maturity(maturity)
            .add_input(input0)
            .add_random_fee_input()
            .add_output(output1)
            .finalize()
            .into_checked(height, &consensus_params)
            .expect("failed to check tx");

    client.transact(tx_deploy_loader)
}

fn pad(a: u16) -> u16 {
    const SIZE: u16 = 8;
    let rem = a % SIZE;
    if rem == 0 {
        a
    } else {
        a + (SIZE - rem)
    }
}

fn ldc_reason_helper(cmd: Vec<Instruction>, expected_reason: PanicReason) {
    let rng = &mut StdRng::seed_from_u64(2322u64);
    let salt: Salt = rng.gen();
    let gas_price = 0;

    // make gas costs free
    let gas_costs = GasCosts::default();

    let mut consensus_params = ConsensusParameters::default();
    consensus_params.set_gas_costs(gas_costs);

    let interpreter_params = InterpreterParams::new(gas_price, &consensus_params);

    let mut client = MemoryClient::<NotSupportedEcal>::new(
        MemoryStorage::default(),
        interpreter_params,
    );

    let gas_limit = 1_000_000;
    let maturity = Default::default();
    let height = Default::default();

    // Start by creating and deploying a new example contract
    let contract_code = vec![
        op::log(RegId::ONE, RegId::ONE, RegId::ONE, RegId::ONE),
        op::ret(RegId::ONE),
        op::ret(RegId::ZERO), // Pad to make length uneven to test padding
    ];

    let program: Witness = contract_code.into_iter().collect::<Vec<u8>>().into();

    let contract = Contract::from(program.as_ref());
    let contract_root = contract.root();
    let state_root = Contract::default_state_root();
    let contract_id = contract.id(&salt, &contract_root, &state_root);

    let output0 = Output::contract_created(contract_id, state_root);

    let tx_create_target = TransactionBuilder::create(program, salt, vec![])
        .maturity(maturity)
        .add_random_fee_input()
        .add_output(output0)
        .finalize()
        .into_checked(height, &consensus_params)
        .expect("failed to check tx");

    client.deploy(tx_create_target).unwrap();

    let load_contract = cmd;

    let tx_deploy_loader = TransactionBuilder::script(
        load_contract.into_iter().collect(),
        contract_id.to_vec(),
    )
    .script_gas_limit(gas_limit)
    .maturity(maturity)
    .add_random_fee_input()
    .finalize()
    .into_checked(height, &consensus_params)
    .expect("failed to check tx");

    let receipts = client.transact(tx_deploy_loader);
    if let Receipt::Panic {
        id: _,
        reason,
        contract_id: actual_contract_id,
        ..
    } = receipts.first().expect("No receipt")
    {
        assert_eq!(
            &expected_reason,
            reason.reason(),
            "Expected {}, found {}",
            expected_reason,
            reason.reason()
        );
        if expected_reason == PanicReason::ContractNotFound {
            assert!(actual_contract_id.is_some());
            assert_ne!(actual_contract_id, &Some(contract_id));
        };
    } else {
        panic!("Script should have panicked before logging");
    }
}

#[test]
fn ldc_ssp_not_sp() {
    let (load_contract, _) = script_with_data_offset!(
        data_offset,
        vec![
            op::movi(0x10, data_offset as Immediate18),
            op::cfei(0x1), // sp += 1
            op::ldc(0x10, RegId::ZERO, RegId::ONE),
        ],
        TxParameters::DEFAULT.tx_offset()
    );

    ldc_reason_helper(load_contract, ExpectedUnallocatedStack);
}

#[test]
fn ldc_mem_offset_above_reg_hp() {
    // Then deploy another contract that attempts to read the first one

    let (load_contract, _) = script_with_data_offset!(
        data_offset,
        vec![
            op::movi(0x10, data_offset as Immediate18),
            op::ldc(0x10, RegId::ZERO, RegId::HP),
        ],
        TxParameters::DEFAULT.tx_offset()
    );

    ldc_reason_helper(load_contract, ContractMaxSize);
}

#[test]
fn ldc_contract_size_overflow() {
    ldc_reason_helper(
        vec![
            op::not(0x20, RegId::ZERO),
            op::ldc(RegId::HP, RegId::ZERO, 0x20),
        ],
        MemoryOverflow,
    );
}

#[test]
fn ldc_contract_id_end_beyond_max_ram() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;
    let reg_b = 0x21;

    // cover contract_id_end beyond max ram
    let load_contract = vec![
        op::move_(reg_a, RegId::HP),        // r[a] := $hp
        op::xor(reg_b, reg_b, reg_b),       // r[b] := 0
        op::ori(reg_b, reg_b, 12),          // r[b] += 12 (will be padded to 16)
        op::ldc(reg_a, RegId::ZERO, reg_b), // Load first two words from the contract
    ];

    ldc_reason_helper(load_contract, MemoryOverflow);
}

#[test]
fn ldc_contract_not_in_inputs() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;
    let reg_b = 0x21;

    // contract not in inputs
    let load_contract = vec![
        op::ldc(reg_a, RegId::ZERO, reg_b), // Load first two words from the contract
    ];

    ldc_reason_helper(load_contract, ContractNotInInputs);
}

#[test]
fn load_contract_code_copies_expected_bytes() {
    let mut test_context = TestBuilder::new(2322u64);
    let gas_limit = 1_000_000;

    let program_ops = vec![
        op::movi(0x10, 0x11),
        op::movi(0x11, 0x2a),
        op::add(0x12, 0x10, 0x11),
        op::log(0x10, 0x11, 0x12, 0x00),
        op::ret(0x20),
    ];

    let program = program_ops.clone().into_iter().collect::<Vec<u8>>();
    let contract_size = program.len();
    let contract_id = test_context
        .setup_contract(program_ops, None, None)
        .contract_id;

    let (script, _) = script_with_data_offset!(
        data_offset,
        vec![
            op::movi(0x20, data_offset as Immediate18),
            op::add(0x11, RegId::ZERO, 0x20),
            op::movi(0x12, 0 as Immediate18),
            op::movi(0x13, contract_size as Immediate18),
            op::move_(0x22, RegId::SSP),
            op::ldc(0x11, 0x12, 0x13),
            op::addi(0x21, 0x20, ContractId::LEN as Immediate12),
            op::meq(0x30, 0x21, 0x22, 0x13),
            op::ret(0x30),
        ],
        TxParameters::DEFAULT.tx_offset()
    );

    let mut script_data = contract_id.to_vec();
    script_data.extend(program.as_slice());

    let result = test_context
        .start_script(script, script_data)
        .script_gas_limit(gas_limit)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute();

    let receipts = result.receipts();
    let ret = receipts
        .first()
        .expect("A `RET` opcode was part of the program.");

    assert_eq!(1, ret.val().expect("Return value"));
}

#[test]
fn load_contract_code_out_of_contract_offset_over_length() {
    // This test like a `load_contract_code_copies_expected_bytes`, but the offset
    // is set to be beyond the length of the contract code. The `meq` should fail.
    let mut test_context = TestBuilder::new(2322u64);
    let gas_limit = 1_000_000;

    let program_ops = vec![
        op::movi(0x10, 0x11),
        op::movi(0x11, 0x2a),
        op::add(0x12, 0x10, 0x11),
        op::log(0x10, 0x11, 0x12, 0x00),
        op::ret(0x20),
    ];

    let program = program_ops.clone().into_iter().collect::<Vec<u8>>();
    let contract_size = program.len();
    let contract_id = test_context
        .setup_contract(program_ops, None, None)
        .contract_id;

    let (script, _) = script_with_data_offset!(
        data_offset,
        vec![
            op::movi(0x20, data_offset as Immediate18),
            op::add(0x11, RegId::ZERO, 0x20),
            op::movi(0x12, (contract_size + 1) as Immediate18),
            op::movi(0x13, contract_size as Immediate18),
            op::move_(0x22, RegId::SSP),
            op::ldc(0x11, 0x12, 0x13),
            op::addi(0x21, 0x20, ContractId::LEN as Immediate12),
            op::meq(0x30, 0x21, 0x22, 0x13),
            op::ret(0x30),
        ],
        TxParameters::DEFAULT.tx_offset()
    );

    let mut script_data = contract_id.to_vec();
    script_data.extend(program.as_slice());

    let result = test_context
        .start_script(script, script_data)
        .script_gas_limit(gas_limit)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute();

    let receipts = result.receipts();
    let ret = receipts
        .first()
        .expect("A `RET` opcode was part of the program.");

    assert_eq!(0, ret.val().expect("Return value"));
}

#[test]
fn code_copy_shorter_zero_padding() {
    let mut test_context = TestBuilder::new(2322u64);
    let gas_limit = 1_000_000;

    let program_ops = vec![
        op::movi(0x10, 0x11),
        op::movi(0x11, 0x2a),
        op::add(0x12, 0x10, 0x11),
        op::log(0x10, 0x11, 0x12, 0x00),
        op::ret(0x20),
    ];

    let program = program_ops.clone().into_iter().collect::<Vec<u8>>();
    let contract_size = program.len();
    let contract_id = test_context
        .setup_contract(program_ops, None, None)
        .contract_id;

    let (script, _) = script_with_data_offset!(
        data_offset,
        vec![
            op::movi(0x10, 2048),
            op::aloc(0x10),
            op::addi(0x10, RegId::HP, contract_size as Immediate12),
            op::movi(0x10, 1234),
            op::addi(0x10, RegId::HP, (contract_size + 1) as Immediate12),
            op::movi(0x10, 1234),
            op::movi(0x20, data_offset as Immediate18),
            op::add(0x11, RegId::ZERO, 0x20),
            op::movi(0x13, (contract_size + 2) as Immediate18),
            op::ccp(RegId::HP, 0x11, RegId::ZERO, 0x13),
            op::addi(0x21, 0x20, ContractId::LEN as Immediate12),
            op::meq(0x30, 0x21, RegId::HP, 0x13),
            op::ret(0x30),
        ],
        TxParameters::DEFAULT.tx_offset()
    );

    let mut script_data = contract_id.to_vec();
    script_data.extend(program.as_slice());
    script_data.extend(vec![0; 2]);

    let result = test_context
        .start_script(script, script_data)
        .script_gas_limit(gas_limit)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute();

    let receipts = result.receipts();
    let ret = receipts
        .first()
        .expect("A `RET` opcode was part of the program.");

    assert_eq!(1, ret.val().expect("A constant `1` was returned."));
}

#[test]
fn code_copy_out_of_contract_offset_over_length() {
    let mut test_context = TestBuilder::new(2322u64);
    let gas_limit = 1_000_000;

    let program_ops = vec![
        op::movi(0x10, 0x11),
        op::movi(0x11, 0x2a),
        op::add(0x12, 0x10, 0x11),
        op::log(0x10, 0x11, 0x12, 0x00),
        op::ret(0x20),
    ];

    let program = program_ops.clone().into_iter().collect::<Vec<u8>>();
    let contract_size = program.len();
    let contract_id = test_context
        .setup_contract(program_ops, None, None)
        .contract_id;

    let (script, _) = script_with_data_offset!(
        data_offset,
        vec![
            op::movi(0x10, 2048),
            op::aloc(0x10),
            op::movi(0x20, data_offset as Immediate18),
            op::add(0x11, RegId::ZERO, 0x20),
            op::movi(0x12, (contract_size + 1) as Immediate18),
            op::movi(0x13, contract_size as Immediate18),
            op::ccp(RegId::HP, 0x11, 0x12, 0x13),
            op::addi(0x21, 0x20, ContractId::LEN as Immediate12),
            op::meq(0x30, 0x21, RegId::HP, 0x13),
            op::ret(0x30),
        ],
        TxParameters::DEFAULT.tx_offset()
    );

    let mut script_data = contract_id.to_vec();
    script_data.extend(vec![0; contract_size].as_slice());

    let result = test_context
        .start_script(script, script_data)
        .script_gas_limit(gas_limit)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute();

    let receipts = result.receipts();
    let ret = receipts
        .first()
        .expect("A `RET` opcode was part of the program.");

    assert_eq!(1, ret.val().expect("Return value"));
}

#[test]
fn code_copy_a_gt_vmmax_sub_d() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // test memory offset above reg_hp value
    let code_copy = vec![
        op::slli(reg_a, RegId::ONE, MAX_MEM_SHL),
        op::addi(reg_a, reg_a, 1),
        op::ccp(reg_a, RegId::ZERO, RegId::ZERO, RegId::ZERO),
    ];

    check_expected_reason_for_instructions(code_copy, MemoryOverflow);
}

#[test]
fn code_copy_b_plus_32_overflow() {
    let reg_a = 0x20;
    // test overflow add
    let code_copy = vec![
        op::not(reg_a, RegId::ZERO),
        op::ccp(RegId::ZERO, reg_a, RegId::ZERO, RegId::ZERO),
    ];

    check_expected_reason_for_instructions(code_copy, MemoryOverflow);
}

#[test]
fn code_copy_b_gt_vm_max_ram() {
    let reg_a = 0x20;
    // test overflow add
    let code_copy = vec![
        op::slli(reg_a, RegId::ONE, MAX_MEM_SHL),
        op::subi(reg_a, reg_a, 31),
        op::ccp(RegId::ZERO, reg_a, RegId::ZERO, RegId::ZERO),
    ];

    check_expected_reason_for_instructions(code_copy, MemoryOverflow);
}

#[test]
fn code_copy_c_gt_vm_max_ram() {
    let reg_a = 0x20;
    // test overflow add
    let code_copy = vec![
        op::slli(reg_a, RegId::ONE, MAX_MEM_SHL),
        op::addi(reg_a, reg_a, 1),
        op::ccp(RegId::ZERO, RegId::ZERO, reg_a, RegId::ZERO),
    ];

    check_expected_reason_for_instructions(code_copy, MemoryOverflow);
}

#[test]
fn code_root_a_plus_32_overflow() {
    // Given
    let mut client = MemoryClient::default();
    let instructions = vec![op::noop(), op::noop(), op::noop()];
    let contract: Witness = instructions.into_iter().collect::<Vec<u8>>().into();

    let salt = Default::default();
    let code_root = Contract::root_from_code(contract.as_ref());
    let storage_slots = vec![];

    let state_root = Contract::initial_state_root(storage_slots.iter());
    let contract_id =
        Contract::from(contract.as_ref()).id(&salt, &code_root, &state_root);

    deploy_contract(&mut client, contract, salt, storage_slots);

    let reg_a = 0x20;
    let reg_contract = 0x21;

    let mut code_root_script = vec![];
    write_contract_id(&mut code_root_script, reg_contract, contract_id);

    // When
    // cover contract_id_end beyond max ram
    code_root_script.extend([op::not(reg_a, RegId::ZERO), op::croo(reg_a, reg_contract)]);

    // Then
    check_expected_reason_for_instructions_with_client(
        client,
        code_root_script,
        MemoryOverflow,
    );
}

#[test]
fn code_root_b_plus_32_overflow() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let code_root = vec![op::not(reg_a, RegId::ZERO), op::croo(RegId::ZERO, reg_a)];

    check_expected_reason_for_instructions(code_root, MemoryOverflow);
}

#[test]
fn code_root_a_over_max_ram() {
    // Given
    let mut client = MemoryClient::default();
    let instructions = vec![op::noop(), op::noop(), op::noop()];
    let contract: Witness = instructions.into_iter().collect::<Vec<u8>>().into();

    let salt = Default::default();
    let code_root = Contract::root_from_code(contract.as_ref());
    let storage_slots = vec![];

    let state_root = Contract::initial_state_root(storage_slots.iter());
    let contract_id =
        Contract::from(contract.as_ref()).id(&salt, &code_root, &state_root);

    deploy_contract(&mut client, contract, salt, storage_slots);

    let reg_a = 0x20;
    let reg_contract = 0x21;

    let mut code_root_script = vec![];
    write_contract_id(&mut code_root_script, reg_contract, contract_id);

    // When
    // cover contract_id_end beyond max ram
    code_root_script.extend([
        op::slli(reg_a, RegId::ONE, MAX_MEM_SHL),
        op::subi(reg_a, reg_a, 31 as Immediate12),
        op::croo(reg_a, reg_contract),
    ]);

    // Then
    check_expected_reason_for_instructions_with_client(
        client,
        code_root_script,
        MemoryOverflow,
    );
}

#[test]
fn code_root_b_over_max_ram() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let code_root = vec![
        op::slli(reg_a, RegId::ONE, MAX_MEM_SHL),
        op::subi(reg_a, reg_a, 31 as Immediate12),
        op::croo(RegId::ZERO, reg_a),
    ];

    check_expected_reason_for_instructions(code_root, MemoryOverflow);
}

#[test]
fn code_size_b_plus_32_overflow() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let code_root = vec![op::not(reg_a, RegId::ZERO), op::csiz(reg_a, reg_a)];

    check_expected_reason_for_instructions(code_root, MemoryOverflow);
}

#[test]
fn code_size_b_over_max_ram() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let code_root = vec![
        op::slli(reg_a, RegId::ONE, MAX_MEM_SHL),
        op::subi(reg_a, reg_a, 31 as Immediate12),
        op::csiz(reg_a, reg_a),
    ];

    check_expected_reason_for_instructions(code_root, MemoryOverflow);
}

#[test]
fn sww_sets_status() {
    #[rustfmt::skip]
    let program = vec![
        op::sww(0x30, SET_STATUS_REG, RegId::ZERO),
        op::srw(0x31, SET_STATUS_REG + 1, RegId::ZERO),
        op::log(SET_STATUS_REG, SET_STATUS_REG + 1, 0x00, 0x00),
        op::ret(RegId::ONE),
    ];

    check_receipts_for_program_call(program, vec![1, 1, 0, 0]);
}

#[test]
fn scwq_clears_status() {
    #[rustfmt::skip]
    let program = vec![
        op::sww(0x30, SET_STATUS_REG, RegId::ZERO),
        op::scwq(0x30, SET_STATUS_REG + 1, RegId::ONE),
        op::srw(0x30, SET_STATUS_REG + 2, RegId::ZERO),
        op::log(SET_STATUS_REG, SET_STATUS_REG + 1, SET_STATUS_REG + 2, 0x00),
        op::ret(RegId::ONE),
    ];

    check_receipts_for_program_call(program, vec![1, 1, 0, 0]);
}

#[test]
fn scwq_clears_status_for_range() {
    #[rustfmt::skip]
    let program = vec![
        op::movi(0x11, 100),
        op::aloc(0x11),
        op::addi(0x31, RegId::HP, 0x4),
        op::movi(0x32, 3),
        op::scwq(0x31, SET_STATUS_REG, 0x32),
        op::addi(0x31, RegId::HP, 0x4),
        op::swwq(0x31, SET_STATUS_REG + 1, 0x31, 0x32),
        op::addi(0x31, RegId::HP, 0x4),
        op::scwq(0x31, SET_STATUS_REG + 2, 0x32),
        op::log(SET_STATUS_REG, SET_STATUS_REG + 1, SET_STATUS_REG + 2, 0x00),
        op::ret(RegId::ONE),
    ];

    check_receipts_for_program_call(program, vec![0, 3, 1, 0]);
}

#[test]
fn srw_reads_status() {
    let program = vec![
        op::sww(0x30, SET_STATUS_REG, RegId::ZERO),
        op::srw(0x30, SET_STATUS_REG + 1, RegId::ZERO),
        op::srw(0x30, SET_STATUS_REG + 2, RegId::ZERO),
        op::srw(0x30, SET_STATUS_REG + 3, RegId::ONE),
        op::log(
            SET_STATUS_REG,
            SET_STATUS_REG + 1,
            SET_STATUS_REG + 2,
            SET_STATUS_REG + 3,
        ),
        op::ret(RegId::ONE),
    ];

    check_receipts_for_program_call(program, vec![1, 1, 1, 0]);
}

#[test]
fn srwq_reads_status() {
    #[rustfmt::skip]
    let program = vec![
        op::aloc(0x10),
        op::addi(0x31, RegId::HP, 0x5),
        op::sww(0x31, SET_STATUS_REG, RegId::ZERO),
        op::srwq(0x31, SET_STATUS_REG + 1, 0x31, RegId::ONE),
        op::srw(0x31, SET_STATUS_REG + 2, 0x31),
        op::log(SET_STATUS_REG, SET_STATUS_REG + 1, SET_STATUS_REG + 2, 0x00),
        op::ret(RegId::ONE),
    ];

    check_receipts_for_program_call(program, vec![1, 1, 1, 0]);
}

#[test]
fn srwq_reads_status_with_range() {
    #[rustfmt::skip]
    let program = vec![
        op::movi(0x11, 100),
        op::aloc(0x11),
        op::addi(0x31, RegId::HP, 0x5),
        op::movi(0x32, 0x2),
        op::srwq(0x31, SET_STATUS_REG, 0x31, 0x32),
        op::movi(0x32, 0x2),
        op::swwq(0x31, SET_STATUS_REG + 1, 0x31, 0x32),
        op::movi(0x32, 0x2),
        op::srwq(0x31, SET_STATUS_REG + 2, 0x31, 0x32),
        op::log(SET_STATUS_REG, SET_STATUS_REG + 1, SET_STATUS_REG + 2, 0x00),
        op::ret(RegId::ONE),
    ];

    check_receipts_for_program_call(program, vec![0, 2, 1, 0]);
}

#[test]
fn swwq_sets_status() {
    #[rustfmt::skip]
    let program = vec![
        op::aloc(0x10),
        op::addi(0x31, RegId::HP, 0x5),
        op::srw(0x31, SET_STATUS_REG, 0x31),
        op::swwq(0x31, SET_STATUS_REG + 1, 0x31, RegId::ONE),
        op::srw(0x31, SET_STATUS_REG + 2, 0x31),
        op::log(SET_STATUS_REG, SET_STATUS_REG + 1, SET_STATUS_REG + 2, 0x00),
        op::ret(RegId::ONE),
    ];

    check_receipts_for_program_call(program, vec![0, 1, 1, 0]);
}

#[test]
fn swwq_sets_status_with_range() {
    #[rustfmt::skip]
    let program = vec![
        op::movi(0x11, 100),
        op::aloc(0x11),
        op::movi(0x32, 0x2),
        op::swwq(RegId::HP, SET_STATUS_REG, 0x31, 0x32),
        op::swwq(RegId::HP, SET_STATUS_REG + 1, 0x31, 0x32),
        op::log(SET_STATUS_REG, SET_STATUS_REG + 1, 0x00, 0x00),
        op::ret(RegId::ONE),
    ];

    check_receipts_for_program_call(program, vec![2, 0, 0, 0]);
}

fn check_receipts_for_program_call(
    program: Vec<Instruction>,
    expected_values: Vec<Word>,
) -> bool {
    let mut test_context = TestBuilder::new(2322u64);
    let gas_limit = 1_000_000;

    let contract_id = test_context.setup_contract(program, None, None).contract_id;

    let (script, script_data_offset) = script_with_data_offset!(
        data_offset,
        vec![
            op::movi(0x10, data_offset as Immediate18),
            op::call(0x10, RegId::ZERO, RegId::ZERO, RegId::CGAS),
            op::ret(RegId::ONE),
        ],
        test_context.get_tx_params().tx_offset()
    );

    let mut script_data = vec![];

    // Routine to be called: Add word to state
    let routine: Word = 0;

    // Offset of the script data relative to the call data
    let call_data_offset = script_data_offset as usize + ContractId::LEN + 2 * WORD_SIZE;
    let call_data_offset = call_data_offset as Word;

    // Key and value to be added
    let key = Hasher::hash(b"some key");
    let val: Word = 150;

    // Script data containing the call arguments (contract_id, a, b) and (key, value)
    script_data.extend(contract_id.as_ref());
    script_data.extend(routine.to_be_bytes());
    script_data.extend(call_data_offset.to_be_bytes());
    script_data.extend(key.as_ref());
    script_data.extend(val.to_be_bytes());

    // Assert the initial state of `key` is empty
    let state = test_context
        .get_storage()
        .contract_state(&contract_id, &key);
    assert_eq!(ContractsStateData::default(), state.into_owned());

    let result = test_context
        .start_script(script, script_data)
        .script_gas_limit(gas_limit)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute();

    let receipts = result.receipts();

    // Expect the correct receipt
    assert_eq!(
        receipts[1].ra().expect("Register value expected"),
        expected_values[0],
        "$ra mismatch"
    );
    assert_eq!(
        receipts[1].rb().expect("Register value expected"),
        expected_values[1],
        "$rb mismatch"
    );
    assert_eq!(
        receipts[1].rc().expect("Register value expected"),
        expected_values[2],
        "$rc mismatch"
    );
    assert_eq!(
        receipts[1].rd().expect("Register value expected"),
        expected_values[3],
        "$rd mismatch"
    );

    true
}

#[test]
fn state_r_word_b_plus_32_over() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let state_read_word = vec![
        op::not(reg_a, RegId::ZERO),
        op::subi(reg_a, reg_a, 31 as Immediate12),
        op::srw(reg_a, SET_STATUS_REG, reg_a),
    ];

    check_expected_reason_for_instructions(state_read_word, MemoryOverflow);
}

#[test]
fn state_r_word_b_over_max_ram() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let state_read_word = vec![
        op::slli(reg_a, RegId::ONE, MAX_MEM_SHL),
        op::subi(reg_a, reg_a, 31 as Immediate12),
        op::srw(reg_a, SET_STATUS_REG, reg_a),
    ];

    check_expected_reason_for_instructions(state_read_word, MemoryOverflow);
}

#[test]
fn state_r_qword_a_plus_32_over() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let state_read_qword = vec![
        op::not(reg_a, RegId::ZERO),
        op::subi(reg_a, reg_a, 31 as Immediate12),
        op::srwq(reg_a, SET_STATUS_REG, RegId::ZERO, RegId::ONE),
    ];

    check_expected_reason_for_instructions(state_read_qword, MemoryOverflow);
}

#[test]
fn state_r_qword_c_plus_32_over() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let state_read_qword = vec![
        op::movi(0x11, 100),
        op::aloc(0x11),
        op::not(reg_a, RegId::ZERO),
        op::subi(reg_a, reg_a, 31 as Immediate12),
        op::srwq(RegId::HP, SET_STATUS_REG, reg_a, RegId::ONE),
    ];

    check_expected_reason_for_instructions(state_read_qword, MemoryOverflow);
}

#[test]
fn state_r_qword_a_over_max_ram() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let state_read_qword = vec![
        op::slli(reg_a, RegId::ONE, MAX_MEM_SHL),
        op::subi(reg_a, reg_a, 31 as Immediate12),
        op::srwq(reg_a, SET_STATUS_REG, RegId::ZERO, RegId::ONE),
    ];

    check_expected_reason_for_instructions(state_read_qword, MemoryOverflow);
}

#[test]
fn state_r_qword_c_over_max_ram() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let state_read_qword = vec![
        op::movi(0x11, 100),
        op::aloc(0x11),
        op::move_(0x31, RegId::HP),
        op::slli(reg_a, RegId::ONE, MAX_MEM_SHL),
        op::subi(reg_a, reg_a, 31 as Immediate12),
        op::srwq(0x31, SET_STATUS_REG, reg_a, RegId::ONE),
    ];

    check_expected_reason_for_instructions(state_read_qword, MemoryOverflow);
}

#[test]
fn state_w_word_a_plus_32_over() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let state_write_word = vec![
        op::not(reg_a, RegId::ZERO),
        op::subi(reg_a, reg_a, 31 as Immediate12),
        op::sww(reg_a, SET_STATUS_REG, RegId::ZERO),
    ];

    check_expected_reason_for_instructions(state_write_word, MemoryOverflow);
}

#[test]
fn state_w_word_a_over_max_ram() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let state_write_word = vec![
        op::slli(reg_a, RegId::ONE, MAX_MEM_SHL),
        op::subi(reg_a, reg_a, 31 as Immediate12),
        op::sww(reg_a, SET_STATUS_REG, RegId::ZERO),
    ];

    check_expected_reason_for_instructions(state_write_word, MemoryOverflow);
}

#[test]
fn state_w_qword_a_plus_32_over() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let state_write_qword = vec![
        op::not(reg_a, RegId::ZERO),
        op::subi(reg_a, reg_a, 31 as Immediate12),
        op::swwq(reg_a, SET_STATUS_REG, RegId::ZERO, RegId::ONE),
    ];

    check_expected_reason_for_instructions(state_write_qword, MemoryOverflow);
}

#[test]
fn state_w_qword_b_plus_32_over() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let state_write_qword = vec![
        op::not(reg_a, RegId::ZERO),
        op::subi(reg_a, reg_a, 31 as Immediate12),
        op::swwq(RegId::ZERO, SET_STATUS_REG, reg_a, RegId::ONE),
    ];

    check_expected_reason_for_instructions(state_write_qword, MemoryOverflow);
}

#[test]
fn state_w_qword_a_over_max_ram() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let state_write_qword = vec![
        op::slli(reg_a, RegId::ONE, MAX_MEM_SHL),
        op::subi(reg_a, reg_a, 31),
        op::swwq(reg_a, SET_STATUS_REG, RegId::ZERO, RegId::ONE),
    ];

    check_expected_reason_for_instructions(state_write_qword, MemoryOverflow);
}

#[test]
fn state_w_qword_b_over_max_ram() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let state_write_qword = vec![
        op::slli(reg_a, RegId::ONE, MAX_MEM_SHL),
        op::subi(reg_a, reg_a, 31),
        op::swwq(RegId::ZERO, SET_STATUS_REG, reg_a, RegId::ONE),
    ];

    check_expected_reason_for_instructions(state_write_qword, MemoryOverflow);
}

#[test]
fn message_output_b_gt_msg_len() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let message_output = vec![
        op::slli(reg_a, RegId::ONE, (VM_MAX_RAM.ilog2() + 2) as u16),
        op::smo(RegId::ZERO, reg_a, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ];

    check_expected_reason_for_instructions(message_output, MemoryOverflow);
}

#[test]
fn message_output_a_b_over() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let message_output = vec![
        op::not(reg_a, RegId::ZERO), // r[a] = MAX
        op::smo(reg_a, RegId::ONE, RegId::ZERO, RegId::ZERO),
    ];

    check_expected_reason_for_instructions(message_output, MemoryOverflow);
}

#[test]
fn message_output_a_b_gt_max_mem() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let message_output = vec![
        op::slli(reg_a, RegId::ONE, MAX_MEM_SHL),
        op::smo(reg_a, RegId::ONE, RegId::ZERO, RegId::ZERO),
    ];

    check_expected_reason_for_instructions(message_output, MemoryOverflow);
}

#[test]
fn smo_instruction_works() {
    fn execute_test<R>(
        rng: &mut R,
        inputs: Vec<(u64, Vec<u8>)>,
        message_output_amount: Word,
        gas_price: Word,
    ) -> bool
    where
        R: Rng + CryptoRng,
    {
        let mut client = MemoryClient::default();
        client.set_gas_price(gas_price);

        let gas_limit = 1_000_000;
        let max_fee = 100_000_000;
        let maturity = Default::default();
        let block_height = Default::default();

        let secret = SecretKey::random(rng);
        let sender = rng.gen();

        // Two bytes of random data to send as the message
        let msg_data = [rng.gen::<u8>(), rng.gen::<u8>()];

        #[rustfmt::skip]
        let script = vec![
            op::movi(0x10, 2),                          // data buffer allocation size
            op::aloc(0x10),                             // allocate
            op::movi(0x10, msg_data[0].into()),         // first message byte
            op::sb(RegId::HP, 0x10, 0),                 // store above to the message buffer
            op::movi(0x10, msg_data[1].into()),         // second message byte
            op::sb(RegId::HP, 0x10, 1),                 // store above to the message buffer
            op::movi(0x10, 0),                          // set the txid as recipient
            op::movi(0x12, 2),                          // one byte of data
            op::movi(0x13, message_output_amount as Immediate24),      // expected output amount
            op::smo(0x10, RegId::HP, 0x12, 0x13),
            op::ret(RegId::ONE),
        ];

        let script = script.into_iter().collect();
        let script_data = vec![];

        let mut tx = TransactionBuilder::script(script, script_data);
        tx.script_gas_limit(gas_limit)
            .maturity(maturity)
            .max_fee_limit(max_fee);
        // add inputs
        for (amount, data) in inputs {
            tx.add_unsigned_message_input(secret, sender, rng.gen(), amount, data);
        }
        tx.add_unsigned_coin_input(secret, rng.gen(), max_fee, AssetId::BASE, rng.gen());
        let tx = tx
            .add_output(Output::Change {
                to: Default::default(),
                amount: 0,
                asset_id: Default::default(),
            })
            .finalize_checked(block_height);

        let non_retryable_free_balance =
            tx.metadata().non_retryable_balances[&AssetId::BASE];
        let retryable_balance: u64 = tx.metadata().retryable_balance.into();

        let txid = tx.transaction().id(&ChainId::default());
        let receipts = client.transact(tx);

        let success = receipts.iter().any(|r| {
            matches!(
                r,
                Receipt::ScriptResult {
                    result: ScriptExecutionResult::Success,
                    ..
                }
            )
        });

        let state = client.state_transition().expect("tx was executed");
        let message_receipt = state.receipts().iter().find_map(|o| match o {
            Receipt::MessageOut {
                recipient,
                amount,
                data,
                ..
            } => Some((*recipient, *amount, data.clone())),
            _ => None,
        });
        assert_eq!(message_receipt.is_some(), success);
        if let Some((recipient, transferred, data)) = message_receipt {
            assert_eq!(txid.as_ref(), recipient.as_ref());
            assert_eq!(message_output_amount, transferred);
            assert_eq!(data.unwrap(), msg_data);
        }
        // get gas used from script result
        let gas_used = if let Receipt::ScriptResult { gas_used, .. } =
            state.receipts().last().unwrap()
        {
            gas_used
        } else {
            panic!("expected script result")
        };
        // get refunded fee amount
        let refund_amount = state
            .tx()
            .refund_fee(
                client.gas_costs(),
                client.fee_params(),
                *gas_used,
                gas_price,
            )
            .unwrap();

        // check that refundable balances aren't converted into change on failed txs
        if !success {
            assert!(
                matches!(state.tx().outputs()[0], Output::Change { amount, ..} if amount == non_retryable_free_balance + refund_amount)
            );
        } else {
            // check that unused retryable balance is returned as change
            assert!(
                matches!(state.tx().outputs()[0], Output::Change { amount, ..} if amount == non_retryable_free_balance + refund_amount + retryable_balance - message_output_amount)
            );
        }

        success
    }

    let rng = &mut StdRng::seed_from_u64(2322u64);

    // check arbitrary amount
    assert!(execute_test(rng, vec![(10, vec![0xfa; 15])], 10, 0));

    // check message with zero amount
    assert!(execute_test(rng, vec![(0, vec![0xfa; 15])], 0, 0));

    // Send more than we have
    assert!(!execute_test(rng, vec![(10, vec![0xfa; 15])], 11, 0));

    // check that retryable balance isn't refunded on revert
    assert!(!execute_test(
        rng,
        vec![(10, vec![0xfa; 15]), (1000, vec![])],
        1011,
        1,
    ));

    // check that retryable balance is refunded to user
    assert!(execute_test(
        rng,
        vec![(10, vec![0xfa; 15]), (1000, vec![])],
        50,
        1,
    ));
}

#[test]
fn timestamp_works() {
    let mut client = MemoryClient::default();

    let gas_limit = 1_000_000;
    let maturity = Default::default();
    let block_height = Default::default();

    // TODO consider using quickcheck after PR lands
    // https://github.com/FuelLabs/fuel-vm/pull/187
    let cases = vec![
        (0, 0),
        (0, 1),
        (5, 0),
        (5, 5),
        (5, 6),
        (10, 0),
        (10, 5),
        (10, 10),
        (10, 1_000),
    ];

    for (height, input) in cases {
        client.as_mut().set_block_height(height.into());

        let expected = client
            .as_ref()
            .timestamp(input.into())
            .expect("failed to calculate timestamp");

        #[rustfmt::skip]
        let script = vec![
            op::movi(0x11, input),              // set the argument
            op::time(0x10, 0x11),               // perform the instruction
            op::log(0x10, 0x00, 0x00, 0x00),    // log output
            op::ret(RegId::ONE),
        ];

        let script = script.into_iter().collect();
        let script_data = vec![];

        let tx = TransactionBuilder::script(script, script_data)
            .script_gas_limit(gas_limit)
            .maturity(maturity)
            .add_random_fee_input()
            .finalize_checked(block_height);

        let receipts = client.transact(tx);
        let result = receipts.iter().any(|r| {
            matches!(
                r,
                Receipt::ScriptResult {
                    result: ScriptExecutionResult::Success,
                    ..
                }
            )
        });

        assert_eq!(result, input <= height);

        if result {
            let ra = receipts
                .iter()
                .find_map(|r| match r {
                    Receipt::Log { ra, .. } => Some(*ra),
                    _ => None,
                })
                .expect("failed to fetch log");

            assert_eq!(ra, expected);
        }
    }
}

#[rstest::rstest]
fn block_height_works(#[values(0, 1, 2, 10, 100)] current_height: u32) {
    let current_height: BlockHeight = current_height.into();

    let mut client = MemoryClient::default();

    let gas_limit = 1_000_000;
    let maturity = Default::default();

    client.as_mut().set_block_height(current_height);

    #[rustfmt::skip]
    let script = vec![
        op::bhei(0x20),         // perform the instruction
        op::log(0x20, 0, 0, 0), // log output
        op::ret(RegId::ONE),
    ];

    let script = script.into_iter().collect();
    let script_data = vec![];

    let tx = TransactionBuilder::script(script, script_data)
        .script_gas_limit(gas_limit)
        .maturity(maturity)
        .add_random_fee_input()
        .finalize_checked(current_height);

    let receipts = client.transact(tx);
    let Some(Receipt::Log { ra, .. }) = receipts.first() else {
        panic!("expected log receipt");
    };

    let r: u32 = (*ra).try_into().unwrap();
    let result: BlockHeight = r.into();
    assert_eq!(result, current_height);
}

#[rstest::rstest]
fn block_hash_works(
    #[values(0, 1, 2, 10, 100)] current_height: u32,
    #[values(0, 1, 2, 10, 100)] test_height: u32,
) {
    let current_height: BlockHeight = current_height.into();
    let test_height: BlockHeight = test_height.into();

    let mut client = MemoryClient::default();

    let gas_limit = 1_000_000;
    let maturity = Default::default();

    client.as_mut().set_block_height(current_height);

    let expected = client
        .as_ref()
        .block_hash(test_height)
        .expect("failed to calculate block hash");

    #[rustfmt::skip]
    let script = vec![
        op::movi(0x10, 32),                 // allocation size
        op::aloc(0x10),                     // allocate memory
        op::movi(0x11, test_height.into()), // set the argument
        op::bhsh(RegId::HP, 0x11),          // perform the instruction
        op::logd(0, 0, RegId::HP, 0x10),    // log output
        op::ret(RegId::ONE),
    ];

    let script = script.into_iter().collect();
    let script_data = vec![];

    let tx = TransactionBuilder::script(script, script_data)
        .script_gas_limit(gas_limit)
        .maturity(maturity)
        .add_random_fee_input()
        .finalize_checked(current_height);

    let receipts = client.transact(tx);
    let Some(Receipt::LogData { data, .. }) = receipts.first() else {
        panic!("expected log receipt");
    };

    assert_eq!(data.as_ref().unwrap(), &*expected);
}

#[rstest::rstest]
fn coinbase_works() {
    let mut client = MemoryClient::default();

    let gas_limit = 1_000_000;
    let maturity = Default::default();

    let expected = client
        .as_ref()
        .coinbase()
        .expect("failed to calculate block hash");

    #[rustfmt::skip]
    let script = vec![
        op::movi(0x10, 32),                 // allocation size
        op::aloc(0x10),                     // allocate memory
        op::cb(RegId::HP),                  // perform the instruction
        op::logd(0, 0, RegId::HP, 0x10),    // log output
        op::ret(RegId::ONE),
    ];

    let script = script.into_iter().collect();
    let script_data = vec![];

    let tx = TransactionBuilder::script(script, script_data)
        .script_gas_limit(gas_limit)
        .maturity(maturity)
        .add_random_fee_input()
        .finalize_checked(10.into());

    let receipts = client.transact(tx);
    let Some(Receipt::LogData { data, .. }) = receipts.first() else {
        panic!("expected log receipt");
    };

    assert_eq!(data.as_ref().unwrap(), &*expected);
}
