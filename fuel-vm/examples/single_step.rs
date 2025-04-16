//! This example shows how you can run the VM in single-stepping mode,
//! allowing for e.g. visualization of the execution state at each step.

use fuel_asm::{
    RawInstruction,
    RegId,
    op,
};
use fuel_tx::{
    ConsensusParameters,
    TransactionBuilder,
};
use fuel_vm::{
    interpreter::{
        Interpreter,
        Memory,
        NotSupportedEcal,
    },
    prelude::*,
};

fn get_next_instruction<M, S, Tx>(
    vm: &Interpreter<M, S, Tx, NotSupportedEcal>,
) -> Option<Instruction>
where
    M: Memory,
{
    let pc = vm.registers()[RegId::PC];
    let instruction = RawInstruction::from_be_bytes(vm.memory().read_bytes(pc).ok()?);
    Instruction::try_from(instruction).ok()
}

fn main() {
    let mut vm = Interpreter::<_, _, _, NotSupportedEcal>::with_memory_storage();
    vm.set_single_stepping(true);

    let script_data: Vec<u8> = file!().bytes().collect();
    let script = vec![
        op::movi(0x21, 5),                    // How many times to loop
        op::addi(0x20, 0x20, 1),              // Increment loop counter
        op::jneb(0x20, 0x21, RegId::ZERO, 0), // Jump back to increment
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
        .into_ready(
            0,
            consensus_params.gas_costs(),
            consensus_params.fee_params(),
            None,
        )
        .expect("Failed to finalize tx");

    let mut t = *vm.transact(tx).expect("panicked").state();

    loop {
        match t {
            ProgramState::Return(r) => {
                println!("done: returned {r:?}");
                break;
            }
            ProgramState::ReturnData(r) => {
                println!("done: returned data {r:?}");
                break;
            }
            ProgramState::Revert(r) => {
                println!("done: reverted {r:?}");
                break;
            }
            ProgramState::RunProgram(d) => {
                match d {
                    DebugEval::Breakpoint(bp) => {
                        println!(
                            "at {:>4} reg[0x20] = {:4}, next instruction: {}",
                            bp.pc(),
                            &vm.registers()[0x20],
                            get_next_instruction(&vm)
                                .map(|i| format!("{i:?}"))
                                .unwrap_or_else(|| "???".to_owned()),
                        );
                    }
                    DebugEval::Continue => {}
                }
                t = vm.resume().expect("panicked");
            }
            ProgramState::VerifyPredicate(d) => {
                println!("paused on debugger {d:?} (in predicate)");
                t = vm.resume().expect("panicked");
            }
        }
    }
}
