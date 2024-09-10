//! See README.md for usage example

use fuel_vm_fuzz::FuzzData;
use fuel_vm_fuzz::{decode, encode};
use std::fs;
use std::path::PathBuf;

fn main() {
    let input = std::env::args().nth(1).expect("no input path given");
    let output = std::env::args().nth(2).expect("no output path given");
    let paths = fs::read_dir(input).expect("failed to read directory");

    for path in paths {
        let entry = path.unwrap();
        let program = std::fs::read(entry.path()).expect("failed to read file");

        println!("{:?}", entry.file_name().to_str().expect("faile to convert to string"));

        let data = FuzzData {
            program,
            sub_program: vec![],
            script_data: vec![],
        };

        let encoded = encode(&data);
        let decoded = decode(&encoded).expect("failed to decode");

        if decoded != data {
            println!("{:?}", data);
            println!("{:?}", decoded);
            panic!("mismatch")
        }

        fs::write(PathBuf::from(&output).join(entry.file_name()), &encoded).expect("failed to write file");
    }
}
