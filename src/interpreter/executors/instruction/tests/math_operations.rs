use super::unchecked_ilog;

/// Check for https://github.com/FuelLabs/fuel-vm/issues/150
#[test]
fn mlog_rounding_issues() {
    assert_eq!(unchecked_ilog(999, 10), 2);
    assert_eq!(unchecked_ilog(1000, 10), 3);
    assert_eq!(unchecked_ilog(1001, 10), 3);
}
