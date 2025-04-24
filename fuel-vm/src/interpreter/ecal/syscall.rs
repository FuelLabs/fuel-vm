//! Defines a syscall `ecal` handler for the FuelVM interpreter. It supports syscall used
//! by Sway for local development.

use crate::{
    interpreter::EcalHandler,
    prelude::{
        Interpreter,
        RegId,
    },
};
use fuel_asm::PanicReason;

#[cfg(feature = "alloc")]
use alloc::{
    format,
    string::String,
    vec::Vec,
};

/// Syscall ID for logging operation.
pub const LOG_SYSCALL: u64 = 1000;
/// File descriptor for standard output.
pub const STDOUT: u64 = 1;
/// File descriptor for standard error.
pub const STDERR: u64 = 2;

/// Handle VM `ecal` as syscalls.
///
/// The application of the syscalls can be turned off,
/// guaranteeing total isolation from the outside world.
///
/// Supported syscalls:
/// 1000 - write(fd: u64, buf: raw_ptr, count: u64) -> u64
#[derive(Debug, Clone)]
pub struct EcalSyscallHandler {
    enabled: bool,
    logs: Vec<String>,
}

impl Default for EcalSyscallHandler {
    fn default() -> Self {
        Self::new(false)
    }
}

impl EcalSyscallHandler {
    /// Creates a new instance of `EcalSyscallHandler`.
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            logs: Vec::new(),
        }
    }

    /// Returns the list of logs emitted during the execution.
    pub fn logs(&self) -> &[String] {
        &self.logs
    }
}

impl EcalHandler for EcalSyscallHandler {
    fn ecal<M, S, Tx, V>(
        vm: &mut Interpreter<M, S, Tx, Self, V>,
        a: RegId,
        b: RegId,
        c: RegId,
        d: RegId,
    ) -> crate::error::SimpleResult<()>
    where
        M: crate::prelude::Memory,
    {
        if !vm.ecal_state().enabled {
            return Err(PanicReason::EcalError.into());
        }

        let regs = vm.registers();
        match regs[a] {
            LOG_SYSCALL => {
                let fd = regs[b];

                let addr = regs[c];
                let size = regs[d];
                let bytes = vm.memory().read(addr, size)?;
                let log =
                    core::str::from_utf8(bytes).map_err(|_| PanicReason::EcalError)?;

                let log = match fd {
                    STDOUT => format!("stdout: {}", log),
                    STDERR => format!("stderr: {}", log),
                    _ => format!("fd {}: {}", fd, log),
                };

                vm.ecal_state_mut().logs.push(log);
            }
            _ => {
                return Err(PanicReason::EcalError.into());
            }
        };

        Ok(())
    }
}

#[allow(non_snake_case)]
#[allow(clippy::cast_possible_truncation)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        checked_transaction::{
            CheckPredicateParams,
            EstimatePredicates,
            IntoChecked,
        },
        prelude::MemoryInstance,
        storage::predicate::EmptyStorage,
        verification,
    };
    use fuel_asm::{
        op,
        GTFArgs,
    };
    use fuel_tx::{
        ConsensusParameters,
        Finalizable,
        Input,
        Script,
        TransactionBuilder,
    };
    use fuel_types::AssetId;

    #[cfg(feature = "alloc")]
    use alloc::vec;

    use tracing_test as _;

    #[test]
    fn log_syscall_works__stdout() {
        let syscall = EcalSyscallHandler::new(true);
        let mut vm =
        Interpreter::<_, _, Script, EcalSyscallHandler, verification::Normal>::with_memory_storage_and_ecal(
            syscall,
        );

        // Given
        let test_input = "Hello, LogSyscall!";
        let script_data: Vec<u8> = test_input.bytes().collect();

        let text_reg = 0x10;
        let text_size_reg = 0x11;
        let syscall_id_reg = 0x12;
        let fd_reg = 0x13;

        let script = vec![
            op::movi(syscall_id_reg, LOG_SYSCALL as u32),
            op::movi(fd_reg, STDOUT as u32),
            op::gtf_args(text_reg, 0x00, GTFArgs::ScriptData),
            op::movi(text_size_reg, script_data.len().try_into().unwrap()),
            op::ecal(syscall_id_reg, fd_reg, text_reg, text_size_reg),
            op::ret(RegId::ONE),
        ]
        .into_iter()
        .collect();

        // Execute transaction
        let tx = TransactionBuilder::script(script, script_data)
            .script_gas_limit(1_000_000)
            .add_fee_input()
            .finalize()
            .into_checked(Default::default(), &ConsensusParameters::standard())
            .expect("failed to generate a checked tx");
        let ready_tx = tx.test_into_ready();

        // When
        let _ = vm.transact(ready_tx);

        // Then
        let logs = vm.ecal_state().logs.clone();

        assert_eq!(logs.len(), 1);
        assert_eq!(format!("stdout: {}", test_input), logs[0]);
    }

    #[test]
    fn log_syscall_works__stderr() {
        let syscall = EcalSyscallHandler::new(true);
        let mut vm =
            Interpreter::<_, _, Script, EcalSyscallHandler, verification::Normal>::with_memory_storage_and_ecal(
                syscall,
            );

        // Given
        let test_input = "Hello, LogSyscall!";
        let script_data: Vec<u8> = test_input.bytes().collect();

        let text_reg = 0x10;
        let text_size_reg = 0x11;
        let syscall_id_reg = 0x12;
        let fd_reg = 0x13;

        let script = vec![
            op::movi(syscall_id_reg, LOG_SYSCALL as u32),
            op::movi(fd_reg, STDERR as u32),
            op::gtf_args(text_reg, 0x00, GTFArgs::ScriptData),
            op::movi(text_size_reg, script_data.len().try_into().unwrap()),
            op::ecal(syscall_id_reg, fd_reg, text_reg, text_size_reg),
            op::ret(RegId::ONE),
        ]
        .into_iter()
        .collect();

        // Execute transaction
        let tx = TransactionBuilder::script(script, script_data)
            .script_gas_limit(1_000_000)
            .add_fee_input()
            .finalize()
            .into_checked(Default::default(), &ConsensusParameters::standard())
            .expect("failed to generate a checked tx");
        let ready_tx = tx.test_into_ready();

        // When
        let _ = vm.transact(ready_tx);

        // Then
        let logs = vm.ecal_state().logs.clone();

        assert_eq!(logs.len(), 1);
        assert_eq!(format!("stderr: {}", test_input), logs[0]);
    }

    #[test]
    fn log_syscall_works__in_predicate_if_allowed() {
        let test_input = "Hello, LogSyscall!";
        let predicate_data: Vec<u8> = test_input.bytes().collect();

        let text_reg = 0x10;
        let text_size_reg = 0x11;
        let syscall_id_reg = 0x12;
        let fd_reg = 0x13;

        let predicate = vec![
            op::movi(syscall_id_reg, LOG_SYSCALL as u32),
            op::movi(fd_reg, STDOUT as u32),
            op::gtf_args(text_reg, 0x00, GTFArgs::InputCoinPredicateData),
            op::movi(text_size_reg, predicate_data.len().try_into().unwrap()),
            op::ecal(syscall_id_reg, fd_reg, text_reg, text_size_reg),
            op::ret(RegId::ONE),
        ]
        .into_iter()
        .collect();
        let owner = Input::predicate_owner(&predicate);

        let input = Input::coin_predicate(
            Default::default(),
            owner,
            100,
            AssetId::BASE,
            Default::default(),
            0,
            predicate,
            predicate_data,
        );

        let consensus_parameters = ConsensusParameters::standard();
        let mut tx = TransactionBuilder::script(vec![], vec![])
            .script_gas_limit(1_000_000)
            .add_input(input)
            .finalize();

        // Given
        let mut predicate_parameters: CheckPredicateParams =
            (&consensus_parameters).into();
        predicate_parameters.allow_syscall = true;
        tx.estimate_predicates(
            &predicate_parameters,
            MemoryInstance::new(),
            &EmptyStorage,
        )
        .unwrap();

        // When
        let result = tx.into_checked_reusable_memory(
            Default::default(),
            &consensus_parameters,
            &predicate_parameters,
            MemoryInstance::new(),
            &EmptyStorage,
        );

        // Then
        result.expect("Predicate with ecal should be executed successfully");
    }

    #[test]
    fn log_syscall_fails__in_predicate_if_not_allowed() {
        let test_input = "Hello, LogSyscall!";
        let predicate_data: Vec<u8> = test_input.bytes().collect();

        let text_reg = 0x10;
        let text_size_reg = 0x11;
        let syscall_id_reg = 0x12;
        let fd_reg = 0x13;

        let predicate = vec![
            op::movi(syscall_id_reg, LOG_SYSCALL as u32),
            op::movi(fd_reg, STDOUT as u32),
            op::gtf_args(text_reg, 0x00, GTFArgs::InputCoinPredicateData),
            op::movi(text_size_reg, predicate_data.len().try_into().unwrap()),
            op::ecal(syscall_id_reg, fd_reg, text_reg, text_size_reg),
            op::ret(RegId::ONE),
        ]
        .into_iter()
        .collect();
        let owner = Input::predicate_owner(&predicate);

        let input = Input::coin_predicate(
            Default::default(),
            owner,
            100,
            AssetId::BASE,
            Default::default(),
            0,
            predicate,
            predicate_data,
        );

        let consensus_parameters = ConsensusParameters::standard();
        let mut tx = TransactionBuilder::script(vec![], vec![])
            .script_gas_limit(1_000_000)
            .add_input(input)
            .finalize();

        // Given
        let mut predicate_parameters: CheckPredicateParams =
            (&consensus_parameters).into();
        predicate_parameters.allow_syscall = false;
        tx.estimate_predicates(
            &predicate_parameters,
            MemoryInstance::new(),
            &EmptyStorage,
        )
        .unwrap();

        // When
        let result = tx.into_checked_reusable_memory(
            Default::default(),
            &consensus_parameters,
            &predicate_parameters,
            MemoryInstance::new(),
            &EmptyStorage,
        );

        // Then
        result.expect_err("Predicate with ecal should fail if `allow_syscall` is false");
    }

    #[cfg(feature = "std")]
    #[test]
    #[tracing_test::traced_test]
    fn log_syscall_prints_to_tracing_in_predicates() {
        use tracing_test::internal::global_buf;

        let test_input = "Hello, \n LogSyscall!";
        let predicate_data: Vec<u8> = test_input.bytes().collect();

        let text_reg = 0x10;
        let text_size_reg = 0x11;
        let syscall_id_reg = 0x12;
        let fd_reg = 0x13;

        let predicate = vec![
            op::movi(syscall_id_reg, LOG_SYSCALL as u32),
            op::movi(fd_reg, STDOUT as u32),
            op::gtf_args(text_reg, 0x00, GTFArgs::InputCoinPredicateData),
            op::movi(text_size_reg, predicate_data.len().try_into().unwrap()),
            // Log 3 times
            op::ecal(syscall_id_reg, fd_reg, text_reg, text_size_reg),
            op::ecal(syscall_id_reg, fd_reg, text_reg, text_size_reg),
            op::ecal(syscall_id_reg, fd_reg, text_reg, text_size_reg),
            op::ret(RegId::ONE),
        ]
        .into_iter()
        .collect();
        let owner = Input::predicate_owner(&predicate);

        let input = Input::coin_predicate(
            Default::default(),
            owner,
            100,
            AssetId::BASE,
            Default::default(),
            0,
            predicate,
            predicate_data,
        );

        let consensus_parameters = ConsensusParameters::standard();
        let mut tx = TransactionBuilder::script(vec![], vec![])
            .script_gas_limit(1_000_000)
            .add_input(input)
            .finalize();

        // Given
        let mut predicate_parameters: CheckPredicateParams =
            (&consensus_parameters).into();
        predicate_parameters.allow_syscall = true;

        // Given
        let one_log = format!("stdout: {}", test_input);
        let expected_logs = format!("\n{}\n{}\n{}", one_log, one_log, one_log);
        let all_logs = String::from_utf8(global_buf().lock().unwrap().to_vec()).unwrap();
        assert!(!all_logs.contains(expected_logs.as_str()));

        // When
        tx.estimate_predicates(
            &predicate_parameters,
            MemoryInstance::new(),
            &EmptyStorage,
        )
        .unwrap();

        // Then
        let all_logs = String::from_utf8(global_buf().lock().unwrap().to_vec()).unwrap();
        assert!(all_logs.contains(expected_logs.as_str()));
    }
}
