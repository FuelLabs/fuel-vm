use fuel_asm::{
    op,
    RegId,
};
use fuel_tx::{
    ConsensusParameters,
    Finalizable,
    Script,
    TransactionBuilder,
};
use fuel_types::canonical::Deserialize;

use crate::{
    consts::{
        VM_REGISTER_COUNT,
        WORD_SIZE,
    },
    interpreter::{
        trace::ExecutionTraceHooks,
        InterpreterParams,
        NotSupportedEcal,
    },
    prelude::*,
    tests::test_helpers::assert_success,
};

#[derive(Debug, Clone)]
pub struct Record {
    pub registers: [Word; VM_REGISTER_COUNT],
    pub call_frame: Option<CallFrame>,
    /// Call frame params (a, b) interpreted as (ptr, len) slice, if available.
    pub call_frame_params_slice: Option<Vec<u8>>,
    pub receipt_count: usize,
}
impl Record {
    fn capture<M, S, Tx, Ecal, Trace>(vm: &Interpreter<M, S, Tx, Ecal, Trace>) -> Self
    where
        M: Memory,
    {
        let mut registers = [0; VM_REGISTER_COUNT];
        registers.copy_from_slice(vm.registers());
        let receipt_count = vm.receipts().len();

        let (call_frame, call_frame_params_slice) = if vm.context().is_internal() {
            let size = CallFrame::serialized_size();
            let call_frame_data = vm
                .memory()
                .read(vm.registers()[RegId::FP], size)
                .expect("Invalid fp value");
            let frame =
                CallFrame::from_bytes(call_frame_data).expect("Invalid call frame");
            let params_slice = vm
                .memory()
                .read(frame.a(), frame.b() as usize)
                .ok()
                .map(|slice| slice.to_vec());
            (Some(frame), params_slice)
        } else {
            (None, None)
        };

        Record {
            registers,
            call_frame,
            call_frame_params_slice,
            receipt_count,
        }
    }
}

/// Trace that's captured every time an new receipt is produced.
#[derive(Debug, Clone, Default)]
pub struct TestTrace {
    receipts_before: usize,
    frames: Vec<Record>,
}

impl ExecutionTraceHooks for TestTrace {
    fn before_instruction<M, S, Tx, Ecal, Trace>(
        vm: &mut Interpreter<M, S, Tx, Ecal, Self>,
    ) where
        M: Memory,
    {
        vm.trace_mut().receipts_before = vm.receipts().len();
    }

    fn after_instruction<M, S, Tx, Ecal, Trace>(
        vm: &mut Interpreter<M, S, Tx, Ecal, Self>,
    ) where
        M: Memory,
    {
        if vm.receipts().len() > vm.trace().receipts_before {
            let record = Record::capture(vm);
            vm.trace_mut().frames.push(record);
        }
    }
}

#[test]
fn can_trace_simple_loop() {
    let test_loop_rounds: usize = 5;

    let script_data: Vec<u8> = file!().bytes().collect();
    let script = vec![
        op::movi(0x20, test_loop_rounds as _),
        op::log(0x20, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::subi(0x20, 0x20, 1),
        op::jnzb(0x20, RegId::ZERO, 1),
        op::ret(RegId::ONE),
    ]
    .into_iter()
    .collect();

    let consensus_params = ConsensusParameters::standard();
    let tx = TransactionBuilder::script(script, script_data)
        .script_gas_limit(1_000_000)
        .maturity(Default::default())
        .add_fee_input()
        .finalize()
        .into_checked(Default::default(), &consensus_params)
        .expect("failed to generate a checked tx")
        .test_into_ready();

    let mut vm: Interpreter<_, _, Script, NotSupportedEcal, TestTrace> =
        Interpreter::with_memory_storage();
    let result = vm.transact(tx).expect("Failed to transact");
    let receipts = result.receipts().to_vec();

    assert_success(&receipts);
    let trace = vm.trace();
    assert_eq!(trace.frames.len(), test_loop_rounds + 1); // + 1 for return
    for i in 0..test_loop_rounds {
        let frame = &trace.frames[i];
        assert_eq!(frame.receipt_count, i + 1);
        assert_eq!(frame.registers[0x20], (test_loop_rounds - i) as Word);
    }
}

#[test]
fn can_trace_call_input_struct() {
    let mut test_context = TestBuilder::new(2322u64);
    let gas_limit = 1_000_000;

    // For this contract, param1 is pointer and param2 is length.
    // Contract the logs this input data.
    let contract_code = vec![
        // Fetch paramters from the call frame
        op::addi(0x10, RegId::FP, CallFrame::a_offset() as _),
        op::lw(0x10, 0x10, 0),
        op::addi(0x11, RegId::FP, CallFrame::b_offset() as _),
        op::lw(0x11, 0x11, 0),
        // Log the input data
        op::logd(RegId::ZERO, RegId::ZERO, 0x10, 0x11),
        // Return
        op::ret(RegId::ZERO),
    ];

    let contract_id = test_context
        .setup_contract(contract_code, None, None)
        .contract_id;

    // This script calls the input contract with params pointing to
    // the the bytes of the first four instructions of this script.
    // This is just used as some arbitrary data for testing.
    let instructions_to_point = 4;
    let script = vec![
        op::movi(0x10, (ContractId::LEN + WORD_SIZE * 2) as _),
        op::aloc(0x10),
        op::gtf_args(0x10, RegId::ZERO, GTFArgs::ScriptData),
        op::mcpi(RegId::HP, 0x10, 32),
        // a/param1: pointer to the input data
        op::addi(0x11, RegId::HP, ContractId::LEN as _),
        op::sw(0x11, RegId::IS, 0),
        op::addi(0x11, 0x11, WORD_SIZE as _),
        // b/param2: size of the input data
        op::movi(0x12, (Instruction::SIZE * instructions_to_point) as _),
        op::sw(0x11, 0x12, 0),
        op::call(RegId::HP, RegId::ZERO, 0x10, 0x10),
        op::ret(RegId::ONE),
    ];
    let pointed_data: Vec<u8> = script.clone().into_iter().take(instructions_to_point).collect();

    let tx = test_context
        .start_script(script, contract_id.to_vec())
        .script_gas_limit(gas_limit)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .build()
        .test_into_ready();

    let mut vm: Interpreter<_, _, _, NotSupportedEcal, TestTrace> =
        Interpreter::with_storage(
            MemoryInstance::new(),
            test_context.storage.clone(),
            InterpreterParams::default(),
        );

    let result = vm.transact(tx).expect("Failed to transact");
    let receipts = result.receipts();
    dbg!(&receipts);
    assert_success(&receipts);

    let call_frame = vm.trace().frames[0]
        .call_frame
        .as_ref()
        .expect("Missing call frame");
    assert_eq!(*call_frame.to(), contract_id);
    assert_eq!(
        vm.trace().frames[0].call_frame_params_slice,
        Some(pointed_data)
    );
}
