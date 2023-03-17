use fuel_asm::RegId;
use fuel_crypto::{Hasher, SecretKey};
use fuel_tx::{field::Outputs, Input, Output, Receipt, TransactionBuilder};
use fuel_types::{bytes, AssetId};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use fuel_vm::consts::*;
use fuel_vm::prelude::*;

use fuel_asm::{
    op, Instruction,
    PanicReason::{ArithmeticOverflow, ContractNotInInputs, ExpectedUnallocatedStack, MemoryOverflow, MemoryOwnership},
};
use fuel_tx::field::Script as ScriptField;
use fuel_vm::util::test_helpers::check_expected_reason_for_instructions;

const SET_STATUS_REG: u8 = 0x39;
// log2(VM_MAX_MEM) - used to set a pointer to the memory boundary via SHL: 1<<log2(VM_MAX_MEM)
const MAX_MEM_SHL: Immediate12 = 26 as Immediate12;

#[test]
fn state_read_write() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::default();

    let salt: Salt = rng.gen();

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

    let program: Witness = function_selector
        .into_iter()
        .chain(call_arguments_parser.into_iter())
        .chain(routine_add_word_to_state.into_iter())
        .chain(routine_unpack_and_xor_limbs_into_state.into_iter())
        .chain(invalid_call.into_iter())
        .collect::<Vec<u8>>()
        .into();

    let contract = Contract::from(program.as_ref());
    let contract_root = contract.root();
    let state_root = Contract::default_state_root();
    let contract = contract.id(&salt, &contract_root, &state_root);

    let output = Output::contract_created(contract, state_root);

    let bytecode_witness = 0;
    let tx_deploy = Transaction::create(
        gas_price,
        gas_limit,
        maturity,
        bytecode_witness,
        salt,
        vec![],
        vec![],
        vec![output],
        vec![program],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to check tx");

    let input = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), contract);
    let output = Output::contract(0, rng.gen(), rng.gen());

    // The script needs to locate the data offset at runtime. Hence, we need to know
    // upfront the serialized size of the script so we can set the registers
    // accordingly.
    //
    // This variable is created to assert we have correct script size in the
    // instructions.
    let script_len = 16;

    // Based on the defined script length, we set the appropriate data offset
    let script_data_offset = client.tx_offset() + Script::script_offset_static() + script_len;
    let script_data_offset = script_data_offset as Immediate18;

    let script = vec![
        op::movi(0x10, script_data_offset),
        op::call(0x10, RegId::ZERO, RegId::ZERO, RegId::CGAS),
        op::ret(RegId::ONE),
    ]
    .into_iter()
    .collect::<Vec<u8>>();

    // Assert the offsets are set correctly
    let offset = client.tx_offset() + Script::script_offset_static() + bytes::padded_len(script.as_slice());
    assert_eq!(script_data_offset, offset as Immediate18);

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
    script_data.extend(contract.as_ref());
    script_data.extend(routine.to_be_bytes());
    script_data.extend(call_data_offset.to_be_bytes());
    script_data.extend(key.as_ref());
    script_data.extend(val.to_be_bytes());

    let tx_add_word = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script.clone(),
        script_data,
        vec![input.clone()],
        vec![output],
        vec![],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to check tx");

    // Assert the initial state of `key` is empty
    let state = client.as_ref().contract_state(&contract, &key);
    assert_eq!(Bytes32::default(), state.into_owned());

    client.deploy(tx_deploy);
    client.transact(tx_add_word);

    let receipts = client.receipts().expect("The transaction was executed");
    let state = client.as_ref().contract_state(&contract, &key);

    // Assert the state of `key` is mutated to `val`
    assert_eq!(&val.to_be_bytes()[..], &state.as_ref()[..WORD_SIZE]);

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
    script_data.extend(contract.as_ref());
    script_data.extend(routine.to_be_bytes());
    script_data.extend(call_data_offset.to_be_bytes());
    script_data.extend(key.as_ref());
    script_data.extend(val.to_be_bytes());

    let tx_unpack_xor = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script,
        script_data,
        vec![input],
        vec![output],
        vec![],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to check tx");

    // Mutate the state
    client.transact(tx_unpack_xor);

    let receipts = client.receipts().expect("Expected receipts");

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
    let bytes = Bytes32::from(bytes);
    let state = client.as_ref().contract_state(&contract, &key);
    assert_eq!(bytes, state.into_owned());
}

#[test]
fn load_external_contract_code() {
    let rng = &mut StdRng::seed_from_u64(2322u64);
    let salt: Salt = rng.gen();

    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::default();

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

    let input0 = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), contract_id);
    let output0 = Output::contract_created(contract_id, state_root);
    let output1 = Output::contract(0, rng.gen(), rng.gen());

    let tx_create_target = Transaction::create(
        gas_price,
        gas_limit,
        maturity,
        0,
        salt,
        vec![],
        vec![],
        vec![output0],
        vec![program.clone()],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to check tx");

    client.deploy(tx_create_target);

    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;
    let reg_b = 0x21;

    let count = ContractId::LEN as Immediate12;

    let mut load_contract = vec![
        op::xor(reg_a, reg_a, reg_a), // r[a] := 0
        op::ori(reg_a, reg_a, count), // r[a] := r[a] | ContractId::LEN
        op::aloc(reg_a),              // Reserve space for contract id in the heap
    ];

    // Generate code for pushing contract id to heap
    for (i, byte) in contract_id.as_ref().iter().enumerate() {
        let index = i as Immediate12;
        let value = *byte as Immediate12;
        load_contract.extend([
            op::xor(reg_a, reg_a, reg_a),    // r[a] := 0
            op::ori(reg_a, reg_a, value),    // r[a] := r[a] | value
            op::sb(RegId::HP, reg_a, index), // m[$hp+index] := r[a] (=value)
        ]);
    }

    load_contract.extend([
        op::move_(reg_a, RegId::HP),                      // r[a] := $hp
        op::xor(reg_b, reg_b, reg_b),                     // r[b] := 0
        op::ori(reg_b, reg_b, 12),                        // r[b] += 12 (will be padded to 16)
        op::ldc(reg_a, RegId::ZERO, reg_b),               // Load first two words from the contract
        op::move_(reg_a, RegId::SSP),                     // r[b] := $ssp
        op::subi(reg_a, reg_a, 8 * 2),                    // r[a] -= 16 (start of the loaded code)
        op::xor(reg_b, reg_b, reg_b),                     // r[b] := 0
        op::addi(reg_b, reg_b, 16),                       // r[b] += 16 (length of the loaded code)
        op::logd(RegId::ZERO, RegId::ZERO, reg_a, reg_b), // Log digest of the loaded code
        op::noop(),                                       // Patched to the jump later
    ]);

    let tx_deploy_loader = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        #[allow(clippy::iter_cloned_collect)]
        load_contract.iter().copied().collect(),
        vec![],
        vec![input0.clone()],
        vec![output1],
        vec![],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to check tx");

    // Patch the code with correct jump address
    let transaction_end_addr = tx_deploy_loader.transaction().serialized_size() - Script::script_offset_static();
    *load_contract.last_mut().unwrap() = op::ji((transaction_end_addr / 4) as Immediate24);

    let tx_deploy_loader = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        load_contract.into_iter().collect(),
        vec![],
        vec![input0],
        vec![output1],
        vec![],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to check tx");

    let receipts = client.transact(tx_deploy_loader);

    if let Receipt::LogData { digest, .. } = receipts.get(0).expect("No receipt") {
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

fn ldc_reason_helper(cmd: Vec<Instruction>, expected_reason: PanicReason, should_patch_jump: bool) {
    let rng = &mut StdRng::seed_from_u64(2322u64);
    let salt: Salt = rng.gen();

    let mut client = MemoryClient::new(MemoryStorage::default(), Default::default(), GasCosts::free());

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::default();

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

    let input0 = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), contract_id);
    let output0 = Output::contract_created(contract_id, state_root);
    let output1 = Output::contract(0, rng.gen(), rng.gen());

    let tx_create_target = Transaction::create(
        gas_price,
        gas_limit,
        maturity,
        0,
        salt,
        vec![],
        vec![],
        vec![output0],
        vec![program],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to check tx");

    client.deploy(tx_create_target);

    //test ssp != sp for LDC
    let mut load_contract: Vec<Instruction>;

    let mut tx_deploy_loader;

    if !should_patch_jump {
        load_contract = cmd;

        tx_deploy_loader = Transaction::script(
            gas_price,
            gas_limit,
            maturity,
            load_contract.into_iter().collect(),
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .into_checked(height, &params, client.gas_costs())
        .expect("failed to check tx");
    } else {
        let reg_a = 0x20;
        let count = ContractId::LEN as Immediate12;

        load_contract = vec![
            op::xor(reg_a, reg_a, reg_a), // r[a] := 0
            op::ori(reg_a, reg_a, count), // r[a] := r[a] | ContractId::LEN
            op::aloc(reg_a),              // Reserve space for contract id in the heap
        ];

        // Generate code for pushing contract id to heap
        for (i, byte) in contract_id.as_ref().iter().enumerate() {
            let index = i as Immediate12;
            let value = *byte as Immediate12;
            load_contract.extend([
                op::xor(reg_a, reg_a, reg_a),    // r[a] := 0
                op::ori(reg_a, reg_a, value),    // r[a] := r[a] | value
                op::sb(RegId::HP, reg_a, index), // m[$hp+index] := r[a] (=value)
            ]);
        }

        load_contract.extend(cmd);

        tx_deploy_loader = Transaction::script(
            gas_price,
            gas_limit,
            maturity,
            load_contract.clone().into_iter().collect(),
            vec![],
            vec![input0.clone()],
            vec![output1],
            vec![],
        )
        .into_checked(height, &params, client.gas_costs())
        .expect("failed to check tx");

        // Patch the code with correct jump address
        let transaction_end_addr = tx_deploy_loader.transaction().serialized_size() - Script::script_offset_static();
        *load_contract.last_mut().unwrap() = op::ji((transaction_end_addr / 4) as Immediate24);

        tx_deploy_loader = Transaction::script(
            gas_price,
            gas_limit,
            maturity,
            load_contract.into_iter().collect(),
            vec![],
            vec![input0],
            vec![output1],
            vec![],
        )
        .into_checked(height, &params, client.gas_costs())
        .expect("failed to check tx");
    }

    let receipts = client.transact(tx_deploy_loader);
    if let Receipt::Panic {
        id: _,
        reason,
        contract_id: actual_contract_id,
        ..
    } = receipts.get(0).expect("No receipt")
    {
        assert_eq!(
            &expected_reason,
            reason.reason(),
            "Expected {}, found {}",
            expected_reason,
            reason.reason()
        );
        if expected_reason == PanicReason::ContractNotInInputs {
            assert!(actual_contract_id.is_some());
            assert_ne!(actual_contract_id, &Some(contract_id));
        };
    } else {
        panic!("Script should have panicked");
    }
}

#[test]
fn ldc_ssp_not_sp() {
    //test ssp != sp for LDC
    let load_contract = vec![
        op::cfei(0x1),                                  // sp += 1
        op::ldc(RegId::ZERO, RegId::ZERO, RegId::ZERO), // Load first two words from the contract
    ];

    ldc_reason_helper(load_contract, ExpectedUnallocatedStack, false);
}

#[test]
fn ldc_mem_offset_above_reg_hp() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    //test memory offset above reg_hp value
    let load_contract = vec![
        op::move_(reg_a, RegId::HP),              // r[a] := $hp
        op::ldc(RegId::ZERO, RegId::ZERO, reg_a), // Load first two words from the contract
    ];

    ldc_reason_helper(load_contract, MemoryOverflow, false);
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

    ldc_reason_helper(load_contract, MemoryOverflow, false);
}

#[test]
fn ldc_contract_not_in_inputs() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;
    let reg_b = 0x21;

    //contract not in inputs
    let load_contract = vec![
        op::xor(reg_a, reg_a, reg_a),       // r[b] := 0
        op::addi(reg_a, reg_a, 1),          // r[a] += 1
        op::xor(reg_b, reg_b, reg_b),       // r[b] := 0
        op::ori(reg_b, reg_b, 12),          // r[b] += 12 (will be padded to 16)
        op::ldc(reg_a, RegId::ZERO, reg_b), // Load first two words from the contract
    ];

    ldc_reason_helper(load_contract, ContractNotInInputs, false);
}

#[test]
fn ldc_contract_offset_over_length() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;
    let reg_b = 0x21;

    let load_contract = vec![
        op::move_(reg_a, RegId::HP),                      // r[a] := $hp
        op::xor(reg_b, reg_b, reg_b),                     // r[b] := 0
        op::ori(reg_b, reg_b, 12),                        // r[b] += 12 (will be padded to 16)
        op::ldc(reg_a, reg_a, reg_b),                     // Load first two words from the contract
        op::move_(reg_a, RegId::SSP),                     // r[b] := $ssp
        op::subi(reg_a, reg_a, 8 * 2),                    // r[a] -= 16 (start of the loaded code)
        op::xor(reg_b, reg_b, reg_b),                     // r[b] := 0
        op::ori(reg_b, reg_b, 16),                        // r[b] += 16 (length of the loaded code)
        op::logd(RegId::ZERO, RegId::ZERO, reg_a, reg_b), // Log digest of the loaded code
        op::noop(),                                       // Patched to the jump later
    ];

    ldc_reason_helper(load_contract, MemoryOverflow, true);
}

#[test]
fn code_copy_a_gt_vmmax_sub_d() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    //test memory offset above reg_hp value
    let code_copy = vec![
        op::xor(reg_a, reg_a, reg_a),
        op::ori(reg_a, reg_a, 1),
        op::slli(reg_a, reg_a, MAX_MEM_SHL),
        op::addi(reg_a, reg_a, 1),
        op::ccp(reg_a, RegId::ZERO, RegId::ZERO, RegId::ZERO),
    ];

    check_expected_reason_for_instructions(code_copy, MemoryOverflow);
}

#[test]
fn code_copy_b_plus_32_overflow() {
    let reg_a = 0x20;
    //test overflow add
    let code_copy = vec![
        op::xor(reg_a, reg_a, reg_a),
        op::not(reg_a, reg_a),
        op::ccp(RegId::ZERO, reg_a, RegId::ZERO, RegId::ZERO),
    ];

    check_expected_reason_for_instructions(code_copy, MemoryOverflow);
}

#[test]
fn code_copy_b_gt_vm_max_ram() {
    let reg_a = 0x20;
    //test overflow add
    let code_copy = vec![
        op::xor(reg_a, reg_a, reg_a),
        op::ori(reg_a, reg_a, 1),
        op::slli(reg_a, reg_a, MAX_MEM_SHL),
        op::subi(reg_a, reg_a, 31),
        op::ccp(RegId::ZERO, reg_a, RegId::ZERO, RegId::ZERO),
    ];

    check_expected_reason_for_instructions(code_copy, MemoryOverflow);
}

#[test]
fn code_copy_c_gt_vm_max_ram() {
    let reg_a = 0x20;
    //test overflow add
    let code_copy = vec![
        op::xor(reg_a, reg_a, reg_a),
        op::ori(reg_a, reg_a, 1),
        op::slli(reg_a, reg_a, MAX_MEM_SHL),
        op::addi(reg_a, reg_a, 1),
        op::ccp(RegId::ZERO, RegId::ZERO, reg_a, RegId::ZERO),
    ];

    check_expected_reason_for_instructions(code_copy, MemoryOverflow);
}

#[test]
fn code_root_a_plus_32_overflow() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let code_root = vec![
        op::xor(reg_a, reg_a, reg_a),
        op::not(reg_a, reg_a),
        op::croo(reg_a, RegId::ZERO),
    ];

    check_expected_reason_for_instructions(code_root, ArithmeticOverflow);
}

#[test]
fn code_root_b_plus_32_overflow() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let code_root = vec![
        op::xor(reg_a, reg_a, reg_a),
        op::not(reg_a, reg_a),
        op::croo(RegId::ZERO, reg_a),
    ];

    check_expected_reason_for_instructions(code_root, MemoryOverflow);
}

#[test]
fn code_root_a_over_max_ram() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let code_root = vec![
        op::xor(reg_a, reg_a, reg_a),
        op::ori(reg_a, reg_a, 1),
        op::slli(reg_a, reg_a, MAX_MEM_SHL),
        op::subi(reg_a, reg_a, 31 as Immediate12),
        op::croo(reg_a, RegId::ZERO),
    ];

    check_expected_reason_for_instructions(code_root, MemoryOverflow);
}

#[test]
fn code_root_b_over_max_ram() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let code_root = vec![
        op::xor(reg_a, reg_a, reg_a),
        op::ori(reg_a, reg_a, 1),
        op::slli(reg_a, reg_a, MAX_MEM_SHL),
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
    let code_root = vec![
        op::xor(reg_a, reg_a, reg_a),
        op::not(reg_a, reg_a),
        op::csiz(reg_a, reg_a),
    ];

    check_expected_reason_for_instructions(code_root, MemoryOverflow);
}

#[test]
fn code_size_b_over_max_ram() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let code_root = vec![
        op::xor(reg_a, reg_a, reg_a),
        op::ori(reg_a, reg_a, 1),
        op::slli(reg_a, reg_a, MAX_MEM_SHL),
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

    check_receipts_for_program_call(program, vec![0, 1, 0, 0]);
}

#[test]
fn scwq_clears_status() {
    #[rustfmt::skip]
    let program = vec![
        op::sww(0x30,  SET_STATUS_REG, RegId::ZERO),
        op::scwq(0x30, SET_STATUS_REG + 1, RegId::ONE),
        op::srw(0x30, SET_STATUS_REG + 2, RegId::ZERO),
        op::log(SET_STATUS_REG, SET_STATUS_REG + 1, SET_STATUS_REG + 2, 0x00),
        op::ret(RegId::ONE),
    ];

    check_receipts_for_program_call(program, vec![0, 1, 0, 0]);
}

#[test]
fn scwq_clears_status_for_range() {
    #[rustfmt::skip]
    let program = vec![
        op::movi(0x11, 100),
        op::aloc(0x11),
        op::addi(0x31, RegId::HP, 0x4),
        op::addi(0x32, RegId::ONE, 2),
        op::scwq(0x31, SET_STATUS_REG, 0x32),
        op::addi(0x31, RegId::HP, 0x4),
        op::swwq(0x31, SET_STATUS_REG + 1, 0x31, 0x32),
        op::addi(0x31, RegId::HP, 0x4),
        op::scwq(0x31, SET_STATUS_REG + 2, 0x32),
        op::log(SET_STATUS_REG, SET_STATUS_REG + 1, SET_STATUS_REG + 2, 0x00),
        op::ret(RegId::ONE),
    ];

    check_receipts_for_program_call(program, vec![0, 0, 1, 0]);
}

#[test]
fn srw_reads_status() {
    #[rustfmt::skip]
    let program = vec![
        op::sww(0x30,  SET_STATUS_REG, RegId::ZERO),
        op::srw(0x30, SET_STATUS_REG + 1, RegId::ZERO),
        op::srw(0x30, SET_STATUS_REG + 2, RegId::ZERO),
        op::srw(0x30, SET_STATUS_REG + 3, RegId::ONE),
        op::log(SET_STATUS_REG,
                    SET_STATUS_REG + 1,
                    SET_STATUS_REG + 2,
                    SET_STATUS_REG + 3),
        op::ret(RegId::ONE),
    ];

    check_receipts_for_program_call(program, vec![0, 1, 1, 0]);
}

#[test]
fn srwq_reads_status() {
    #[rustfmt::skip]
    let program = vec![
        op::aloc(0x10),
        op::addi(0x31, RegId::HP, 0x5),
        op::sww(0x31,  SET_STATUS_REG, RegId::ZERO),
        op::srwq(0x31, SET_STATUS_REG + 1, 0x31, RegId::ONE),
        op::srw(0x31, SET_STATUS_REG + 2, 0x31),
        op::log(SET_STATUS_REG, SET_STATUS_REG + 1, SET_STATUS_REG + 2, 0x00),
        op::ret(RegId::ONE),
    ];

    check_receipts_for_program_call(program, vec![0, 1, 1, 0]);
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

    check_receipts_for_program_call(program, vec![0, 0, 1, 0]);
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

    check_receipts_for_program_call(program, vec![0, 0, 1, 0]);
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

    check_receipts_for_program_call(program, vec![0, 1, 0, 0]);
}

fn check_receipts_for_program_call(program: Vec<Instruction>, expected_values: Vec<Word>) -> bool {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::default();

    let salt: Salt = rng.gen();

    let program: Witness = program.into_iter().collect::<Vec<u8>>().into();

    let contract = Contract::from(program.as_ref());
    let contract_root = contract.root();
    let state_root = Contract::default_state_root();
    let contract = contract.id(&salt, &contract_root, &state_root);

    let output = Output::contract_created(contract, state_root);

    let bytecode_witness = 0;
    let tx_deploy = Transaction::create(
        gas_price,
        gas_limit,
        maturity,
        bytecode_witness,
        salt,
        vec![],
        vec![],
        vec![output],
        vec![program],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to check tx");

    let input = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), contract);
    let output = Output::contract(0, rng.gen(), rng.gen());

    // The script needs to locate the data offset at runtime. Hence, we need to know
    // upfront the serialized size of the script so we can set the registers
    // accordingly.
    //
    // This variable is created to assert we have correct script size in the
    // instructions.
    let script_len = 16;

    // Based on the defined script length, we set the appropriate data offset
    let script_data_offset = client.tx_offset() + Script::script_offset_static() + script_len;
    let script_data_offset = script_data_offset as Immediate18;

    let script = vec![
        op::movi(0x10, script_data_offset),
        op::call(0x10, RegId::ZERO, RegId::ZERO, RegId::CGAS),
        op::ret(RegId::ONE),
    ]
    .into_iter()
    .collect::<Vec<u8>>();

    // Assert the offsets are set correctly
    let offset = client.tx_offset() + Script::script_offset_static() + bytes::padded_len(script.as_slice());
    assert_eq!(script_data_offset, offset as Immediate18);

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
    script_data.extend(contract.as_ref());
    script_data.extend(routine.to_be_bytes());
    script_data.extend(call_data_offset.to_be_bytes());
    script_data.extend(key.as_ref());
    script_data.extend(val.to_be_bytes());

    let tx_add_word = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script,
        script_data,
        vec![input],
        vec![output],
        vec![],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to check tx");

    // Assert the initial state of `key` is empty
    let state = client.as_ref().contract_state(&contract, &key);
    assert_eq!(Bytes32::default(), state.into_owned());

    client.deploy(tx_deploy);
    client.transact(tx_add_word);

    let receipts = client.receipts().expect("The transaction was executed");

    // Expect the correct receipt
    assert_eq!(receipts[1].ra().expect("Register value expected"), expected_values[0]);
    assert_eq!(receipts[1].rb().expect("Register value expected"), expected_values[1]);
    assert_eq!(receipts[1].rc().expect("Register value expected"), expected_values[2]);
    assert_eq!(receipts[1].rd().expect("Register value expected"), expected_values[3]);

    true
}

#[test]
fn state_r_word_b_plus_32_over() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let state_read_word = vec![
        op::xor(reg_a, reg_a, reg_a),
        op::not(reg_a, reg_a),
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
        op::xor(reg_a, reg_a, reg_a),
        op::ori(reg_a, reg_a, 1),
        op::slli(reg_a, reg_a, MAX_MEM_SHL),
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
        op::xor(reg_a, reg_a, reg_a),
        op::not(reg_a, reg_a),
        op::subi(reg_a, reg_a, 31 as Immediate12),
        op::srwq(reg_a, SET_STATUS_REG, RegId::ZERO, RegId::ONE),
    ];

    check_expected_reason_for_instructions(state_read_qword, MemoryOwnership);
}

#[test]
fn state_r_qword_c_plus_32_over() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let state_read_qword = vec![
        op::movi(0x11, 100),
        op::aloc(0x11),
        op::xor(reg_a, reg_a, reg_a),
        op::not(reg_a, reg_a),
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
        op::xor(reg_a, reg_a, reg_a),
        op::ori(reg_a, reg_a, 1),
        op::slli(reg_a, reg_a, MAX_MEM_SHL),
        op::subi(reg_a, reg_a, 31 as Immediate12),
        op::srwq(reg_a, SET_STATUS_REG, RegId::ZERO, RegId::ONE),
    ];

    check_expected_reason_for_instructions(state_read_qword, MemoryOwnership);
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
        op::xor(reg_a, reg_a, reg_a),
        op::ori(reg_a, reg_a, 1),
        op::slli(reg_a, reg_a, MAX_MEM_SHL),
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
        op::xor(reg_a, reg_a, reg_a),
        op::not(reg_a, reg_a),
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
        op::xor(reg_a, reg_a, reg_a),
        op::ori(reg_a, reg_a, 1),
        op::slli(reg_a, reg_a, MAX_MEM_SHL),
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
        op::xor(reg_a, reg_a, reg_a),
        op::not(reg_a, reg_a),
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
        op::xor(reg_a, reg_a, reg_a),
        op::not(reg_a, reg_a),
        op::subi(reg_a, reg_a, 31 as Immediate12),
        op::swwq(RegId::ZERO, SET_STATUS_REG, reg_a, RegId::ONE),
    ];

    check_expected_reason_for_instructions(state_write_qword, ArithmeticOverflow);
}

#[test]
fn state_w_qword_a_over_max_ram() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;

    // cover contract_id_end beyond max ram
    let state_write_qword = vec![
        op::xor(reg_a, reg_a, reg_a),
        op::ori(reg_a, reg_a, 1),
        op::slli(reg_a, reg_a, MAX_MEM_SHL),
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
        op::xor(reg_a, reg_a, reg_a),
        op::ori(reg_a, reg_a, 1),
        op::slli(reg_a, reg_a, MAX_MEM_SHL),
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
        op::xor(reg_a, reg_a, reg_a), // r[a] = 0
        op::ori(reg_a, reg_a, 1),     // r[a] = 1
        op::slli(reg_a, reg_a, 20),   // r[a] = 2^20
        op::addi(reg_a, reg_a, 1),    // r[a] = 2^20 + 1
        op::smo(RegId::ZERO, reg_a, RegId::ZERO, RegId::ZERO),
    ];

    check_expected_reason_for_instructions(message_output, MemoryOverflow);
}

#[test]
fn message_output_a_b_over() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;
    let reg_b = 0x21;

    // cover contract_id_end beyond max ram
    let message_output = vec![
        op::xor(reg_a, reg_a, reg_a), //r[a] = 0
        op::xor(reg_b, reg_b, reg_b), //r[b] = 0
        op::not(reg_a, reg_a),        //r[a] = MAX
        op::addi(reg_b, reg_b, 1),    //r[b] = 1
        op::smo(reg_a, reg_b, RegId::ZERO, RegId::ZERO),
    ];

    check_expected_reason_for_instructions(message_output, MemoryOverflow);
}

#[test]
fn message_output_a_b_gt_max_mem() {
    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;
    let reg_b = 0x21;

    // cover contract_id_end beyond max ram
    let message_output = vec![
        op::xor(reg_a, reg_a, reg_a),
        op::xor(reg_b, reg_b, reg_b),
        op::ori(reg_a, reg_a, 1),
        op::slli(reg_a, reg_a, MAX_MEM_SHL),
        op::addi(reg_b, reg_b, 1),
        op::smo(reg_a, reg_b, RegId::ZERO, RegId::ZERO),
    ];

    check_expected_reason_for_instructions(message_output, MemoryOverflow);
}

#[test]
fn smo_instruction_works() {
    fn execute_test<R>(rng: &mut R, inputs: Vec<(u64, Vec<u8>)>, message_output_amount: Word, gas_price: Word) -> bool
    where
        R: Rng,
    {
        let mut client = MemoryClient::default();

        let gas_limit = 1_000_000;
        let maturity = 0;
        let block_height = 0;

        let params = client.params();

        let secret = SecretKey::random(rng);
        let sender = rng.gen();

        #[rustfmt::skip]
        let script = vec![
            op::movi(0x10, 0),                          // set the txid as recipient
            op::movi(0x11, 1),                          // send the whole data buffer
            op::movi(0x12, 0),                          // tx output idx
            op::movi(0x13, message_output_amount as Immediate24),      // expected output amount
            op::smo(0x10,0x11,0x12,0x13),
            op::ret(RegId::ONE)
        ];

        let script = script.into_iter().collect();
        let script_data = vec![];

        let mut tx = TransactionBuilder::script(script, script_data);
        tx.gas_price(gas_price).gas_limit(gas_limit).maturity(maturity);
        // add inputs
        for (amount, data) in inputs {
            tx.add_unsigned_message_input(secret, sender, rng.gen(), amount, data);
        }
        let tx = tx
            .add_output(Output::Change {
                to: Default::default(),
                amount: 0,
                asset_id: Default::default(),
            })
            .finalize_checked(block_height, params, client.gas_costs());

        let non_retryable_free_balance = tx.metadata().non_retryable_balances[&AssetId::BASE];
        let retryable_balance: u64 = tx.metadata().retryable_balance.into();

        let txid = tx.transaction().id();
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
        // TODO: Add check for the `data` field too, but it requires fixing of the `smo` behaviour.
        let message_receipt = state.receipts().iter().find_map(|o| match o {
            Receipt::MessageOut { recipient, amount, .. } => Some((*recipient, *amount)),
            _ => None,
        });
        assert_eq!(message_receipt.is_some(), success);
        if let Some((recipient, transferred)) = message_receipt {
            assert_eq!(txid.as_ref(), recipient.as_ref());
            assert_eq!(message_output_amount, transferred);
        }
        // get gas used from script result
        let gas_used = if let Receipt::ScriptResult { gas_used, .. } = state.receipts().last().unwrap() {
            gas_used
        } else {
            panic!("expected script result")
        };
        // get refunded fee amount
        let refund_amount = TransactionFee::gas_refund_value(client.params(), *gas_used, gas_price).unwrap();

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
    assert!(!execute_test(rng, vec![(10, vec![0xfa; 15]), (1000, vec![])], 1011, 1));

    // check that retryable balance is refunded to user
    assert!(execute_test(rng, vec![(10, vec![0xfa; 15]), (1000, vec![])], 50, 1));
}

#[test]
fn timestamp_works() {
    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let block_height = 0;

    let params = *client.params();

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
        client.as_mut().set_block_height(height);

        let expected = client.as_ref().timestamp(input).expect("failed to calculate timestamp");

        #[rustfmt::skip]
        let script = vec![
            op::movi(0x11, input),              // set the argument
            op::time(0x10, 0x11),               // perform the instruction
            op::log(0x10, 0x00, 0x00, 0x00),    // log output
            op::ret(RegId::ONE)
        ];

        let script = script.into_iter().collect();
        let script_data = vec![];

        let tx = TransactionBuilder::script(script, script_data)
            .gas_price(gas_price)
            .gas_limit(gas_limit)
            .maturity(maturity)
            .finalize_checked(block_height, &params, client.gas_costs());

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
