use core::panic;

use crate::{
    prelude::*,
    script_with_data_offset,
    tests::{
        receipts,
        test_helpers::{
            assert_panics,
            assert_success,
        },
    },
    util::test_helpers::TestBuilder,
};
use alloc::{
    vec,
    vec::Vec,
};
use fuel_asm::{
    Imm06,
    RegId,
    op,
};
use fuel_tx::{
    ConsensusParameters,
    Witness,
    policies::Policies,
};
use fuel_types::{
    bytes::WORD_SIZE,
    canonical::Serialize,
};
use rand::{
    Rng,
    SeedableRng,
    rngs::StdRng,
};

/// Helper to deploy and call a contract once.
fn call_contract_once(program: Vec<Instruction>) -> Vec<Receipt> {
    let mut test_context = TestBuilder::new(2322u64);

    let contract_id = test_context.setup_contract(program, None, None).contract_id;

    let (script_call, _) = script_with_data_offset!(
        data_offset,
        vec![
            op::movi(0x10, data_offset as Immediate18),
            op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS),
            op::ret(RegId::ONE),
        ],
        test_context.get_tx_params().tx_offset()
    );
    let script_call_data = Call::new(contract_id, 0, 0).to_bytes();

    let result = test_context
        .start_script(script_call.clone(), script_call_data)
        .script_gas_limit(1_000_000)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .variable_output(AssetId::zeroed())
        .execute();

    result.receipts().to_vec()
}

#[test]
fn sww_writes_32_bytes() {
    const DISCARD: RegId = RegId::new(0x39);

    let receipts = call_contract_once(vec![
        // Allocate a buffer
        op::movi(0x15, 64),
        op::aloc(0x15),
        // Store some data
        op::movi(0x10, 0x01),
        op::sww(RegId::HP, DISCARD, 0x10),
        op::addi(0x11, RegId::HP, 32),
        op::sb(0x11, RegId::ONE, 31),
        op::movi(0x10, 0x02),
        op::sww(0x11, DISCARD, 0x10),
        // Load it back in 32 byte groups
        op::movi(0x10, 0x02),
        op::srwq(RegId::HP, DISCARD, RegId::HP, 0x10),
        // Log it
        op::logd(RegId::ZERO, RegId::ZERO, RegId::HP, 0x15),
        // Done
        op::ret(RegId::ONE),
    ]);

    assert_success(&receipts);

    for r in receipts {
        let Receipt::LogData { data, .. } = r else {
            continue;
        };
        let data = data.as_ref().unwrap();
        let mut expected = [0u8; 64];
        expected[7] = 1;
        expected[32 + 7] = 2;
        assert_eq!(&**data, &expected);
        return;
    }

    panic!("Missing LogData receipt");
}

#[test]
fn srw_offset_works() {
    const DISCARD: RegId = RegId::new(0x39);

    // Construct a program that writes 1024 bytes to storage,
    // then reads two bytes at different offsets using `srw` and logs them.
    let mut program = vec![
        // Allocate slot key (all zeroes)
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(0x14, RegId::HP),
        // Allocate a buffer for the data
        op::movi(0x15, 1024),
        op::aloc(0x15),
    ];
    // Make up some data and write it to the storage
    for offset in 0..(1024 / WORD_SIZE) {
        program.extend([
            op::movi(0x10, (offset % 256) as u8 as _),
            op::sw(RegId::HP, 0x10, offset as _),
        ]);
    }
    program.push(op::swri(0x14, RegId::HP, 1024));
    // Log test cases
    for offset in 0..=Imm06::MAX.to_u8() {
        program.extend([
            op::srw(0x10, DISCARD, 0x14, offset),
            op::log(0x10, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        ]);
    }
    // Done
    program.push(op::ret(RegId::ONE));

    let receipts = call_contract_once(program);

    assert_success(&receipts);

    let logged_values: Vec<u64> = receipts
        .iter()
        .filter_map(|r| match r {
            Receipt::Log { ra, .. } => Some(*ra),
            _ => None,
        })
        .collect();

    assert_eq!(logged_values.len(), Imm06::MAX.to_u8() as usize + 1);

    for i in 0..=Imm06::MAX.to_u8() {
        assert_eq!(logged_values[i as usize], i as u64);
    }
}

#[rstest::rstest]
fn storage_op_storage_key_read_past_boudnds_panics(
    #[values(
        op::sclr(0x10, RegId::ONE),
        op::srdd(RegId::HP, 0x10, RegId::ZERO, RegId::ZERO),
        op::srdi(RegId::HP, 0x10, RegId::ZERO, 0),
        op::swrd(0x10, RegId::HP, RegId::ZERO),
        op::swri(0x10, RegId::HP, 0),
        op::supd(0x10, RegId::HP, RegId::ZERO, RegId::ZERO),
        op::supi(0x10, RegId::HP, RegId::ZERO, 0),
        op::spld(0x11, 0x10)
    )]
    instr: Instruction,
    #[values("offbyone", "outside", "overflow")] case: &str,
) {
    let mut program = vec![
        // Allocate slot key (all zeroes)
        op::movi(0x15, 32),
        op::aloc(0x15),
    ];
    // Set up the case
    program.push(match case {
        "offbyone" => op::addi(0x10, RegId::HP, 1),
        "outside" => op::addi(0x10, RegId::HP, 32),
        "overflow" => op::not(0x10, RegId::ZERO),
        _ => unreachable!(),
    });
    // Perform the storage operation
    program.push(instr);
    // The instruction above should panic, so we never reach this
    program.push(op::ret(RegId::ONE));

    let receipts = call_contract_once(program);
    assert_panics(&receipts, PanicReason::MemoryOverflow);
}

#[rstest::rstest]
fn sclr_clears_correct_number_of_slots(
    #[values(0, 1, 2, 3, 100)] num_create: u8,
    #[values(0, 1, 2, 3, 100)] num_clear: u8,
) {
    const DISCARD: RegId = RegId::new(0x39);

    let mut program = vec![
        // Allocate slot key buffer (all zeroes)
        op::movi(0x15, 32),
        op::aloc(0x15),
    ];
    // Create slots
    for i in 0..num_create {
        program.extend([
            op::movi(0x10, i as _),
            op::sb(RegId::HP, 0x10, 31),
            op::sww(RegId::HP, DISCARD, RegId::ONE),
        ]);
    }
    // Clear slots
    program.extend([
        op::mcli(RegId::HP, 32),
        op::movi(0x10, num_clear as _),
        op::sclr(RegId::HP, 0x10),
    ]);
    // Log the first 256 slots
    for i in 0..256 {
        program.extend([
            op::movi(0x10, i as _),
            op::sb(RegId::HP, 0x10, 31),
            op::spld(0x10, RegId::HP),
            op::log(0x10, RegId::ERR, RegId::ZERO, RegId::ZERO),
        ]);
    }
    // Done
    program.push(op::ret(RegId::ONE));

    let receipts = call_contract_once(program);
    assert_success(&receipts);

    let slots_after: Vec<bool> = receipts
        .iter()
        .filter_map(|r| match r {
            Receipt::Log { ra, rb, .. } => Some(match rb {
                0 => {
                    // $err clear, so this is an occupied slot
                    assert_eq!(*ra, 32, "All created slots should have length of 32");
                    true
                }
                1 => {
                    // $err set, so this is a cleared slot
                    assert_eq!(*ra, 0, "Cleared slots should have length of 0");
                    false
                }
                _ => unreachable!("Unexpected $err value in Log receipt"),
            }),
            _ => None,
        })
        .collect();

    let mut expected = vec![false; 256];
    expected[..(num_create as usize)].fill(true);
    expected[..(num_clear as usize)].fill(false);

    assert_eq!(slots_after, expected);
}
