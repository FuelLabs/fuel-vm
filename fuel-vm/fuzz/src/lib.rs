use fuel_vm::fuel_asm::op;
use fuel_vm::fuel_asm::{Instruction, InvalidOpcode};
use fuel_vm::fuel_types::Word;
use fuel_vm::prelude::field::Script;
use fuel_vm::prelude::*;

use fuel_vm::util::test_helpers::TestBuilder;
use fuel_vm::{fuel_asm, script_with_data_offset};
use fuel_vm::fuel_types::canonical::Serialize;
use std::ops::Range;

/// Magic value used as separator between fuzz data components in corpus files.
const MAGIC_VALUE_SEPARATOR: [u8; 8] = [0x00u8, 0xAD, 0xBE, 0xEF, 0x55, 0x66, 0xCE, 0xAA];

#[derive(Debug, Eq, PartialEq)]
pub struct FuzzData {
    pub program: Vec<u8>,
    pub sub_program: Vec<u8>,
    pub script_data: Vec<u8>,
}

pub fn encode(data: &FuzzData) -> Vec<u8> {
    let separator: Vec<u8> = MAGIC_VALUE_SEPARATOR.into();
    data.program.iter()
        .copied()
        .chain(separator.iter().copied())
        .chain(data.script_data.iter().copied())
        .chain(separator.iter().copied())
        .chain(data.sub_program.iter().copied())
        .collect()
}

fn split_by_separator(data: &[u8], separator: &[u8]) -> Vec<Range<usize>> {
    let separator_len = separator.len();

    let mut last: usize = 0;

    let mut result: Vec<_> = data
        .windows(separator_len)
        .enumerate()
        .filter_map(|(i, window)| {
            if window == separator {
                let option = Some(last..i);
                last = i + separator_len;
                return option;
            } else {
                None
            }
        })
        .collect();

    result.push(last..data.len());

    result
}

pub fn decode(data: &[u8]) -> Option<FuzzData> {
    let x = split_by_separator(data, &MAGIC_VALUE_SEPARATOR);
    if x.len() != 3 {
        return None;
    }
    Some(FuzzData {
        program: data[x[0].clone()].to_vec(),
        script_data: data[x[1].clone()].to_vec(),
        sub_program: data[x[2].clone()].to_vec(),
    })
}
pub fn decode_instructions(bytes: &[u8]) -> Option<Vec<Instruction>> {
    let instructions: Vec<_> = fuel_vm::fuel_asm::from_bytes(bytes.iter().cloned())
        .flat_map(|i: Result<Instruction, InvalidOpcode>| i.ok())
        .collect();
    return Some(instructions);
}

pub struct ExecuteResult {
    pub success: bool,
    pub gas_used: u64,
}

pub fn execute(data: FuzzData) -> ExecuteResult {
    let gas_limit = 1_000_000;
    let asset_id: AssetId = AssetId::new([
        0xFA, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1,
    ]);

    let mut test_context = TestBuilder::new(2322u64);
    let subcontract: Vec<_> = [
        // Pass some data to fuzzer
        op::addi(
            0x10,
            fuel_asm::RegId::FP,
            CallFrame::b_offset() as Immediate12,
        ),
        fuel_vm::fuel_asm::op::lw(0x10, 0x10, 0), // load address word
    ]
    .iter()
    .copied()
    .flat_map(Instruction::to_bytes)
    .chain(data.sub_program.iter().copied())
    .collect::<Vec<u8>>();

    let contract_id = test_context
        .setup_contract_bytes(subcontract.clone(), None, None)
        .contract_id;

    let max_program_length = 2usize.pow(18)
        - test_context.get_tx_params().tx_offset()
        - <fuel_vm::prelude::Script as Script>::script_offset_static();

    let actual_program = &data.program[..max_program_length.min(data.program.len())];

    let (script_ops, script_data_offset): (Vec<u8>, Immediate18) = script_with_data_offset!(
        script_data_offset,
        actual_program.to_vec(),
        test_context.get_tx_params().tx_offset()
    );

    let call = Call::new(contract_id, 0, script_data_offset as Word).to_bytes();
    let script_data: [&[u8]; 2] = [asset_id.as_ref(), call.as_slice()];

    // Provide an asset id and the contract_id
    let script_data: Vec<u8> = script_data.iter().copied().flatten().copied().collect();

    let script_data = script_data
        .iter()
        .chain(data.script_data.iter())
        .copied()
        .collect::<Vec<_>>();

    let transfer_tx = test_context
        .start_script_bytes(script_ops.iter().copied().collect(), script_data)
        .script_gas_limit(gas_limit)
        .gas_price(0)
        .coin_input(asset_id, 1000)
        .contract_input(contract_id)
        .contract_output(&contract_id)
        .change_output(asset_id)
        .execute();

    let gas_used: u64 = *transfer_tx
        .receipts()
        .iter()
        .filter_map(|recipt| match recipt {
            Receipt::ScriptResult { gas_used, .. } => Some(gas_used),
            _ => None,
        })
        .next()
        .unwrap();

    ExecuteResult {
        success: !transfer_tx.should_revert(),
        gas_used,
    }
}
