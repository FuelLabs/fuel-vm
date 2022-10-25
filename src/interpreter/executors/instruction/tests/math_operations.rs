use super::checked_ilog;

/// Check for https://github.com/FuelLabs/fuel-vm/issues/150
#[test]
fn mlog_rounding_issues() {
    assert_eq!(checked_ilog(999, 10), Some(2));
    assert_eq!(checked_ilog(1000, 10), Some(3));
    assert_eq!(checked_ilog(1001, 10), Some(3));
}
