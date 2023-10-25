#[cfg(test)]
use alloc::{
    vec,
    vec::Vec,
};

use super::Interpreter;
use crate::prelude::*;
use fuel_asm::RegId;

impl<S, Tx, Ecal> Interpreter<S, Tx, Ecal>
where
    Tx: ExecutableTransaction,
{
    /// Get single-stepping mode
    pub const fn single_stepping(&self) -> bool {
        self.debugger.single_stepping()
    }

    /// Set single-stepping mode
    pub fn set_single_stepping(&mut self, single_stepping: bool) {
        self.debugger.set_single_stepping(single_stepping)
    }

    /// Set a new breakpoint for the provided location.
    pub fn set_breakpoint(&mut self, breakpoint: Breakpoint) {
        self.debugger.set_breakpoint(breakpoint)
    }

    /// Remove a previously set breakpoint.
    pub fn remove_breakpoint(&mut self, breakpoint: &Breakpoint) {
        self.debugger.remove_breakpoint(breakpoint)
    }

    pub(crate) fn eval_debugger_state(&mut self) -> DebugEval {
        let debugger = &mut self.debugger;

        let contract = self.frames.last().map(CallFrame::to);
        let pc = self.registers[RegId::PC].saturating_sub(self.registers[RegId::IS]);

        debugger.eval_state(contract, pc)
    }

    pub(crate) fn debugger_set_last_state(&mut self, state: ProgramState) {
        self.debugger.set_last_state(state)
    }

    pub(crate) const fn debugger_last_state(&self) -> &Option<ProgramState> {
        self.debugger.last_state()
    }
}

#[test]
fn breakpoint_script() {
    use fuel_asm::op;
    use fuel_tx::ConsensusParameters;

    let mut vm = Interpreter::<_, _>::with_memory_storage();

    let gas_limit = 1_000_000;
    let height = Default::default();

    let script = [
        op::addi(0x10, RegId::ZERO, 8),
        op::addi(0x11, RegId::ZERO, 16),
        op::addi(0x12, RegId::ZERO, 32),
        op::addi(0x13, RegId::ZERO, 64),
        op::addi(0x14, RegId::ZERO, 128),
        op::ret(0x10),
    ]
    .into_iter()
    .collect();

    let consensus_params = ConsensusParameters::standard();

    let tx = TransactionBuilder::script(script, vec![])
        .gas_limit(gas_limit)
        .add_random_fee_input()
        .finalize()
        .into_checked(height, &consensus_params)
        .expect("failed to generate checked tx");

    let suite = vec![
        (
            Breakpoint::script(0),
            vec![(0x10, 0), (0x11, 0), (0x12, 0), (0x13, 0), (0x14, 0)],
        ),
        (
            Breakpoint::script(2),
            vec![(0x10, 8), (0x11, 16), (0x12, 0), (0x13, 0), (0x14, 0)],
        ),
        (
            Breakpoint::script(3),
            vec![(0x10, 8), (0x11, 16), (0x12, 32), (0x13, 0), (0x14, 0)],
        ),
        (
            Breakpoint::script(5),
            vec![(0x10, 8), (0x11, 16), (0x12, 32), (0x13, 64), (0x14, 128)],
        ),
    ];

    suite.iter().for_each(|(b, _)| vm.set_breakpoint(*b));

    let state = vm
        .transact(tx)
        .map(ProgramState::from)
        .expect("Failed to execute script!");

    suite
        .into_iter()
        .fold(state, |state, (breakpoint, registers)| {
            let debug = state.debug_ref().expect("Expected breakpoint");
            let b = debug
                .breakpoint()
                .expect("State without expected breakpoint");

            assert_eq!(&breakpoint, b);
            registers.into_iter().for_each(|(r, w)| {
                assert_eq!(w, vm.registers()[r]);
            });

            vm.resume().expect("Failed to resume")
        });
}

#[test]
fn single_stepping() {
    use fuel_asm::op;
    use fuel_tx::ConsensusParameters;

    let mut vm = Interpreter::<_, _>::with_memory_storage();

    let gas_limit = 1_000_000;
    let height = Default::default();

    // Repeats the middle two instructions five times
    let script = [
        op::addi(0x10, RegId::ZERO, 5),
        op::addi(0x11, 0x11, 1),
        op::jnei(0x10, 0x11, 1),
        op::ret(0x10),
    ]
    .into_iter()
    .collect();

    let consensus_params = ConsensusParameters::standard();

    let tx = TransactionBuilder::script(script, vec![])
        .gas_limit(gas_limit)
        .add_random_fee_input()
        .finalize()
        .into_checked(height, &consensus_params)
        .expect("failed to generate checked tx");

    vm.set_single_stepping(true);

    let mut state = vm
        .transact(tx)
        .map(ProgramState::from)
        .expect("Failed to execute script!");

    let mut stops = Vec::new();

    while let Some(debug) = state.debug_ref() {
        let b = debug
            .breakpoint()
            .expect("State without expected breakpoint");

        stops.push(b.pc());

        state = vm.resume().expect("Failed to resume");
    }

    assert_eq!(stops, vec![0, 4, 8, 4, 8, 4, 8, 4, 8, 4, 8, 12]);
}
