use fuel_vm::consts::WORD_SIZE;
use fuel_vm::fuel_asm::op;
use fuel_vm::fuel_asm::RegId;
use fuel_vm::fuel_asm::{Instruction, RawInstruction};
use fuel_vm::fuel_crypto::rand::Rng;
use fuel_vm::fuel_crypto::rand::SeedableRng;
use fuel_vm::fuel_types::Word;
use fuel_vm::prelude::SecretKey;
use fuel_vm_fuzz::execute;
use fuel_vm_fuzz::FuzzData;
use fuel_vm_fuzz::{decode, decode_instructions, encode};
use std::convert::TryFrom;
use std::convert::TryInto;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let path = std::env::args().nth(1).expect("no path given");

    if Path::new(&path).is_file() {
        let data = std::fs::read(&path).unwrap();

        let data = decode(&data).unwrap();

        let result = execute(data);
        if result.success {
            println!("{:?}:{}", path, result.success);
        }
    } else {
        let paths = fs::read_dir(path).unwrap();

        for path in paths {
            let entry = path.unwrap();
            println!("{:?}", entry.file_name());

            let data = std::fs::read(entry.path()).unwrap();

            let data = decode(&data).unwrap();

            let result = execute(data);
            if result.success {
                println!("{:?}:{}", entry.file_name(), result.success);
            }
        }
    }
}
