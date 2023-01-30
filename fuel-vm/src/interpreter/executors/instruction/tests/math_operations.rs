use num_integer::Roots;
use rayon::prelude::*;

use super::checked_nth_root;

/// Check for https://github.com/FuelLabs/fuel-vm/issues/150
#[test]
fn mlog_rounding_issues() {
    assert_eq!(999u32.checked_ilog(10), Some(2));
    assert_eq!(1000u32.checked_ilog(10), Some(3));
    assert_eq!(1001u32.checked_ilog(10), Some(3));
}

/// Verify some subsets of possible inputs against a known-good implementation.
#[test]
#[ignore = "This is super slow to run"]
fn mroo_verify_subsets() {
    (2..(u32::MAX as u64)).into_par_iter().for_each(|a| {
        for b in 2..64 {
            assert_eq!(checked_nth_root(a, b), Some(a.nth_root(b as u32)));
        }
    });
    ((u64::MAX - (u32::MAX as u64))..u64::MAX)
        .into_par_iter()
        .for_each(|a| {
            for b in 2..64 {
                assert_eq!(checked_nth_root(a, b), Some(a.nth_root(b as u32)));
            }
        });
}
