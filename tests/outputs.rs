/// Testing of post-execution output handling

#[test]
fn transaction_byte_fees_are_charged_from_base_asset() {}

#[test]
fn transaction_gas_fees_are_charged_from_base_asset() {}

#[test]
fn base_asset_change_includes_unused_gas() {}

#[test]
fn base_asset_change_includes_unused_gas_on_revert() {}

// TODO: implement these test cases when TR opcode is supported
#[test]
#[ignore]
fn base_asset_change_is_reduced_by_contract_transfer() {}

#[test]
#[ignore]
fn base_asset_change_is_not_reduced_by_contract_transfer_on_revert() {}

#[test]
#[ignore]
fn asset_change_reduced_by_contract_transfer() {}

#[test]
#[ignore]
fn asset_change_not_reduced_by_contract_transfer_on_revert() {}
