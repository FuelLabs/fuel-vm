use fuel_vm_fuzz::execute;
use fuel_vm_fuzz::decode;
use std::path::Path;

fn main() {
    let path = std::env::args().nth(1).expect("no path given");

    if Path::new(&path).is_file() {
        let data = std::fs::read(&path).expect("failed to read file");

        let data = decode(&data).expect("failed to decode data");

        let result = execute(data);
        if result.success {
            println!("{:?}:{}", path, result.success);
        }
    } else {
        let paths = std::fs::read_dir(path).expect("failed to read dir");

        for path in paths {
            let entry = path.expect("failed to yield directory entry");
            println!("{:?}", entry.file_name());

            let data = std::fs::read(entry.path()).expect("failed to read file");

            let data = decode(&data).expect("failed to decode");

            let result = execute(data);
            if result.success {
                println!("{:?}:{}", entry.file_name(), result.success);
            }
        }
    }
}
