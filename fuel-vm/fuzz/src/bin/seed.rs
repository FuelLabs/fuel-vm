//! See README.md for usage example

use fuel_vm_fuzz::FuzzData;
use fuel_vm_fuzz::{decode, encode};
use std::fs;
use std::path::PathBuf;

fn main() {
    let input = std::env::args().nth(1).expect("no input path given");
    let output = std::env::args().nth(2).expect("no output path given");
    let paths = fs::read_dir(input).unwrap();

    for path in paths {
        let entry = path.unwrap();
        let program = std::fs::read(entry.path()).unwrap();

        println!("{:?}", entry.file_name().to_str().unwrap());

        let data = FuzzData {
            program,
            sub_program: vec![],
            script_data: vec![],
        };

        let encoded = encode(&data);
        let decoded = decode(&encoded).unwrap();

        if decoded != data {
            println!("{:?}", data);
            println!("{:?}", decoded);
            panic!("mismatch")
        }

        fs::write(PathBuf::from(&output).join(entry.file_name()), &encoded).unwrap();
    }
}
