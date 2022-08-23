use fuel_crypto::{Hasher, SecretKey};
use fuel_tx::{TransactionBuilder};
use fuel_types::bytes;
// use fuel_vm::script_with_data_offset;
// use fuel_vm::util::test_helpers::TestBuilder;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use fuel_vm::consts::*;
use fuel_vm::prelude::*;

use std::mem;
use fuel_asm::PanicReason::{ContractNotInInputs, ExpectedUnallocatedStack, MemoryOverflow};

const WORD_SIZE: usize = mem::size_of::<Word>();

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
    let function_selector: Vec<Opcode> = vec![
        Opcode::MOVE(0x30,  REG_ZERO),
        Opcode::MOVE(0x31, REG_ONE),
    ];

    #[rustfmt::skip]
    let call_arguments_parser: Vec<Opcode> = vec![
        Opcode::ADDI(0x10, REG_FP, CallFrame::a_offset() as Immediate12),
        Opcode::LW(0x10, 0x10, 0),
        Opcode::ADDI(0x11, REG_FP, CallFrame::b_offset() as Immediate12),
        Opcode::LW(0x11, 0x11, 0),
    ];

    #[rustfmt::skip]
    let routine_add_word_to_state: Vec<Opcode> = vec![
        Opcode::JNEI(0x10, 0x30, 13),       // (0, b) Add word to state
        Opcode::LW(0x20, 0x11, 4),          // r[0x20]      := m[b+32, 8]
        Opcode::SRW(0x21, 0x11),            // r[0x21]      := s[m[b, 32], 8]
        Opcode::ADD(0x20, 0x20, 0x21),      // r[0x20]      += r[0x21]
        Opcode::SWW(0x11, 0x20),            // s[m[b,32]]   := r[0x20]
        Opcode::LOG(0x20, 0x21, 0x00, 0x00),
        Opcode::RET(REG_ONE),
    ];

    #[rustfmt::skip]
    let routine_unpack_and_xor_limbs_into_state: Vec<Opcode> = vec![
        Opcode::JNEI(0x10, 0x31, 45),       // (1, b) Unpack arg into 4x16 and xor into state
        Opcode::MOVI(0x20, 32),   // r[0x20]      := 32
        Opcode::ALOC(0x20),                 // aloc            0x20
        Opcode::ADDI(0x20, REG_HP, 1),      // r[0x20]      := $hp+1
        Opcode::SRWQ(0x20, 0x11),           // m[0x20,32]   := s[m[b, 32], 32]
        Opcode::LW(0x21, 0x11, 4),          // r[0x21]      := m[b+32, 8]
        Opcode::LOG(0x21, 0x00, 0x00, 0x00),
        Opcode::SRLI(0x22, 0x21, 48),       // r[0x22]      := r[0x21] >> 48
        Opcode::SRLI(0x23, 0x21, 32),       // r[0x23]      := r[0x21] >> 32
        Opcode::ANDI(0x23, 0x23, 0xff),     // r[0x23]      &= 0xffff
        Opcode::SRLI(0x24, 0x21, 16),       // r[0x24]      := r[0x21] >> 16
        Opcode::ANDI(0x24, 0x24, 0xff),     // r[0x24]      &= 0xffff
        Opcode::ANDI(0x25, 0x21, 0xff),     // r[0x25]      := r[0x21] & 0xffff
        Opcode::LOG(0x22, 0x23, 0x24, 0x25),
        Opcode::LW(0x26, 0x20, 0),          // r[0x26]      := m[$fp, 8]
        Opcode::XOR(0x26, 0x26, 0x22),      // r[0x26]      ^= r[0x22]
        Opcode::LOG(0x26, 0x00, 0x00, 0x00),
        Opcode::SW(0x20, 0x26, 0),          // m[0x20,8]    := r[0x26]
        Opcode::LW(0x26, 0x20, 1),          // r[0x26]      := m[$fp+8, 8]
        Opcode::XOR(0x26, 0x26, 0x22),      // r[0x26]      ^= r[0x22]
        Opcode::LOG(0x26, 0x00, 0x00, 0x00),
        Opcode::SW(0x20, 0x26, 1),          // m[0x20+8,8]  := r[0x26]
        Opcode::LW(0x26, 0x20, 2),          // r[0x26]      := m[$fp+16, 8]
        Opcode::XOR(0x26, 0x26, 0x22),      // r[0x26]      ^= r[0x22]
        Opcode::LOG(0x26, 0x00, 0x00, 0x00),
        Opcode::SW(0x20, 0x26, 2),          // m[0x20+16,8] := r[0x26]
        Opcode::LW(0x26, 0x20, 3),          // r[0x26]      := m[$fp+24, 8]
        Opcode::XOR(0x26, 0x26, 0x22),      // r[0x26]      ^= r[0x22]
        Opcode::LOG(0x26, 0x00, 0x00, 0x00),
        Opcode::SW(0x20, 0x26, 3),          // m[0x20+24,8] := r[0x26]
        Opcode::SWWQ(0x11, 0x20),           // s[m[b,32],32]:= m[0x20, 32]
        Opcode::RET(REG_ONE),
    ];

    #[rustfmt::skip]
    let invalid_call: Vec<Opcode> = vec![
        Opcode::RET(REG_ZERO),
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
    .check(height, &params)
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
    let script_data_offset = client.tx_offset() + Transaction::script_offset() + script_len;
    let script_data_offset = script_data_offset as Immediate18;

    let script = vec![
        Opcode::MOVI(0x10, script_data_offset),
        Opcode::CALL(0x10, REG_ZERO, REG_ZERO, REG_CGAS),
        Opcode::RET(REG_ONE),
    ]
    .iter()
    .copied()
    .collect::<Vec<u8>>();

    // Assert the offsets are set correctnly
    let offset = client.tx_offset() + Transaction::script_offset() + bytes::padded_len(script.as_slice());
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
    script_data.extend(&routine.to_be_bytes());
    script_data.extend(&call_data_offset.to_be_bytes());
    script_data.extend(key.as_ref());
    script_data.extend(&val.to_be_bytes());

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
    .check(height, &params)
    .expect("failed to check tx");

    // Assert the initial state of `key` is empty
    let state = client.as_ref().contract_state(&contract, &key);
    assert_eq!(Bytes32::default(), state.into_owned());

    client.transact(tx_deploy);
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
    script_data.extend(&routine.to_be_bytes());
    script_data.extend(&call_data_offset.to_be_bytes());
    script_data.extend(key.as_ref());
    script_data.extend(&val.to_be_bytes());

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
    .check(height, &params)
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
    let n = a ^ 0x00;
    let o = a ^ 0x00;
    let p = a ^ 0x00;

    // Expect the value to be unpacked correctly into 4x16 limbs + XOR state
    assert_eq!(receipts[3].ra().expect("Register value expected"), m);
    assert_eq!(receipts[4].ra().expect("Register value expected"), n);
    assert_eq!(receipts[5].ra().expect("Register value expected"), o);
    assert_eq!(receipts[6].ra().expect("Register value expected"), p);

    let mut bytes = [0u8; 32];

    // Reconstruct the final state out of the limbs
    (&mut bytes[..8]).copy_from_slice(&m.to_be_bytes());
    (&mut bytes[8..16]).copy_from_slice(&n.to_be_bytes());
    (&mut bytes[16..24]).copy_from_slice(&o.to_be_bytes());
    (&mut bytes[24..]).copy_from_slice(&p.to_be_bytes());

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
    let contract_code: Vec<Opcode> = vec![
        Opcode::LOG(REG_ONE, REG_ONE, REG_ONE, REG_ONE),
        Opcode::RET(REG_ONE),
        Opcode::RET(REG_ZERO), // Pad to make length uneven to test padding
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
    .check(height, &params)
    .expect("failed to check tx");

    client.transact(tx_create_target);

    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;
    let reg_b = 0x21;

    let count = ContractId::LEN as Immediate12;

    let mut load_contract: Vec<Opcode> = vec![
        Opcode::XOR(reg_a, reg_a, reg_a), // r[a] := 0
        Opcode::ORI(reg_a, reg_a, count), // r[a] := r[a] | ContractId::LEN
        Opcode::ALOC(reg_a),              // Reserve space for contract id in the heap
    ];

    // Generate code for pushing contract id to heap
    for (i, byte) in contract_id.as_ref().iter().enumerate() {
        let index = i as Immediate12;
        let value = *byte as Immediate12;
        load_contract.extend(&[
            Opcode::XOR(reg_a, reg_a, reg_a),     // r[a] := 0
            Opcode::ORI(reg_a, reg_a, value),     // r[a] := r[a] | value
            Opcode::SB(REG_HP, reg_a, index + 1), // m[$hp+index+1] := r[a] (=value)
        ]);
    }

    load_contract.extend(vec![
        Opcode::MOVE(reg_a, REG_HP),                    // r[a] := $hp
        Opcode::ADDI(reg_a, reg_a, 1),                  // r[a] += 1
        Opcode::XOR(reg_b, reg_b, reg_b),               // r[b] := 0
        Opcode::ORI(reg_b, reg_b, 12),                  // r[b] += 12 (will be padded to 16)
        Opcode::LDC(reg_a, REG_ZERO, reg_b),            // Load first two words from the contract
        Opcode::MOVE(reg_a, REG_SSP),                   // r[b] := $ssp
        Opcode::SUBI(reg_a, reg_a, 8 * 2),              // r[a] -= 16 (start of the loaded code)
        Opcode::XOR(reg_b, reg_b, reg_b),               // r[b] := 0
        Opcode::ADDI(reg_b, reg_b, 16),                 // r[b] += 16 (lenght of the loaded code)
        Opcode::LOGD(REG_ZERO, REG_ZERO, reg_a, reg_b), // Log digest of the loaded code
        Opcode::NOOP,                                   // Patched to the jump later
    ]);

    let tx_deploy_loader = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        load_contract.clone().into_iter().collect(),
        vec![],
        vec![input0.clone()],
        vec![output1],
        vec![],
    )
    .check(height, &params)
    .expect("failed to check tx");

    // Patch the code with correct jump address
    let transaction_end_addr = tx_deploy_loader.transaction().serialized_size() - Transaction::script_offset();
    *load_contract.last_mut().unwrap() = Opcode::JI((transaction_end_addr / 4) as Immediate24);

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
    .check(height, &params)
    .expect("failed to check tx");

    let receipts = client.transact(tx_deploy_loader);

    if let Receipt::LogData { digest, .. } = receipts.get(0).expect("No receipt") {
        let mut code = program.into_inner();
        code.extend(&[0; 4]);
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
fn load_contract_code_invalid_args() {
    let rng = &mut StdRng::seed_from_u64(2322u64);
    let salt: Salt = rng.gen();

    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::default();

    // Start by creating and deploying a new example contract
    let contract_code: Vec<Opcode> = vec![
        Opcode::LOG(REG_ONE, REG_ONE, REG_ONE, REG_ONE),
        Opcode::RET(REG_ONE),
        Opcode::RET(REG_ZERO), // Pad to make length uneven to test padding
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
        .check(height, &params)
        .expect("failed to check tx");

    client.transact(tx_create_target);

    // Then deploy another contract that attempts to read the first one
    let reg_a = 0x20;
    let reg_b = 0x21;

    //test ssp != sp for LDC
    let load_contract: Vec<Opcode> = vec![
        Opcode::CFEI(0x1),                    // sp += 1
        Opcode::LDC(reg_a, REG_ZERO, reg_b),            // Load first two words from the contract
    ];

    let tx_deploy_loader = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        load_contract.into_iter().collect(),
        vec![],
        vec![],
        vec![],
        vec![],
    )
        .check(height, &params)
        .expect("failed to check tx");

    let receipts = client.transact(tx_deploy_loader);

    if let Receipt::Panic { id:_, reason, .. } = receipts.get(0).expect("No receipt") {
        assert_eq!(&ExpectedUnallocatedStack, reason.reason(), "Expected ExpectedUnallocatedStack, found {}", reason.reason());
    } else {
        panic!("Script should have panicked");
    }

    //test memory offset above reg_hp value
    let load_contract: Vec<Opcode> = vec![
        Opcode::MOVE(reg_b, REG_HP),                    // r[a] := $hp
        Opcode::LDC(reg_a, REG_ZERO, reg_b),            // Load first two words from the contract
    ];

    let tx_deploy_loader = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        load_contract.into_iter().collect(),
        vec![],
        vec![],
        vec![],
        vec![],
    )
        .check(height, &params)
        .expect("failed to check tx");

    let receipts = client.transact(tx_deploy_loader);

    if let Receipt::Panic { id:_, reason, .. } = receipts.get(0).expect("No receipt") {
        assert_eq!(&MemoryOverflow, reason.reason(), "Expected MemoryOverflow, found {}", reason.reason());
    } else {
        panic!("Script should have panicked");
    }

    // cover contract_id_end beyond max ram
    let load_contract: Vec<Opcode> = vec![
        Opcode::MOVE(reg_a, REG_HP),                    // r[a] := $hp
        Opcode::XOR(reg_b, reg_b, reg_b),               // r[b] := 0
        Opcode::ORI(reg_b, reg_b, 12),                  // r[b] += 12 (will be padded to 16)
        Opcode::LDC(reg_a, REG_ZERO, reg_b),            // Load first two words from the contract
    ];

    let tx_deploy_loader = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        load_contract.into_iter().collect(),
        vec![],
        vec![],
        vec![],
        vec![],
    )
        .check(height, &params)
        .expect("failed to check tx");

    let receipts = client.transact(tx_deploy_loader);

    if let Receipt::Panic { id:_, reason, .. } = receipts.get(0).expect("No receipt") {
        assert_eq!(&MemoryOverflow, reason.reason(), "Expected MemoryOverflow, found {}", reason.reason());
    } else {
        panic!("Script should have panicked");
    }

    // cover contract_id_end beyond max ram
    let load_contract: Vec<Opcode> = vec![
        Opcode::XOR(reg_a, reg_a, reg_a),               // r[b] := 0
        // Opcode::MOVE(reg_a, REG_HP),                    // r[a] := $hp
        Opcode::ADDI(reg_a, reg_a, 1),                  // r[a] += 1
        Opcode::XOR(reg_b, reg_b, reg_b),               // r[b] := 0
        Opcode::ORI(reg_b, reg_b, 12),                  // r[b] += 12 (will be padded to 16)
        Opcode::LDC(reg_a, REG_ZERO, reg_b),            // Load first two words from the contract
    ];

    let tx_deploy_loader = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        load_contract.into_iter().collect(),
        vec![],
        vec![],
        vec![],
        vec![],
    )
        .check(height, &params)
        .expect("failed to check tx");

    let receipts = client.transact(tx_deploy_loader);

    if let Receipt::Panic { id:_, reason, .. } = receipts.get(0).expect("No receipt") {
        assert_eq!(&ContractNotInInputs, reason.reason(), "Expected ContractNotInInputs, found {}", reason.reason());
    } else {
        panic!("Script should have panicked");
    }

    let count = ContractId::LEN as Immediate12;

    let mut load_contract: Vec<Opcode> = vec![
        Opcode::XOR(reg_a, reg_a, reg_a), // r[a] := 0
        Opcode::ORI(reg_a, reg_a, count), // r[a] := r[a] | ContractId::LEN
        Opcode::ALOC(reg_a),              // Reserve space for contract id in the heap
    ];

    // Generate code for pushing contract id to heap
    for (i, byte) in contract_id.as_ref().iter().enumerate() {
        let index = i as Immediate12;
        let value = *byte as Immediate12;
        load_contract.extend(&[
            Opcode::XOR(reg_a, reg_a, reg_a),     // r[a] := 0
            Opcode::ORI(reg_a, reg_a, value),     // r[a] := r[a] | value
            Opcode::SB(REG_HP, reg_a, index + 1), // m[$hp+index+1] := r[a] (=value)
        ]);
    }

    load_contract.extend(vec![
        Opcode::MOVE(reg_a, REG_HP),                    // r[a] := $hp
        Opcode::ADDI(reg_a, reg_a, 1),                  // r[a] += 1
        Opcode::XOR(reg_b, reg_b, reg_b),               // r[b] := 0
        Opcode::ORI(reg_b, reg_b, 12),                  // r[b] += 12 (will be padded to 16)
        Opcode::LDC(reg_a, reg_a, reg_b),            // Load first two words from the contract
        Opcode::MOVE(reg_a, REG_SSP),                   // r[b] := $ssp
        Opcode::SUBI(reg_a, reg_a, 8 * 2),              // r[a] -= 16 (start of the loaded code)
        Opcode::XOR(reg_b, reg_b, reg_b),               // r[b] := 0
        Opcode::ADDI(reg_b, reg_b, 16),                 // r[b] += 16 (lenght of the loaded code)
        Opcode::LOGD(REG_ZERO, REG_ZERO, reg_a, reg_b), // Log digest of the loaded code
        Opcode::NOOP,                                   // Patched to the jump later
    ]);

    let tx_deploy_loader = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        load_contract.clone().into_iter().collect(),
        vec![],
        vec![input0.clone()],
        vec![output1],
        vec![],
    )
        .check(height, &params)
        .expect("failed to check tx");

    // Patch the code with correct jump address
    let transaction_end_addr = tx_deploy_loader.transaction().serialized_size() - Transaction::script_offset();
    *load_contract.last_mut().unwrap() = Opcode::JI((transaction_end_addr / 4) as Immediate24);

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
        .check(height, &params)
        .expect("failed to check tx");

    let receipts = client.transact(tx_deploy_loader);

    if let Receipt::Panic { id:_, reason, .. } = receipts.get(0).expect("No receipt") {
        assert_eq!(&MemoryOverflow, reason.reason(), "Expected MemoryOverflow, found {}", reason.reason());
    } else {
        panic!("Script should have panicked");
    }
}

#[test]
fn smo_instruction_works() {
    fn execute_test<R>(rng: &mut R, balance: Word, amount: Word, data: Vec<u8>)
    where
        R: Rng,
    {
        let mut client = MemoryClient::default();

        let gas_price = 1;
        let gas_limit = 1_000_000;
        let maturity = 0;
        let block_height = 0;

        let params = client.params();

        let secret = SecretKey::random(rng);
        let sender = rng.gen();
        let recipient = rng.gen();
        let nonce = rng.gen();

        let message = Output::message(Address::zeroed(), 0);

        #[rustfmt::skip]
        let script = vec![
            Opcode::MOVI(0x10, 0),                          // set the txid as recipient
            Opcode::MOVI(0x11, data.len() as Immediate24),  // send the whole data buffer
            Opcode::MOVI(0x12, 0),                          // tx output idx
            Opcode::MOVI(0x13, amount as Immediate24),      // expected output amount
            Opcode::SMO(0x10,0x11,0x12,0x13),
            Opcode::RET(REG_ONE)
        ];

        let script = script.into_iter().collect();
        let script_data = vec![];

        let tx = TransactionBuilder::script(script, script_data)
            .gas_price(gas_price)
            .gas_limit(gas_limit)
            .maturity(maturity)
            .add_unsigned_message_input(secret, sender, recipient, nonce, balance, data)
            .add_output(message)
            .finalize_checked(block_height, params);

        let txid = tx.transaction().id();
        let receipts = client.transact(tx);

        let success = receipts.iter().any(|r| match r {
            Receipt::ScriptResult {
                result: ScriptExecutionResult::Success,
                ..
            } => true,
            _ => false,
        });

        assert!(success);

        let (recipient, transferred) = client
            .state_transition()
            .expect("tx was executed")
            .tx()
            .outputs()
            .iter()
            .find_map(|o| match o {
                Output::Message { recipient, amount } => Some((recipient, *amount)),
                _ => None,
            })
            .expect("failed to find message output");

        assert_eq!(txid.as_ref(), recipient.as_ref());
        assert_eq!(amount, transferred);
    }

    let rng = &mut StdRng::seed_from_u64(2322u64);

    // check arbitrary amount
    execute_test(rng, 1_000, 10, vec![0xfa; 15]);

    // check message with zero amount
    execute_test(rng, 1_000, 0, vec![0xfa; 15]);
}

#[test]
fn timestamp_works() {
    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let block_height = 0;

    let params = client.params().clone();

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
            Opcode::MOVI(0x11, input),              // set the argument
            Opcode::TIME(0x10, 0x11),               // perform the instruction
            Opcode::LOG(0x10, 0x00, 0x00, 0x00),    // log output
            Opcode::RET(REG_ONE)
        ];

        let script = script.into_iter().collect();
        let script_data = vec![];

        let tx = TransactionBuilder::script(script, script_data)
            .gas_price(gas_price)
            .gas_limit(gas_limit)
            .maturity(maturity)
            .finalize_checked(block_height, &params);

        let receipts = client.transact(tx);
        let result = receipts.iter().any(|r| match r {
            Receipt::ScriptResult {
                result: ScriptExecutionResult::Success,
                ..
            } => true,
            _ => false,
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
