use std::fs::File;
use fuel_vm_fuzz::execute;
use fuel_vm_fuzz::decode;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).expect("no path given");
    let mut file = File::create("gas_statistics.csv").unwrap();

    write!(file, "name\tgas\ttime_ms\n").unwrap();

    if Path::new(&path).is_file() {
        eprintln!("Pass directory")
    } else {
        let paths = fs::read_dir(path).unwrap();

        for path in paths {
            let entry = path.unwrap();
            let data = std::fs::read(entry.path()).unwrap();
            let name = entry.file_name();
            let name = name.to_str().unwrap();
            println!("{:?}", name);

            let Some(data) = decode(&data) else {  eprintln!("unable to decode"); continue; };

            let now = Instant::now();
            let result = execute(data);
            let gas = result.gas_used;

            write!(file, "{name}\t{gas}\t{}\n", now.elapsed().as_millis()).unwrap();
            if result.success {
                println!("{:?}:{}", name, result.success);
            }
        }
    }
}
