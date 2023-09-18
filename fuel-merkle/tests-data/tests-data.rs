use std::{
    error::Error,
    fs::File,
    path::Path,
};

use fuel_merkle_test_helpers::data::sparse::Test;

fn test(path: &Path) -> datatest_stable::Result<()> {
    let data_file = File::open(path)?;
    let test: Test = serde_yaml::from_reader(data_file)?;
    test.execute().map_err(|e| Box::new(e) as Box<dyn Error>)
}

datatest_stable::harness!(test, "./tests-data/fixtures", r"^.*/*");
