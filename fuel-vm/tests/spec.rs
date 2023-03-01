use fuel_asm::*;
use fuel_tx::{ConsensusParameters, Receipt, ScriptExecutionResult, Transaction, Witness, Contract, Output, Input, field::ScriptData};
use fuel_types::{Immediate18, Salt, AssetId, Immediate12};
use fuel_vm::{
    consts::VM_MAX_RAM,
    prelude::{IntoChecked, MemoryClient, Call, SerializableVec},
};

use rand::{rngs::StdRng, SeedableRng, Rng};
use rstest::rstest;

mod test_helpers;

/// Assert that transaction didn't panic
fn assert_success(receipts: &[Receipt]) {
    if let Receipt::ScriptResult { result, .. } = receipts.last().unwrap() {
        if *result != ScriptExecutionResult::Success {
            panic!("Expected vm success, got {result:?} instead");
        }
    } else {
        unreachable!("No script result");
    }
}

/// Assert that transaction receipts end in a panic with the given reason
fn assert_panics(receipts: &[Receipt], reason: PanicReason) {
    if let Receipt::ScriptResult { result, .. } = receipts.last().unwrap() {
        if *result != ScriptExecutionResult::Panic {
            panic!("Expected vm panic, got {result:?} instead");
        }
    } else {
        unreachable!("No script result");
    }

    let n = receipts.len();
    assert!(n >= 2, "Invalid receipts len");
    if let Receipt::Panic { reason: pr, .. } = receipts.get(n - 2).unwrap() {
        assert_eq!(*pr.reason(), reason, "Panic reason differs for the expected reason");
    } else {
        unreachable!("No script receipt for a paniced tx");
    }
}

/// Setup some useful values
/// * 0x31 to all ones, i.e. max word
/// * 0x32 to two
fn common_setup() -> Vec<Instruction> {
    let mut setup = vec![op::not(0x31, RegId::ZERO), op::movi(0x32, 2)];

    setup.extend(&test_helpers::set_full_word(0x33, VM_MAX_RAM));
    setup.extend(&test_helpers::set_full_word(
        0x34,
        VM_MAX_RAM / (Instruction::SIZE as u64),
    ));

    setup
}

fn run_script(script: Vec<u8>) -> Vec<Receipt> {
    let mut client = MemoryClient::default();
    let tx = Transaction::script(0, 1_000_000, 0, script, vec![], vec![], vec![], vec![])
        .into_checked(0, &ConsensusParameters::DEFAULT, client.gas_costs())
        .expect("failed to generate a checked tx");
    client.transact(tx);
    client.receipts().expect("Expected receipts").to_vec()
}

#[rstest]
fn spec_unsafemath_flag(
    #[values(
        op::div(0x10, RegId::ZERO, RegId::ZERO),
        op::divi(0x10, RegId::ZERO, 0),
        op::mlog(0x10, 0x31, RegId::ZERO),
        op::mod_(0x10, 0x31, RegId::ZERO),
        op::modi(0x10, 0x31, 0),
        op::mroo(0x10, 0x31, RegId::ZERO)
    )]
    case: Instruction,
    #[values(true, false)] flag: bool,
) {
    let mut script = common_setup();
    if flag {
        script.extend(&[op::addi(0x10, RegId::ZERO, 0x01), op::flag(0x10)]);
    }
    script.push(case);
    script.push(op::log(RegId::IS, RegId::PC, RegId::OF, RegId::ERR));
    script.push(op::ret(RegId::ONE));

    let receipts = run_script(script.into_iter().collect());

    if flag {
        if let Receipt::Log { rd: err, .. } = receipts[0] {
            assert_eq!(err, 1);
        } else {
            panic!("No log data");
        }
    } else {
        assert_panics(&receipts, PanicReason::ErrorFlag);
    }
}

#[rstest]
fn spec_wrapping_flag(
    #[values(
        op::add(0x10, 0x31, 0x31),
        op::addi(0x10, 0x31, 0x31),
        op::exp(0x10, 0x31, 0x31),
        op::expi(0x10, 0x31, 0x31),
        op::mul(0x10, 0x31, 0x31),
        op::muli(0x10, 0x31, 0x31),
        op::sub(0x10, RegId::ZERO, RegId::ONE),
        op::subi(0x10, RegId::ZERO, 1)
    )]
    case: Instruction,
    #[values(true, false)] flag: bool,
) {
    let mut script = common_setup();
    if flag {
        script.extend(&[op::addi(0x10, RegId::ZERO, 0x02), op::flag(0x10)]);
    }
    script.push(case);
    script.push(op::log(RegId::IS, RegId::PC, RegId::OF, RegId::ERR));
    script.push(op::ret(RegId::ONE));

    let receipts = run_script(script.into_iter().collect());

    if flag {
        if let Receipt::Log { rc: of, .. } = receipts[0] {
            assert_ne!(of, 0);
        } else {
            panic!("No log data");
        }
    } else {
        assert_panics(&receipts, PanicReason::ArithmeticOverflow);
    }
}

#[rstest]
fn spec_logic_ops_clear_of(
    #[values(
        op::and(0x10, RegId::ZERO, RegId::ONE),
        op::andi(0x10, RegId::ZERO, 1),
        op::eq(0x10, RegId::ZERO, RegId::ONE),
        op::gt(0x10, RegId::ZERO, RegId::ONE),
        op::lt(0x10, RegId::ZERO, RegId::ONE),
        op::move_(0x10, RegId::ONE),
        op::movi(0x10, 1),
        op::noop(),
        op::or(0x10, RegId::ZERO, RegId::ONE),
        op::ori(0x10, RegId::ZERO, 1),
        op::sll(0x10, RegId::ZERO, RegId::ONE),
        op::slli(0x10, RegId::ZERO, 1),
        op::srl(0x10, RegId::ZERO, RegId::ONE),
        op::srli(0x10, RegId::ZERO, 1),
        op::xor(0x10, RegId::ZERO, RegId::ONE),
        op::xori(0x10, RegId::ZERO, 1)
    )]
    case: Instruction,
) {
    let mut script = common_setup();
    script.extend(&[op::addi(0x10, RegId::ZERO, 0x02), op::flag(0x10)]);
    script.push(op::add(0x10, 0x31, 0x31)); // Set $of to nonzero
    script.push(op::log(RegId::IS, RegId::PC, RegId::OF, RegId::ERR));
    script.push(case); // Check that the logic op clears it
    script.push(op::log(RegId::IS, RegId::PC, RegId::OF, RegId::ERR));
    script.push(op::ret(RegId::ONE));

    let receipts = run_script(script.into_iter().collect());

    if let Receipt::Log { rc: of, .. } = receipts[0] {
        assert_ne!(of, 0);
    } else {
        panic!("No log data");
    }

    if let Receipt::Log { rc: of, .. } = receipts[1] {
        assert_eq!(of, 0);
    } else {
        panic!("No log data");
    }
}

#[rstest]
fn spec_alu_immediates_are_zero_extended(
    #[values(
        (op::addi(0x10, RegId::ZERO, Imm12::MAX.into()), Imm12::MAX.into()),
        (op::andi(0x10, 0x31, Imm12::MAX.into()), Imm12::MAX.into()),
        (op::divi(0x10, 0x31, Imm12::MAX.into()), u64::MAX / (Imm12::MAX.to_u16() as u64)),
        (op::expi(0x10, 1, Imm12::MAX.into()), 1), // pow(Imm12::MAX, 2) would overflow
        (op::modi(0x10, 0x31, Imm12::MAX.into()), u64::MAX % (Imm12::MAX.to_u16() as u64)),
        (op::movi(0x10, Imm18::MAX.into()), Imm18::MAX.into()),
        (op::muli(0x10, 0x32, Imm12::MAX.into()), 8190),
        (op::ori(0x10, RegId::ZERO, Imm12::MAX.into()), Imm12::MAX.into()),
        (op::slli(0x10, 0x31, Imm12::MAX.into()), 0), // These cases don't make much sense, since
        (op::srli(0x10, 0x31, Imm12::MAX.into()), 0), // shifting more than 64 bits is meaningless
        (op::subi(0x10, 0x31, Imm12::MAX.into()), u64::MAX - (Imm12::MAX.to_u16() as u64)),
        (op::xori(0x10, 0x31, Imm12::MAX.into()), u64::MAX ^ (Imm12::MAX.to_u16() as u64))
    )]
    case: (Instruction, u64),
) {
    let (op, expected) = case;

    let mut script = common_setup();
    script.push(op);
    script.push(op::log(0x10, RegId::ZERO, RegId::OF, RegId::ERR));
    script.push(op::ret(RegId::ONE));

    let receipts = run_script(script.into_iter().collect());

    if let Receipt::Log { ra, .. } = receipts[0] {
        assert_eq!(ra, expected);
    } else {
        panic!("No log data");
    }
}

#[rstest]
fn spec_logic_ops_clear_err(
    #[values(
        op::and(0x10, RegId::ZERO, RegId::ONE),
        op::andi(0x10, RegId::ZERO, 1),
        op::eq(0x10, RegId::ZERO, RegId::ONE),
        op::gt(0x10, RegId::ZERO, RegId::ONE),
        op::lt(0x10, RegId::ZERO, RegId::ONE),
        op::move_(0x10, RegId::ONE),
        op::movi(0x10, 1),
        op::noop(),
        op::or(0x10, RegId::ZERO, RegId::ONE),
        op::ori(0x10, RegId::ZERO, 1),
        op::sll(0x10, RegId::ZERO, RegId::ONE),
        op::slli(0x10, RegId::ZERO, 1),
        op::srl(0x10, RegId::ZERO, RegId::ONE),
        op::srli(0x10, RegId::ZERO, 1),
        op::xor(0x10, RegId::ZERO, RegId::ONE),
        op::xori(0x10, RegId::ZERO, 1)
    )]
    case: Instruction,
) {
    let mut script = common_setup();
    script.extend(&[op::addi(0x10, RegId::ZERO, 0x01), op::flag(0x10)]);
    script.push(op::div(0x10, RegId::ZERO, RegId::ZERO)); // Set $err to nonzero
    script.push(op::log(RegId::IS, RegId::PC, RegId::OF, RegId::ERR));
    script.push(case); // Check that the logic op clears it
    script.push(op::log(RegId::IS, RegId::PC, RegId::OF, RegId::ERR));
    script.push(op::ret(RegId::ONE));

    let receipts = run_script(script.into_iter().collect());

    if let Receipt::Log { rd: err, .. } = receipts[0] {
        assert_ne!(err, 0);
    } else {
        panic!("No log data");
    }

    if let Receipt::Log { rd: err, .. } = receipts[1] {
        assert_eq!(err, 0);
    } else {
        panic!("No log data");
    }
}

#[rstest]
fn spec_reserved_reg_write(
    #[values(
        op::add(0, 0, 0),
        op::addi(0, 0, 0),
        op::and(0, 0, 0),
        op::andi(0, 0, 0),
        op::div(0, 0, 0),
        op::divi(0, 0, 0),
        op::eq(0, 0, 0),
        op::exp(0, 0, 0),
        op::expi(0, 0, 0),
        op::gt(0, 0, 0),
        op::lt(0, 0, 0),
        op::mlog(0, 0, 0),
        op::mod_(0, 0, 0),
        op::modi(0, 0, 0),
        op::move_(0, 0),
        op::movi(0, 0),
        op::mroo(0, 0, 0),
        op::mul(0, 0, 0),
        op::muli(0, 0, 0),
        op::not(0, 0),
        op::or(0, 0, 0),
        op::ori(0, 0, 0),
        op::sll(0, 0, 0),
        op::slli(0, 0, 0),
        op::srl(0, 0, 0),
        op::srli(0, 0, 0),
        op::sub(0, 0, 0),
        op::subi(0, 0, 0),
        op::xor(0, 0, 0),
        op::xori(0, 0, 0)
    )]
    case: Instruction,
) {
    let mut script = common_setup();
    script.push(case);
    script.push(op::ret(RegId::ONE));

    let receipts = run_script(script.into_iter().collect());
    assert_panics(&receipts, PanicReason::ReservedRegisterNotWritable);
}

#[rstest]
fn spec_incr_pc_by_four(
    #[values(
        op::add(RegId::WRITABLE, 0, 0),
        op::addi(RegId::WRITABLE, 0, 0),
        op::and(RegId::WRITABLE, 0, 0),
        op::andi(RegId::WRITABLE, 0, 0),
        op::div(RegId::WRITABLE, 0, 1),
        op::divi(RegId::WRITABLE, 0, 1),
        op::eq(RegId::WRITABLE, 0, 0),
        op::exp(RegId::WRITABLE, 0, 0),
        op::expi(RegId::WRITABLE, 0, 0),
        op::gt(RegId::WRITABLE, 0, 0),
        op::lt(RegId::WRITABLE, 0, 0),
        op::mlog(RegId::WRITABLE, 1, 0x32),
        op::mod_(RegId::WRITABLE, 0, 1),
        op::modi(RegId::WRITABLE, 0, 1),
        op::move_(RegId::WRITABLE, 0),
        op::movi(RegId::WRITABLE, 0),
        op::mroo(RegId::WRITABLE, 0, 0x32),
        op::mul(RegId::WRITABLE, 0, 0),
        op::muli(RegId::WRITABLE, 0, 0),
        op::not(RegId::WRITABLE, 0),
        op::or(RegId::WRITABLE, 0, 0),
        op::ori(RegId::WRITABLE, 0, 0),
        op::sll(RegId::WRITABLE, 0, 0),
        op::slli(RegId::WRITABLE, 0, 0),
        op::srl(RegId::WRITABLE, 0, 0),
        op::srli(RegId::WRITABLE, 0, 0),
        op::sub(RegId::WRITABLE, 0, 0),
        op::subi(RegId::WRITABLE, 0, 0),
        op::xor(RegId::WRITABLE, 0, 0),
        op::xori(RegId::WRITABLE, 0, 0)
    )]
    case: Instruction,
    #[values(0, 1, 2)] offset: usize,
) {
    let mut script = common_setup();
    for _ in 0..offset {
        script.push(op::noop());
    }
    let setup_size = (script.len() as Word) * 4;
    script.push(case);
    script.push(op::log(RegId::IS, RegId::PC, RegId::OF, RegId::ERR));
    script.push(op::ret(RegId::ONE));

    let receipts = run_script(script.into_iter().collect());

    if let Receipt::Log { ra: is, rb: pc, .. } = receipts[0] {
        assert_eq!(is + setup_size + 4, pc);
    } else {
        panic!("No log data");
    }
}

#[rstest]
fn spec_jmp_beyond_ram_panics(
    #[values(
        op::jmp(0x20),
        op::ji(Imm24::MAX.into()),
        op::jne(RegId::ZERO, RegId::ONE, 0x20),
        // TODO: Test JNEI and JNZI instructions
        //       The immediates of these are 18 and 12 bits long, so they
        //       cannot hold large-enough values to go over the RAM limit,
        //       since the transaction size is limited. It could be possible
        //       to setup a suitable scenario by repeatedly calling contract
        //       until the $is is close enough to the RAM limit.
    )]
    case: Instruction,
) {
    let mut script = common_setup();
    // This is just beyond ram
    script.push(op::add(0x20, RegId::IS, 0x34));
    script.push(case);

    let receipts = run_script(script.into_iter().collect());
    assert_panics(&receipts, PanicReason::MemoryOverflow);
}

#[test]
fn spec_TODO_recursive_calls() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut balance = 1000;

    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::DEFAULT;

    let salt: Salt = rng.gen();
    let program: Witness = [
        op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS),
        op::ret(RegId::ONE),
    ]
    .into_iter()
    .collect::<Vec<u8>>()
    .into();

    let contract = Contract::from(program.as_ref());
    let contract_root = contract.root();
    let state_root = Contract::default_state_root();
    let contract = contract.id(&salt, &contract_root, &state_root);

    let asset_id = AssetId::from(*contract);
    let output = Output::contract_created(contract, state_root);

    let bytecode_witness = 0;
    let tx = Transaction::create(
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
    .expect("failed to generate checked tx");

    client.deploy(tx);

    let input = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), contract);
    let output = Output::contract(0, rng.gen(), rng.gen());

    let mut script_ops = vec![
        op::movi(0x10, 0),
        op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS),
        op::ret(RegId::ONE),
    ];

    let script: Vec<u8> = script_ops.clone().into_iter().collect();
    let tx = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script,
        vec![],
        vec![input.clone()],
        vec![output],
        vec![],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to generate checked tx");

    let script_data_offset = client.tx_offset() + tx.transaction().script_data_offset();
    script_ops[0] = op::movi(0x10, script_data_offset as Immediate18);

    let script: Vec<u8> = script_ops.clone().into_iter().collect();
    let script_data = Call::new(contract, 0, balance).to_bytes();
    let tx = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script,
        script_data,
        vec![input.clone()],
        vec![output],
        vec![],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to generate checked tx");

    let script_data_check_balance: Vec<u8> = asset_id
        .as_ref()
        .iter()
        .chain(contract.as_ref().iter())
        .copied()
        .collect();
    let mut script_check_balance = vec![
        op::noop(),
        op::move_(0x11, 0x10),
        op::addi(0x12, 0x10, AssetId::LEN as Immediate12),
        op::bal(0x10, 0x11, 0x12),
        op::log(0x10, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ];

    let tx_check_balance = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script_check_balance.clone().into_iter().collect(),
        vec![],
        vec![input.clone()],
        vec![output],
        vec![],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to generate checked tx");

    let script_data_offset = client.tx_offset() + tx_check_balance.transaction().script_data_offset();
    script_check_balance[0] = op::movi(0x10, script_data_offset as Immediate18);

    let tx_check_balance = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script_check_balance.into_iter().collect(),
        script_data_check_balance,
        vec![input.clone()],
        vec![output],
        vec![],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to generate checked tx");

    let storage_balance = client.transact(tx_check_balance.clone())[0]
        .ra()
        .expect("Balance expected");
    assert_eq!(0, storage_balance);

    client.transact(tx);

    let storage_balance = client.transact(tx_check_balance.clone())[0]
        .ra()
        .expect("Balance expected");
    assert_eq!(balance as Word, storage_balance);

    // Try to burn more than the available balance
    let script: Vec<u8> = script_ops.clone().into_iter().collect();
    let script_data = Call::new(contract, 1, balance + 1).to_bytes();
    let tx = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script,
        script_data,
        vec![input.clone()],
        vec![output],
        vec![],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to generate checked tx");

    let storage_balance = client.transact(tx_check_balance.clone())[0]
        .ra()
        .expect("Balance expected");
    assert_eq!(balance as Word, storage_balance);

    // Out of balance test
    client.transact(tx);

    let storage_balance = client.transact(tx_check_balance.clone())[0]
        .ra()
        .expect("Balance expected");
    assert_eq!(balance as Word, storage_balance);

    // Burn some of the balance
    let burn = 100;

    let script: Vec<u8> = script_ops.clone().into_iter().collect();
    let script_data = Call::new(contract, 1, burn).to_bytes();
    let tx = Transaction::script(
        gas_price,
        gas_limit,
        maturity,
        script,
        script_data,
        vec![input.clone()],
        vec![output],
        vec![],
    )
    .into_checked(height, &params, client.gas_costs())
    .expect("failed to generate checked tx");

    client.transact(tx);
    balance -= burn;

    let storage_balance = client.transact(tx_check_balance.clone())[0]
        .ra()
        .expect("Balance expected");
    assert_eq!(balance as Word, storage_balance);

    // Burn the remainder balance
    let script: Vec<u8> = script_ops.into_iter().collect();
    let script_data = Call::new(contract, 1, balance).to_bytes();
    let tx = Transaction::script(
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
    .expect("failed to generate checked tx");

    client.transact(tx);

    let storage_balance = client.transact(tx_check_balance)[0].ra().expect("Balance expected");
    assert_eq!(0, storage_balance);
}


#[rstest]
fn spec_can_write_allowed_flag_combinations(#[values(0b00, 0b01, 0b10, 0b11)] flags: Immediate18) {
    let mut script = common_setup();
    script.push(op::movi(0x20, flags));
    script.push(op::flag(0x20));
    script.push(op::ret(RegId::ONE));

    let receipts = run_script(script.into_iter().collect());
    assert_success(&receipts);
}

#[rstest]
fn spec_cannot_write_reserved_flags(#[values(0b100, 0b111)] flags: Immediate18) {
    let mut script = common_setup();
    script.push(op::movi(0x20, flags));
    script.push(op::flag(0x20));
    script.push(op::ret(RegId::ONE));

    let receipts = run_script(script.into_iter().collect());
    assert_panics(&receipts, PanicReason::ErrorFlag);
}
