//! This example shows how you can provide a custom ECAL instruction to the VM.
//! Here we use it to provide a way to read from arbitrary files on the host machine.

use std::{
    fs::{
        self,
        File,
    },
    io::{
        Read,
        Seek,
        SeekFrom,
    },
    path::PathBuf,
    sync::{
        Arc,
        Mutex,
    },
};

use fuel_asm::{
    op,
    GTFArgs,
    PanicReason,
    RegId,
};
use fuel_tx::{
    ConsensusParameters,
    Finalizable,
    Receipt,
    Script,
    TransactionBuilder,
};
use fuel_vm::{
    error::SimpleResult,
    interpreter::{
        EcalHandler,
        Memory,
    },
    prelude::{
        Interpreter,
        IntoChecked,
        MemoryClient,
    },
    storage::MemoryStorage,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct FileReadEcal;

impl EcalHandler for FileReadEcal {
    fn ecal<M, S, Tx, V>(
        vm: &mut Interpreter<M, S, Tx, Self, V>,
        a: RegId,
        b: RegId,
        c: RegId,
        d: RegId,
    ) -> SimpleResult<()>
    where
        M: Memory,
    {
        let a = vm.registers()[a]; // Seek offset
        let b = vm.registers()[b]; // Read length
        let c = vm.registers()[c]; // File path pointer in vm memory
        let d = vm.registers()[d]; // File path length

        vm.gas_charge(b.saturating_add(1))?;

        // Extract file path from vm memory
        let path = String::from_utf8_lossy(vm.memory().read(c, d)?);
        let path = PathBuf::from(path.as_ref());

        // Seek file to correct position
        let mut file = File::open(path).map_err(|_| PanicReason::EcalError)?;
        let _ = file
            .seek(SeekFrom::Start(a))
            .map_err(|_| PanicReason::EcalError)?;

        // Allocate the buffer in the vm memory and read directly from the file into it
        vm.allocate(b)?;
        let hp = vm.registers()[RegId::HP];
        file.read(vm.memory_mut().write_noownerchecks(hp, b)?)
            .map_err(|_| PanicReason::EcalError)?;

        Ok(())
    }
}

fn example_file_read() {
    let vm: Interpreter<_, MemoryStorage, Script, FileReadEcal> =
        Interpreter::with_memory_storage();

    let script_data: Vec<u8> = file!().bytes().collect();
    let script = vec![
        op::movi(0x20, 4),                                     // Seek 4 bytes
        op::movi(0x21, 8),                                     // Read next 8 bytes
        op::gtf_args(0x22, 0x00, GTFArgs::ScriptData),         // File path pointer
        op::movi(0x23, script_data.len().try_into().unwrap()), // File path length
        op::ecal(0x20, 0x21, 0x22, 0x23),                      // Read from file
        op::logd(RegId::ZERO, RegId::ZERO, RegId::HP, 0x21),   // Log the result
        op::ret(RegId::ONE),
    ]
    .into_iter()
    .collect();

    let mut client = MemoryClient::from_txtor(vm.into());
    let consensus_params = ConsensusParameters::standard();
    let tx = TransactionBuilder::script(script, script_data)
        .script_gas_limit(1_000_000)
        .maturity(Default::default())
        .add_fee_input()
        .finalize()
        .into_checked(Default::default(), &consensus_params)
        .expect("failed to generate a checked tx");
    client.transact(tx);
    let receipts = client.receipts().expect("Expected receipts");

    let Receipt::LogData { data, .. } = receipts.first().unwrap() else {
        panic!("Expected a data log receipt");
    };

    let read_bytes = data.as_ref().unwrap();
    let expected_bytes = &fs::read(file!()).expect("Couldn't read")[4..12];
    assert_eq!(read_bytes, expected_bytes);
}

#[derive(Debug, Clone, Default)]
pub struct CounterEcal {
    counter: u64,
}

impl EcalHandler for CounterEcal {
    fn ecal<M, S, Tx, V>(
        vm: &mut Interpreter<M, S, Tx, Self, V>,
        a: RegId,
        _b: RegId,
        _c: RegId,
        _d: RegId,
    ) -> SimpleResult<()>
    where
        M: Memory,
    {
        vm.registers_mut()[a] = vm.ecal_state().counter;
        vm.ecal_state_mut().counter += 1;
        vm.gas_charge(1)?;
        Ok(())
    }
}

fn example_counter() {
    let mut vm: Interpreter<_, MemoryStorage, Script, CounterEcal> =
        Interpreter::with_memory_storage();

    vm.ecal_state_mut().counter = 5;

    let script_data: Vec<u8> = file!().bytes().collect();
    let script = vec![
        op::ecal(0x20, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::ecal(0x21, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::ecal(0x22, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::ecal(0x23, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::log(0x20, 0x21, 0x22, 0x23),
        op::ret(RegId::ONE),
    ]
    .into_iter()
    .collect();

    let mut client = MemoryClient::from_txtor(vm.into());
    let consensus_params = ConsensusParameters::standard();
    let tx = TransactionBuilder::script(script, script_data)
        .script_gas_limit(1_000_000)
        .maturity(Default::default())
        .add_fee_input()
        .finalize()
        .into_checked(Default::default(), &consensus_params)
        .expect("failed to generate a checked tx");
    client.transact(tx);
    let receipts = client.receipts().expect("Expected receipts");

    let Receipt::Log { ra, rb, rc, rd, .. } = receipts.first().unwrap() else {
        panic!("Expected a log receipt");
    };

    assert_eq!(*ra, 5);
    assert_eq!(*rb, 6);
    assert_eq!(*rc, 7);
    assert_eq!(*rd, 8);
}

#[derive(Debug, Clone)]
pub struct SharedCounterEcal {
    counter: Arc<Mutex<u64>>,
}

impl EcalHandler for SharedCounterEcal {
    fn ecal<M, S, Tx, V>(
        vm: &mut Interpreter<M, S, Tx, Self, V>,
        a: RegId,
        _b: RegId,
        _c: RegId,
        _d: RegId,
    ) -> SimpleResult<()> {
        let mut counter = vm.ecal_state().counter.lock().expect("poisoned");
        let old_value = *counter;
        *counter += 1;
        drop(counter);
        vm.registers_mut()[a] = old_value;
        vm.gas_charge(1)?;
        Ok(())
    }
}

fn example_shared_counter() {
    let vm: Interpreter<_, MemoryStorage, Script, SharedCounterEcal> =
        Interpreter::with_memory_storage_and_ecal(SharedCounterEcal {
            counter: Arc::new(Mutex::new(5)),
        });

    let script_data: Vec<u8> = file!().bytes().collect();
    let script = vec![
        op::ecal(0x20, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::ecal(0x21, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::ecal(0x22, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::ecal(0x23, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::log(0x20, 0x21, 0x22, 0x23),
        op::ret(RegId::ONE),
    ]
    .into_iter()
    .collect();

    let mut client = MemoryClient::from_txtor(vm.into());
    let consensus_params = ConsensusParameters::standard();
    let tx = TransactionBuilder::script(script, script_data)
        .script_gas_limit(1_000_000)
        .maturity(Default::default())
        .add_fee_input()
        .finalize()
        .into_checked(Default::default(), &consensus_params)
        .expect("failed to generate a checked tx");
    client.transact(tx);
    let receipts = client.receipts().expect("Expected receipts");

    let Receipt::Log { ra, rb, rc, rd, .. } = receipts.first().unwrap() else {
        panic!("Expected a log receipt");
    };

    assert_eq!(*ra, 5);
    assert_eq!(*rb, 6);
    assert_eq!(*rc, 7);
    assert_eq!(*rd, 8);
}

fn main() {
    example_file_read();
    example_counter();
    example_shared_counter();
}
