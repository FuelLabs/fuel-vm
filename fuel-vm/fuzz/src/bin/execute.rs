use fuel_vm_fuzz::execute;
use fuel_vm_fuzz::decode;
use std::path::Path;

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
        let paths = std::fs::read_dir(path).unwrap();

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
