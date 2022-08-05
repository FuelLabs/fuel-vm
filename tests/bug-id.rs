use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use glob::glob;
use regex::Regex;
use strum::VariantNames;

use fuel_vm::prelude::*;

#[test]
fn check_bug_id_unique() {
    let mut matches = HashSet::new();
    let re = Regex::new(r"BugId::ID\d{3}").expect("failed to create regex");
    let error_source = PathBuf::new().join("src").join("error.rs");

    for source in glob("src/**/*.rs").expect("Failed to read glob pattern") {
        let source = source.expect("failed to fetch source from glob");

        if source != error_source {
            let source = fs::read_to_string(source).expect("failed to read source");

            if source.contains("BugId::*") {
                panic!("BugId isn't supposed to be required as '*'")
            }

            re.find_iter(&source).map(|m| m.as_str().to_string()).for_each(|s| {
                if !matches.insert(s.clone()) {
                    panic!("duplicated bug id detected: {}", s);
                }
            });
        }
    }

    BugId::VARIANTS.iter().for_each(|v| {
        let v = format!("BugId::{}", v);

        if !re.is_match(&v) {
            panic!("the bug id format is inconsistent: {}", v);
        }

        if !matches.contains(&v) {
            panic!("the bug id variant is never constructed: {}", v);
        }
    });
}
