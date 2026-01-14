use core::{
    array,
    panic,
};

use crate::{
    consts::VM_MAX_RAM,
    prelude::*,
    script_with_data_offset,
    tests::test_helpers::{
        assert_panics,
        assert_success,
        set_full_word,
    },
    util::test_helpers::TestBuilder,
};
use alloc::{
    vec,
    vec::Vec,
};
use fuel_asm::{
    Imm06,
    Imm12,
    RegId,
    op,
};
use fuel_tx::ConsensusParameters;
use fuel_types::{
    bytes::WORD_SIZE,
    canonical::Serialize,
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
fn srwq_can_read_slots_when_created_with_dynamic_opcodes() {
    const DISCARD: RegId = RegId::new(0x39);
    const SLOT_KEY: RegId = RegId::new(0x38);
    const BUFFER: RegId = RegId::new(0x37);

    let receipts = call_contract_once(vec![
        // Allocate buffers
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(SLOT_KEY, RegId::HP),
        op::movi(0x15, 64),
        op::aloc(0x15),
        op::move_(BUFFER, RegId::HP),
        // Store dummy zeroes data to both slots
        op::swri(SLOT_KEY, BUFFER, 32),
        op::sb(SLOT_KEY, RegId::ONE, 31),
        op::swri(SLOT_KEY, BUFFER, 32),
        op::sb(SLOT_KEY, RegId::ZERO, 31),
        // Attempt read using SRWQ, should panic
        op::movi(0x10, 2),
        op::srwq(BUFFER, DISCARD, SLOT_KEY, 0x10),
        op::logd(RegId::ZERO, RegId::ZERO, BUFFER, 0x15),
        // Done
        op::ret(RegId::ONE),
    ]);

    assert_success(&receipts);

    for r in receipts {
        let Receipt::LogData { data, .. } = r else {
            continue;
        };
        let data = data.as_ref().unwrap();
        let expected = [0u8; 64];
        assert_eq!(&**data, &expected);
        return;
    }

    panic!("Missing LogData receipt");
}

#[test]
fn srwq_allows_reading_zero_slots_even_if_the_first_would_have_wrong_size() {
    const DISCARD: RegId = RegId::new(0x39);
    const SLOT_KEY: RegId = RegId::new(0x38);
    const BUFFER: RegId = RegId::new(0x37);

    let receipts = call_contract_once(vec![
        // Allocate buffers
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(SLOT_KEY, RegId::HP),
        op::movi(0x15, 64),
        op::aloc(0x15),
        op::move_(BUFFER, RegId::HP),
        // Store dummy data to the slot
        op::swri(SLOT_KEY, BUFFER, 43),
        // Attempt read using SRWQ, should succeed
        op::srwq(BUFFER, DISCARD, SLOT_KEY, RegId::ZERO),
        // Done
        op::ret(RegId::ONE),
    ]);
    assert_success(&receipts);
}

#[test]
fn srwq_panics_when_combined_slots_sum_to_multiple_of_32() {
    const DISCARD: RegId = RegId::new(0x39);
    const SLOT_KEY: RegId = RegId::new(0x38);
    const BUFFER: RegId = RegId::new(0x37);

    const LEN_FIRST: usize = 20;
    const LEN_SECOND: usize = 64 - LEN_FIRST;

    let receipts = call_contract_once(vec![
        // Allocate buffers
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(SLOT_KEY, RegId::HP),
        op::movi(0x15, 64),
        op::aloc(0x15),
        op::move_(BUFFER, RegId::HP),
        // Store dummy data to both slots
        op::swri(SLOT_KEY, BUFFER, LEN_FIRST as _),
        op::sb(SLOT_KEY, RegId::ONE, 31),
        op::swri(SLOT_KEY, BUFFER, LEN_SECOND as _),
        op::sb(SLOT_KEY, RegId::ZERO, 31),
        // Attempt read using SRWQ, should panic
        op::movi(0x10, 2),
        op::srwq(BUFFER, DISCARD, SLOT_KEY, 0x10),
        // Done
        op::ret(RegId::ONE),
    ]);
    assert_panics(&receipts, PanicReason::StorageOutOfBounds);
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

enum MemoryOverflowCase {
    /// Offset is exactly one byte past the end of memory.
    OffByOne,
    /// Offset is completely outside of memory.
    Outside,
    /// Offset calculation overflows.
    Overflow,
}

enum StorageOverflowCase {
    /// length alone (with offset zero) exceeds slot limit
    Length,
    /// offset alone (with length zero) exceeds slot limit
    Offset,
    /// offset + length exceeds slot limit
    OffsetPlusLength,
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
    #[values(
        MemoryOverflowCase::OffByOne,
        MemoryOverflowCase::Outside,
        MemoryOverflowCase::Overflow
    )]
    case: MemoryOverflowCase,
) {
    let mut program = vec![
        // Allocate slot key (all zeroes)
        op::movi(0x15, 32),
        op::aloc(0x15),
    ];
    // Set up the case
    program.push(match case {
        MemoryOverflowCase::OffByOne => op::addi(0x10, RegId::HP, 1),
        MemoryOverflowCase::Outside => op::addi(0x10, RegId::HP, 32),
        MemoryOverflowCase::Overflow => op::not(0x10, RegId::ZERO),
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
    for i in 0..256u32 {
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

/// Allocates and initalizes a 256 byte array with elements from 0 to 255.
fn create_example_buffer() -> Vec<Instruction> {
    let mut ops = vec![op::movi(0x39, 256), op::aloc(0x39)];
    for i in 0..256u32 {
        ops.push(op::movi(0x39, i as u8 as _));
        ops.push(op::sb(RegId::HP, 0x39, i as _));
    }
    ops
}

#[rstest::rstest]
fn srdd_srdi_reads_slot_contents(
    #[values(0, 1, 2, 63, 100)] offset: u8,
    #[values(0, 1, 2, 63, 100)] len: u8,
    #[values(true, false)] imm: bool, // use immediate instruction variant
) {
    const SLOT_KEY: RegId = RegId::new(0x38);
    const BUFFER: RegId = RegId::new(0x37);

    let mut program = vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(SLOT_KEY, RegId::HP),
    ];
    program.extend(create_example_buffer());
    program.extend([
        op::move_(BUFFER, RegId::HP),
        op::swri(SLOT_KEY, BUFFER, 256),
        op::mcli(RegId::HP, 256),
        op::movi(0x10, offset as _),
        op::movi(0x11, len as _),
    ]);

    // Invoke the storage read instruction
    program.push(if imm {
        if len > Imm06::MAX.to_u8() {
            return; // skip inconstructible test case
        }
        op::srdi(BUFFER, SLOT_KEY, 0x10, len)
    } else {
        op::srdd(BUFFER, SLOT_KEY, 0x10, 0x11)
    });

    // Log results
    program.extend([
        op::move_(0x11, RegId::ERR),
        op::movi(0x10, 256),
        op::logd(0x11, RegId::ZERO, BUFFER, 0x10),
        op::ret(RegId::ONE),
    ]);
    let receipts = call_contract_once(program);
    assert_success(&receipts);

    let example_data: [u8; 256] = array::from_fn(|i| i as u8);
    let mut expected = [0u8; 256];
    expected[..(len as usize)].copy_from_slice(
        &example_data[(offset as usize)..(offset as usize + len as usize)],
    );

    for r in receipts {
        let Receipt::LogData { ra, data, .. } = r else {
            continue;
        };
        assert_eq!(ra, 0, "$err should be cleared when read is successful");
        let data = data.as_ref().unwrap();
        assert_eq!(**data, expected);
        return;
    }

    panic!("Missing LogData receipt");
}

#[rstest::rstest]
fn srdd_srdi_reading_nonexistent_slot_sets_err(
    #[values(true, false)] imm: bool, // use immediate instruction variant
) {
    const SLOT_KEY: RegId = RegId::new(0x38);

    // Allocate slot key (all zeroes)
    let mut program = vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(SLOT_KEY, RegId::HP),
    ];

    // Invoke the storage read instruction
    program.push(if imm {
        op::srdi(RegId::HP, SLOT_KEY, RegId::ZERO, 0)
    } else {
        op::srdd(RegId::HP, SLOT_KEY, RegId::ZERO, RegId::ZERO)
    });

    // Log results
    program.extend([
        op::log(RegId::ERR, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ]);
    let receipts = call_contract_once(program);
    assert_success(&receipts);

    for r in receipts {
        let Receipt::Log { ra, .. } = r else {
            continue;
        };
        assert_eq!(ra, 1, "$err should set when reading nonexistent slot");
        return;
    }

    panic!("Missing Log receipt");
}

#[rstest::rstest]
fn srdd_srdi_read_past_the_end_panics(
    #[values(
        StorageOverflowCase::Length,
        StorageOverflowCase::Offset,
        StorageOverflowCase::OffsetPlusLength
    )]
    case: StorageOverflowCase,
    #[values(true, false)] imm: bool, // use immediate instruction variant
) {
    let (len, offset): (u16, u16) = match case {
        StorageOverflowCase::Length => (257, 0),
        StorageOverflowCase::Offset => (0, 257),
        StorageOverflowCase::OffsetPlusLength => (128, 129),
    };

    const SLOT_KEY: RegId = RegId::new(0x38);
    const BUFFER: RegId = RegId::new(0x37);

    let mut program = vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(SLOT_KEY, RegId::HP),
    ];
    program.extend(create_example_buffer());
    program.extend([
        op::move_(BUFFER, RegId::HP),
        op::swri(SLOT_KEY, BUFFER, 256),
        op::mcli(RegId::HP, 256),
        op::movi(0x10, offset as _),
        op::movi(0x11, len as _),
    ]);

    // Invoke the storage read instruction
    program.push(if imm {
        if len > Imm06::MAX.to_u8() as _ {
            return; // skip inconstructible test case
        }
        op::srdi(BUFFER, SLOT_KEY, 0x10, len as _)
    } else {
        op::srdd(BUFFER, SLOT_KEY, 0x10, 0x11)
    });

    // Log results
    program.extend([
        op::movi(0x10, 256),
        op::logd(RegId::ZERO, RegId::ZERO, BUFFER, 0x10),
        op::ret(RegId::ONE),
    ]);
    let receipts = call_contract_once(program);
    assert_panics(&receipts, PanicReason::StorageOutOfBounds);
}

#[rstest::rstest]
fn srdd_srdi_dst_buffer_outside_memory_panics(
    #[values(
        MemoryOverflowCase::OffByOne,
        MemoryOverflowCase::Outside,
        MemoryOverflowCase::Overflow
    )]
    case: MemoryOverflowCase,
    #[values(true, false)] imm: bool, // use immediate instruction variant
) {
    const SLOT_KEY: RegId = RegId::new(0x38);
    const BUFFER: RegId = RegId::new(0x37);

    let mut program = vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(SLOT_KEY, RegId::HP),
    ];
    program.extend(create_example_buffer());
    program.extend([
        op::move_(BUFFER, RegId::HP),
        op::swri(SLOT_KEY, BUFFER, 256),
        op::mcli(RegId::HP, 256),
    ]);
    program.extend(set_full_word(0x11, VM_MAX_RAM));
    program.push(match case {
        MemoryOverflowCase::OffByOne => op::addi(0x10, 0x11, 0),
        MemoryOverflowCase::Outside => op::addi(0x10, 0x11, 1),
        MemoryOverflowCase::Overflow => op::not(0x10, RegId::ZERO),
    });

    program.push(if imm {
        op::srdi(0x10, SLOT_KEY, RegId::ZERO, 1)
    } else {
        op::srdd(0x10, SLOT_KEY, RegId::ZERO, RegId::ONE)
    });

    // Unreachable
    program.push(op::ret(RegId::ONE));

    // Check
    let receipts = call_contract_once(program);
    assert_panics(&receipts, PanicReason::MemoryOverflow);
}

#[rstest::rstest]
fn swrd_swri_writes_storage_slot(
    #[values(0, 1, 2, 63, 100)] len: usize,
    #[values(true, false)] imm: bool, // use immediate instruction variant
) {
    const SLOT_KEY: RegId = RegId::new(0x38);

    let mut program = vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(SLOT_KEY, RegId::HP),
        op::movi(0x10, len as _),
        op::aloc(0x10),
    ];

    program.push(if imm {
        if len > Imm12::MAX.to_u16() as _ {
            return; // skip inconstructible test case
        }
        op::swri(SLOT_KEY, 0x10, len as _)
    } else {
        op::swrd(SLOT_KEY, 0x10, 0x10)
    });

    // Read slot len and log results
    program.extend([
        op::spld(0x10, SLOT_KEY),
        op::log(0x10, RegId::ERR, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ]);

    // Check
    let receipts = call_contract_once(program);

    for r in receipts {
        let Receipt::Log { ra, rb, .. } = r else {
            continue;
        };
        assert_eq!(ra, len as u64, "Logged length should match written length");
        assert_eq!(rb, 0, "$err should be clear since slot exists");
        return;
    }

    panic!("Missing Log receipt");
}

#[rstest::rstest]
fn swrd_swri_src_buffer_outside_memory_panics(
    #[values(
        MemoryOverflowCase::OffByOne,
        MemoryOverflowCase::Outside,
        MemoryOverflowCase::Overflow
    )]
    case: MemoryOverflowCase,
    #[values(true, false)] imm: bool, // use immediate instruction variant
) {
    const SLOT_KEY: RegId = RegId::new(0x38);

    let mut program = vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(SLOT_KEY, RegId::HP),
    ];
    program.extend(set_full_word(0x11, VM_MAX_RAM));
    program.push(match case {
        MemoryOverflowCase::OffByOne => op::addi(0x10, 0x11, 0),
        MemoryOverflowCase::Outside => op::addi(0x10, 0x11, 1),
        MemoryOverflowCase::Overflow => op::not(0x10, RegId::ZERO),
    });

    program.push(if imm {
        op::swri(SLOT_KEY, 0x10, 1)
    } else {
        op::swrd(SLOT_KEY, 0x10, RegId::ONE)
    });

    // Unreachable
    program.push(op::ret(RegId::ONE));

    // Check
    let receipts = call_contract_once(program);
    assert_panics(&receipts, PanicReason::MemoryOverflow);
}

/// Note: swri cannot exceed max limit due to immediate size constraint,
/// unless the limit is unreasonably small, so we wont bother testing it here
#[rstest::rstest]
fn swrd_exceeding_slot_max_length_panics() {
    const SLOT_KEY: RegId = RegId::new(0x38);

    let limit = ConsensusParameters::default()
        .script_params()
        .max_storage_slot_length();

    let mut program = vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(SLOT_KEY, RegId::HP),
    ];
    program.extend(set_full_word(0x11, limit + 1));
    program.extend([
        op::aloc(0x11),
        op::swrd(SLOT_KEY, RegId::HP, 0x11),
        op::ret(RegId::ONE),
    ]);

    // Check
    let receipts = call_contract_once(program);
    assert_panics(&receipts, PanicReason::StorageOutOfBounds);
}

#[rstest::rstest]
fn supd_supi_exceeding_slot_max_length_panics(
    #[values(true, false)] imm: bool, // use immediate instruction variant
) {
    const SLOT_KEY: RegId = RegId::new(0x38);

    let limit = ConsensusParameters::default()
        .script_params()
        .max_storage_slot_length();

    let mut program = vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(SLOT_KEY, RegId::HP),
    ];
    program.extend(set_full_word(0x11, limit));
    program.extend([
        op::aloc(0x11),
        op::swrd(SLOT_KEY, RegId::HP, 0x11),
        // Log here to prove that setup succeeded
        op::log(RegId::ONE, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::not(0x10, RegId::ZERO),
    ]);

    // Write beyond limit by extending by 1 byte
    program.push(if imm {
        op::supi(SLOT_KEY, RegId::HP, 0x10, 1)
    } else {
        op::supd(SLOT_KEY, RegId::HP, 0x10, RegId::ONE)
    });

    // Unreachable
    program.push(op::ret(RegId::ONE));

    // Check
    let receipts = call_contract_once(program);
    assert!(
        receipts
            .iter()
            .any(|r| matches!(r, Receipt::Log { ra, .. } if *ra == 1)),
        "Setup failed"
    );
    assert_panics(&receipts, PanicReason::StorageOutOfBounds);
}

#[rstest::rstest]
fn supd_supi_treat_nonexisting_slot_as_empty(
    #[values(true, false)] imm: bool, // use immediate instruction variant
) {
    const SLOT_KEY: RegId = RegId::new(0x38);
    const BUFFER: RegId = RegId::new(0x37);

    let mut program = vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(SLOT_KEY, RegId::HP),
        op::move_(BUFFER, RegId::HP),
    ];

    program.push(if imm {
        op::supi(SLOT_KEY, BUFFER, RegId::ZERO, 32)
    } else {
        op::supd(SLOT_KEY, BUFFER, RegId::ZERO, 0x15)
    });

    // Log results
    program.extend([
        op::spld(0x10, SLOT_KEY),
        op::aloc(0x10),
        op::srdd(RegId::HP, SLOT_KEY, RegId::ZERO, 0x10),
        op::logd(RegId::ERR, RegId::ZERO, RegId::HP, 0x10),
        op::ret(RegId::ONE),
    ]);

    // Check
    let receipts = call_contract_once(program);
    assert_success(&receipts);

    for r in receipts {
        let Receipt::LogData { ra, data, .. } = r else {
            continue;
        };
        assert_eq!(ra, 0, "$err should be clear when read is successful");
        let data = data.as_ref().unwrap();
        let expected = [0u8; 32];
        assert_eq!(&**data, &expected);
        return;
    }

    panic!("Missing LogData receipt");
}

#[rstest::rstest]
fn supd_supi_write_subrange_works(
    #[values(true, false)] imm: bool, // use immediate instruction variant
) {
    const SLOT_KEY: RegId = RegId::new(0x38);
    const BUFFER: RegId = RegId::new(0x37);

    const W_LEN: u8 = 8;
    const W_OFFSET: u8 = 4;

    let mut program = vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(SLOT_KEY, RegId::HP),
        // Prepare slot with some initial data (zeroes)
        op::swri(SLOT_KEY, SLOT_KEY, 32),
        // Prepare data to be written (8 bytes of all bits set)
        op::movi(0x15, 8),
        op::aloc(0x15),
        op::move_(BUFFER, RegId::HP),
        op::not(0x10, RegId::ZERO),
        op::sw(BUFFER, 0x10, 0),
        // Prepare subrange write
        op::movi(0x10, W_OFFSET as _),
        op::movi(0x11, W_LEN as _),
    ];

    program.push(if imm {
        op::supi(SLOT_KEY, BUFFER, 0x10, W_LEN as _)
    } else {
        op::supd(SLOT_KEY, BUFFER, 0x10, 0x11)
    });

    // Log results
    program.extend([
        op::spld(0x10, SLOT_KEY),
        op::aloc(0x10),
        op::srdd(RegId::HP, SLOT_KEY, RegId::ZERO, 0x10),
        op::logd(RegId::ERR, RegId::ZERO, RegId::HP, 0x10),
        op::ret(RegId::ONE),
    ]);

    // Check
    let receipts = call_contract_once(program);
    assert_success(&receipts);

    for r in receipts {
        let Receipt::LogData { ra, data, .. } = r else {
            continue;
        };
        assert_eq!(ra, 0, "$err should be clear when read is successful");
        let data = data.as_ref().unwrap();
        let mut expected = [0u8; 32];
        expected[W_OFFSET as usize..(W_OFFSET as usize + W_LEN as usize)].fill(0xff);
        assert_eq!(&**data, &expected);
        return;
    }

    panic!("Missing LogData receipt");
}

#[rstest::rstest]
fn supd_supi_writing_past_end_extends_value(
    #[values(true, false)] imm: bool, // use immediate instruction variant
) {
    const SLOT_KEY: RegId = RegId::new(0x38);
    const BUFFER: RegId = RegId::new(0x37);

    const W_LEN: u8 = 8;
    const W_OFFSET: u8 = 4;

    let mut program = vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(SLOT_KEY, RegId::HP),
        op::move_(BUFFER, RegId::HP),
        // Prepare slot with some initial data
        op::swri(SLOT_KEY, BUFFER, 8),
        op::movi(0x10, W_OFFSET as _),
        op::movi(0x11, W_LEN as _),
    ];

    program.push(if imm {
        op::supi(SLOT_KEY, BUFFER, 0x10, W_LEN as _)
    } else {
        op::supd(SLOT_KEY, BUFFER, 0x10, 0x11)
    });

    // Log results
    program.extend([
        op::spld(0x10, SLOT_KEY),
        op::log(0x10, RegId::ERR, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ]);

    // Check
    let receipts = call_contract_once(program);
    assert_success(&receipts);

    for r in receipts {
        let Receipt::Log { ra, rb, .. } = r else {
            continue;
        };
        assert_eq!(ra, (W_OFFSET + W_LEN) as u64, "Final length incorrect");
        assert_eq!(rb, 0, "$err should be clear when read is successful");
        return;
    }

    panic!("Missing Log receipt");
}

#[rstest::rstest]
fn supd_supi_extending_using_u64_MAX_works(
    #[values(true, false)] imm: bool, // use immediate instruction variant
) {
    const SLOT_KEY: RegId = RegId::new(0x38);
    const BUFFER: RegId = RegId::new(0x37);
    const REPETITIONS: usize = 8; // 8 * 8 = 64 bytes

    let mut program = vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(SLOT_KEY, RegId::HP),
        op::movi(0x15, 8),
        op::aloc(0x15),
        op::move_(BUFFER, RegId::HP),
        op::sw(BUFFER, RegId::ONE, 0),
        op::not(0x10, RegId::ZERO),
        op::movi(0x11, 8),
    ];

    for _ in 0..REPETITIONS {
        program.push(if imm {
            op::supi(SLOT_KEY, BUFFER, 0x10, 8)
        } else {
            op::supd(SLOT_KEY, BUFFER, 0x10, 0x11)
        });
    }
    // Log results
    program.extend([
        op::spld(0x10, SLOT_KEY),
        op::aloc(0x10),
        op::srdd(RegId::HP, SLOT_KEY, RegId::ZERO, 0x10),
        op::logd(RegId::ERR, RegId::ZERO, RegId::HP, 0x10),
        op::ret(RegId::ONE),
    ]);

    // Check
    let receipts = call_contract_once(program);
    assert_success(&receipts);

    for r in receipts {
        let Receipt::LogData { ra, data, .. } = r else {
            continue;
        };
        assert_eq!(ra, 0, "$err should be clear when read is successful");
        let data = data.as_ref().unwrap();
        let expected = 1u64.to_be_bytes().repeat(8);
        assert_eq!(&**data, &expected);
        return;
    }

    panic!("Missing LogData receipt");
}

#[rstest::rstest]
fn supd_supi_offset_past_existing_value_panics(
    #[values(true, false)] imm: bool, // use immediate instruction variant
) {
    const SLOT_KEY: RegId = RegId::new(0x38);
    const BUFFER: RegId = RegId::new(0x37);

    let mut program = vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(SLOT_KEY, RegId::HP),
        op::move_(BUFFER, RegId::HP),
        op::movi(0x11, 33),
    ];

    program.push(if imm {
        op::supi(SLOT_KEY, BUFFER, 0x11, 1)
    } else {
        op::supd(SLOT_KEY, BUFFER, 0x11, RegId::ONE)
    });

    // Unreachable
    program.push(op::ret(RegId::ONE));

    // Check
    let receipts = call_contract_once(program);
    assert_panics(&receipts, PanicReason::StorageOutOfBounds);
}

#[rstest::rstest]
fn supd_supi_src_buffer_outside_memory_panics(
    #[values(
        MemoryOverflowCase::OffByOne,
        MemoryOverflowCase::Outside,
        MemoryOverflowCase::Overflow
    )]
    case: MemoryOverflowCase,
    #[values(true, false)] imm: bool, // use immediate instruction variant
) {
    const SLOT_KEY: RegId = RegId::new(0x38);

    let mut program = vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(SLOT_KEY, RegId::HP),
    ];
    program.extend(set_full_word(0x11, VM_MAX_RAM));
    program.push(match case {
        MemoryOverflowCase::OffByOne => op::addi(0x10, 0x11, 0),
        MemoryOverflowCase::Outside => op::addi(0x10, 0x11, 1),
        MemoryOverflowCase::Overflow => op::not(0x10, RegId::ZERO),
    });

    program.push(if imm {
        op::supi(SLOT_KEY, 0x10, RegId::ZERO, 1)
    } else {
        op::supd(SLOT_KEY, 0x10, RegId::ZERO, RegId::ONE)
    });

    // Unreachable
    program.push(op::ret(RegId::ONE));

    // Check
    let receipts = call_contract_once(program);
    assert_panics(&receipts, PanicReason::MemoryOverflow);
}

#[rstest::rstest]
fn spld_returns_correct_len(#[values(0, 1, 2, 3, 4, 5, 100)] len: u8) {
    const SLOT_KEY: RegId = RegId::new(0x38);
    const BUFFER: RegId = RegId::new(0x37);

    let receipts = call_contract_once(vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(SLOT_KEY, RegId::HP),
        op::movi(0x15, len as _),
        op::aloc(0x15),
        op::move_(BUFFER, RegId::HP),
        op::swri(SLOT_KEY, BUFFER, len as _),
        op::spld(0x10, SLOT_KEY),
        op::log(0x10, RegId::ERR, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ]);
    assert_success(&receipts);

    for r in receipts {
        let Receipt::Log { ra, rb, .. } = r else {
            continue;
        };
        assert_eq!(ra, len as u64, "Logged length should match slot length");
        assert_eq!(rb, 0, "$err should clear when reading existing slot");
        return;
    }

    panic!("Missing Log receipt");
}

#[test]
fn spld_reports_nonexisting_slots_in_err() {
    let receipts = call_contract_once(vec![
        op::movi(0x10, 1234),      // dummy value to make sure it's overwritten
        op::spld(0x10, RegId::FP), // nonexistent slot
        op::log(0x10, RegId::ERR, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ]);
    assert_success(&receipts);

    for r in receipts {
        let Receipt::Log { ra, rb, .. } = r else {
            continue;
        };
        assert_eq!(ra, 0, "Logged length should be zero for nonexistent slot");
        assert_eq!(rb, 1, "$err should set when reading nonexistent slot");
        return;
    }

    panic!("Missing Log receipt");
}

#[rstest::rstest]
fn spcp_copies_whole_value(#[values(0, 1, 2, 63, 100)] len: u8) {
    const SLOT_KEY: RegId = RegId::new(0x38);
    const BUFFER: RegId = RegId::new(0x37);

    let mut program = vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(SLOT_KEY, RegId::HP),
    ];
    program.extend(create_example_buffer());
    program.extend([
        op::move_(BUFFER, RegId::HP),
        op::swri(SLOT_KEY, BUFFER, len as _),
        op::spld(0x11, SLOT_KEY),
        op::mcli(BUFFER, 256),
        op::spcp(BUFFER, RegId::ZERO, 0x11, 0),
        op::movi(0x10, 256),
        op::logd(RegId::ZERO, RegId::ZERO, BUFFER, 0x10),
        op::ret(RegId::ONE),
    ]);

    let receipts = call_contract_once(program);
    assert_success(&receipts);

    let example_data: [u8; 256] = array::from_fn(|i| i as u8);
    let mut expected = [0u8; 256];
    expected[..(len as usize)].copy_from_slice(&example_data[..len as usize]);

    for r in receipts {
        let Receipt::LogData { data, .. } = r else {
            continue;
        };
        let data = data.as_ref().unwrap();
        assert_eq!(**data, expected);
        return;
    }

    panic!("Missing LogData receipt");
}

#[rstest::rstest]
fn spcp_copies_correct_subslice(
    #[values(0, 1, 2, 63, 100)] offset: u8,
    #[values(0, 1, 2, 63, 100)] len: u8,
) {
    const DISCARD: RegId = RegId::new(0x39);
    const SLOT_KEY: RegId = RegId::new(0x38);
    const BUFFER: RegId = RegId::new(0x37);

    let mut program = vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(SLOT_KEY, RegId::HP),
    ];
    program.extend(create_example_buffer());
    program.extend([
        op::move_(BUFFER, RegId::HP),
        op::swri(SLOT_KEY, BUFFER, 256),
        op::spld(DISCARD, SLOT_KEY),
        op::mcli(RegId::HP, 256),
        op::movi(0x10, offset as _),
        op::movi(0x11, len as _),
        op::spcp(BUFFER, 0x10, 0x11, 0),
        op::movi(0x10, 256),
        op::logd(RegId::ZERO, RegId::ZERO, BUFFER, 0x10),
        op::ret(RegId::ONE),
    ]);

    let receipts = call_contract_once(program);
    assert_success(&receipts);

    let example_data: [u8; 256] = array::from_fn(|i| i as u8);
    let mut expected = [0u8; 256];
    expected[..(len as usize)].copy_from_slice(
        &example_data[(offset as usize)..(offset as usize + len as usize)],
    );

    for r in receipts {
        let Receipt::LogData { data, .. } = r else {
            continue;
        };
        let data = data.as_ref().unwrap();
        assert_eq!(**data, expected);
        return;
    }

    panic!("Missing LogData receipt");
}

#[rstest::rstest]
fn spcp_panics_if_preloaded_data_range_is_invalid(
    #[values(
        StorageOverflowCase::Length,
        StorageOverflowCase::Offset,
        StorageOverflowCase::OffsetPlusLength
    )]
    case: StorageOverflowCase,
) {
    const DISCARD: RegId = RegId::new(0x39);
    const SLOT_KEY: RegId = RegId::new(0x38);
    const BUFFER: RegId = RegId::new(0x37);

    let (offset, len): (u16, u16) = match case {
        StorageOverflowCase::Length => (0, 33),
        StorageOverflowCase::Offset => (33, 0),
        StorageOverflowCase::OffsetPlusLength => (16, 17),
    };

    let receipts = call_contract_once(vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(SLOT_KEY, RegId::HP),
        op::movi(0x15, 64),
        op::aloc(0x15),
        op::move_(BUFFER, RegId::HP),
        op::swri(SLOT_KEY, BUFFER, 32),
        op::spld(DISCARD, SLOT_KEY),
        op::movi(0x10, offset as _),
        op::movi(0x11, len as _),
        op::spcp(BUFFER, 0x10, 0x11, 0),
        op::ret(RegId::ONE),
    ]);
    assert_panics(&receipts, PanicReason::StorageOutOfBounds);
}

#[rstest::rstest]
fn spcp_panics_if_dst_range_is_invalid(
    #[values(
        MemoryOverflowCase::OffByOne,
        MemoryOverflowCase::Outside,
        MemoryOverflowCase::Overflow
    )]
    case: MemoryOverflowCase,
) {
    const DISCARD: RegId = RegId::new(0x39);
    const SLOT_KEY: RegId = RegId::new(0x38);
    const BUFFER: RegId = RegId::new(0x37);

    let mut program = set_full_word(0x20, VM_MAX_RAM);
    program.push(match case {
        MemoryOverflowCase::OffByOne => op::addi(0x10, 0x20, 0),
        MemoryOverflowCase::Outside => op::addi(0x10, 0x20, 1),
        MemoryOverflowCase::Overflow => op::not(0x10, RegId::ZERO),
    });
    program.extend([
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(SLOT_KEY, RegId::HP),
        op::move_(BUFFER, RegId::HP),
        op::swri(SLOT_KEY, BUFFER, 32),
        op::spld(DISCARD, SLOT_KEY),
        op::spcp(0x10, RegId::ZERO, RegId::ONE, 0),
        op::ret(RegId::ONE),
    ]);

    let receipts = call_contract_once(program);
    assert_panics(&receipts, PanicReason::MemoryOverflow);
}

#[test]
fn spcp_panics_if_length_field_sum_overflows() {
    const DISCARD: RegId = RegId::new(0x39);
    const SLOT_KEY: RegId = RegId::new(0x38);
    const BUFFER: RegId = RegId::new(0x37);

    let receipts = call_contract_once(vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(SLOT_KEY, RegId::HP),
        op::move_(BUFFER, RegId::HP),
        op::swri(SLOT_KEY, BUFFER, 32),
        op::spld(DISCARD, SLOT_KEY),
        op::not(0x10, RegId::ZERO),
        op::spcp(BUFFER, RegId::ZERO, 0x10, 1),
        op::ret(RegId::ONE),
    ]);
    assert_panics(&receipts, PanicReason::MemoryOverflow);
}
