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
    interpreter::EcalHandler,
    prelude::{
        Interpreter,
        IntoChecked,
        MemoryClient,
        MemoryRange,
    },
    storage::MemoryStorage,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct FileReadEcal;

impl EcalHandler for FileReadEcal {
    fn ecal<S, Tx>(
        vm: &mut Interpreter<S, Tx, Self>,
        a: RegId,
        b: RegId,
        c: RegId,
        d: RegId,
    ) -> SimpleResult<()> {
        let a = vm.registers()[a]; // Seek offset
        let b = vm.registers()[b]; // Read length
        let c = vm.registers()[c]; // File path pointer in vm memory
        let d = vm.registers()[d]; // File path length

        vm.gas_charge(b.saturating_add(1))?;

        // Extract file path from vm memory
        let r = MemoryRange::new(c, d)?;
        let path = String::from_utf8_lossy(&vm.memory()[r.usizes()]);
        let path = PathBuf::from(path.as_ref());

        // Seek file to correct position
        let mut file = File::open(path).map_err(|_| PanicReason::EcalError)?;
        let _ = file
            .seek(SeekFrom::Start(a))
            .map_err(|_| PanicReason::EcalError)?;

        // Allocate the buffer in the vm memory and read directly from the file into it
        vm.allocate(b)?;
        let r = MemoryRange::new(vm.registers()[RegId::HP], b)?;
        file.read(&mut vm.memory_mut()[r.usizes()])
            .map_err(|_| PanicReason::EcalError)?;

        Ok(())
    }
}

fn main() {
    let vm: Interpreter<MemoryStorage, Script, FileReadEcal> =
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
        .gas_price(0)
        .script_gas_limit(1_000_000)
        .maturity(Default::default())
        .add_random_fee_input()
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
