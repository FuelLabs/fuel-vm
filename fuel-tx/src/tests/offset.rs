//! This module tests the offset calculation for each field of each transaction type in
//! canonical representation. Since each transaction can be executed inside FuelVM
//! it lives inside VM's memory. The predicate or script may want to have access to
//! different fields of transactions. An offset is used to access it.

use crate::{
    Upgrade,
    field::{
        BlobId as BlobIdField,
        InputContract,
        Inputs,
        MintAmount,
        MintAssetId,
        OutputContract,
        Outputs,
        ReceiptsRoot,
        Salt as SaltField,
        StorageSlots,
        TxPointer as TxPointerField,
        UpgradePurpose as UpgradePurposeField,
        Witnesses,
    },
    input,
    test_helper::TransactionFactory,
    *,
};
use fuel_types::{
    AssetId,
    ChainId,
    canonical::{
        Deserialize,
        Serialize,
    },
};
use rand::{
    Rng,
    SeedableRng,
    rngs::StdRng,
};

// Assert everything is tested. If some of these bools fails, just increase the number of
// cases
#[derive(Default)]
struct TestedFields {
    salt: bool,
    slots: bool,
    upgrade_purpose: bool,
    utxo_id: bool,
    owner: bool,
    asset_id: bool,
    predicate_coin: bool,
    predicate_message: bool,
    predicate_data_coin: bool,
    predicate_data_message: bool,
    contract_balance_root: bool,
    contract_state_root: bool,
    contract_id: bool,
    sender: bool,
    recipient: bool,
    message_data: bool,
    message_predicate: bool,
    message_predicate_data: bool,
    output_to: bool,
    output_asset_id: bool,
    output_balance_root: bool,
    output_contract_state_root: bool,
    output_contract_created_state_root: bool,
    output_contract_created_id: bool,
}

fn chargeable_transaction_parts<Tx>(tx: &Tx, bytes: &[u8], cases: &mut TestedFields)
where
    Tx: Buildable,
{
    inputs_assert(tx, bytes, cases);
    outputs_assert(tx, bytes, cases);
}

fn inputs_assert<Tx>(tx: &Tx, bytes: &[u8], cases: &mut TestedFields)
where
    Tx: Inputs + Buildable,
{
    tx.inputs().iter().enumerate().for_each(|(idx, i)| {
        let input_ofs = tx
            .inputs_offset_at(idx)
            .expect("failed to fetch input offset");
        let i_p =
            Input::from_bytes(&bytes[input_ofs..]).expect("failed to deserialize input");

        assert_eq!(i, &i_p);

        if let Some(utxo_id) = i.utxo_id() {
            cases.utxo_id = true;

            let ofs = input_ofs + i.repr().utxo_id_offset().expect("input have utxo_id");
            let utxo_id_p =
                UtxoId::from_bytes(&bytes[ofs..]).expect("failed to deserialize utxo id");

            assert_eq!(utxo_id, &utxo_id_p);
        }

        if let Some(owner) = i.input_owner() {
            cases.owner = true;

            let ofs = input_ofs + i.repr().owner_offset().expect("input contains owner");
            let owner_p =
                Address::from_bytes_ref_checked(&bytes[ofs..ofs + Address::LEN]).unwrap();

            assert_eq!(owner, owner_p);
        }

        if let Some(asset_id) = i.asset_id(&AssetId::BASE) {
            // Message doesn't store `AssetId` explicitly but works with base asset
            if let Some(offset) = i.repr().asset_id_offset() {
                cases.asset_id = true;

                let ofs = input_ofs + offset;
                let asset_id_p =
                    AssetId::from_bytes_ref_checked(&bytes[ofs..ofs + AssetId::LEN])
                        .unwrap();

                assert_eq!(asset_id, asset_id_p);
            }
        }

        if let Some(predicate) = i.input_predicate() {
            cases.predicate_coin =
                cases.predicate_coin || i.is_coin() && !predicate.is_empty();
            cases.predicate_message =
                cases.predicate_message || i.is_message() && !predicate.is_empty();

            let ofs = input_ofs + i.predicate_offset().expect("input contains predicate");
            let predicate_p = &bytes[ofs..ofs + predicate.len()];

            assert_eq!(predicate, predicate_p);
        }

        if let Some(predicate_data) = i.input_predicate_data() {
            cases.predicate_data_coin =
                cases.predicate_data_coin || i.is_coin() && !predicate_data.is_empty();
            cases.predicate_data_message = cases.predicate_data_message
                || i.is_message() && !predicate_data.is_empty();

            let ofs = input_ofs
                + i.predicate_data_offset()
                    .expect("input contains predicate data");
            let predicate_data_p = &bytes[ofs..ofs + predicate_data.len()];

            assert_eq!(predicate_data, predicate_data_p);
        }

        if let Some(balance_root) = i.balance_root() {
            cases.contract_balance_root = true;

            let ofs = input_ofs
                + i.repr()
                    .contract_balance_root_offset()
                    .expect("input contains balance root");

            let balance_root_p =
                Bytes32::from_bytes_ref_checked(&bytes[ofs..ofs + Bytes32::LEN]).unwrap();

            assert_eq!(balance_root, balance_root_p);
        }

        if let Some(state_root) = i.state_root() {
            cases.contract_state_root = true;

            let ofs = input_ofs
                + i.repr()
                    .contract_state_root_offset()
                    .expect("input contains state root");

            let state_root_p =
                Bytes32::from_bytes_ref_checked(&bytes[ofs..ofs + Bytes32::LEN]).unwrap();

            assert_eq!(state_root, state_root_p);
        }

        if let Some(contract_id) = i.contract_id() {
            cases.contract_id = true;

            let ofs = input_ofs
                + i.repr()
                    .contract_id_offset()
                    .expect("input contains contract id");

            let contract_id_p =
                ContractId::from_bytes_ref_checked(&bytes[ofs..ofs + ContractId::LEN])
                    .unwrap();

            assert_eq!(contract_id, contract_id_p);
        }

        if let Some(sender) = i.sender() {
            cases.sender = true;

            let ofs = input_ofs
                + i.repr()
                    .message_sender_offset()
                    .expect("input contains sender");

            let sender_p =
                Address::from_bytes_ref_checked(&bytes[ofs..ofs + Address::LEN]).unwrap();

            assert_eq!(sender, sender_p);
        }

        if let Some(recipient) = i.recipient() {
            cases.recipient = true;

            let ofs = input_ofs
                + i.repr()
                    .message_recipient_offset()
                    .expect("input contains recipient");

            let recipient_p =
                Address::from_bytes_ref_checked(&bytes[ofs..ofs + Address::LEN]).unwrap();

            assert_eq!(recipient, recipient_p);
        }

        if let Some(data) = i.input_data() {
            cases.message_data = cases.message_data || !data.is_empty();

            let ofs = input_ofs + i.repr().data_offset().expect("input contains data");
            let data_p = &bytes[ofs..ofs + data.len()];

            assert_eq!(data, data_p);
        }

        if i.is_message() {
            if let Some(predicate) = i.input_predicate() {
                cases.message_predicate =
                    cases.message_predicate || !predicate.is_empty();

                let ofs =
                    input_ofs + i.predicate_offset().expect("input contains predicate");
                let predicate_p = &bytes[ofs..ofs + predicate.len()];

                assert_eq!(predicate, predicate_p);
            }
        }

        if i.is_message() {
            if let Some(predicate_data) = i.input_predicate_data() {
                cases.message_predicate_data =
                    cases.message_predicate_data || !predicate_data.is_empty();

                let ofs = input_ofs
                    + i.predicate_data_offset()
                        .expect("input contains predicate data");
                let predicate_data_p = &bytes[ofs..ofs + predicate_data.len()];

                assert_eq!(predicate_data, predicate_data_p);
            }
        }
    });
}

fn outputs_assert<Tx: Outputs>(tx: &Tx, bytes: &[u8], cases: &mut TestedFields) {
    tx.outputs().iter().enumerate().for_each(|(idx, o)| {
        let output_ofs = tx
            .outputs_offset_at(idx)
            .expect("failed to fetch output offset");
        let o_p = Output::from_bytes(&bytes[output_ofs..])
            .expect("failed to deserialize output");

        assert_eq!(o, &o_p);

        if let Some(to) = o.to() {
            cases.output_to = true;

            let ofs = output_ofs + o.repr().to_offset().expect("output have to");
            let to_p =
                Address::from_bytes_ref_checked(&bytes[ofs..ofs + Address::LEN]).unwrap();

            assert_eq!(to, to_p);
        }

        if let Some(asset_id) = o.asset_id() {
            cases.output_asset_id = true;

            let ofs =
                output_ofs + o.repr().asset_id_offset().expect("output have asset id");
            let asset_id_p =
                AssetId::from_bytes_ref_checked(&bytes[ofs..ofs + Address::LEN]).unwrap();

            assert_eq!(asset_id, asset_id_p);
        }

        if let Some(balance_root) = o.balance_root() {
            cases.output_balance_root = true;

            let ofs = output_ofs
                + o.repr()
                    .contract_balance_root_offset()
                    .expect("output have balance root");
            let balance_root_p =
                Bytes32::from_bytes_ref_checked(&bytes[ofs..ofs + Bytes32::LEN]).unwrap();

            assert_eq!(balance_root, balance_root_p);
        }

        if let Some(state_root) = o.state_root() {
            let ofs = if o.is_contract() {
                cases.output_contract_state_root = true;
                o.repr()
                    .contract_state_root_offset()
                    .expect("output have state root")
            } else {
                cases.output_contract_created_state_root = true;
                o.repr()
                    .contract_created_state_root_offset()
                    .expect("output have state root")
            };

            let ofs = output_ofs + ofs;
            let state_root_p =
                Bytes32::from_bytes_ref_checked(&bytes[ofs..ofs + Bytes32::LEN]).unwrap();

            assert_eq!(state_root, state_root_p);
        }

        if let Some(contract_id) = o.contract_id() {
            cases.output_contract_created_id = true;

            let ofs = output_ofs
                + o.repr()
                    .contract_id_offset()
                    .expect("output have contract id");
            let contract_id_p =
                ContractId::from_bytes_ref_checked(&bytes[ofs..ofs + ContractId::LEN])
                    .unwrap();

            assert_eq!(contract_id, contract_id_p);
        }
    });
}

#[test]
fn tx_offset_create() {
    let mut cases = TestedFields::default();
    let number_cases = 100;

    // The seed will define how the transaction factory will generate a new transaction.
    // Different seeds might implicate on how many of the cases we cover - since we
    // assert coverage for all scenarios with the boolean variables above, we need to
    // pick a seed that, with low number of cases, will cover everything.
    TransactionFactory::<_, Create>::from_seed(1295)
        .take(number_cases)
        .for_each(|(tx, _)| {
            let bytes = tx.to_bytes();

            cases.salt = true;

            let ofs = tx.salt_offset();
            let salt_p =
                Salt::from_bytes_ref_checked(&bytes[ofs..ofs + Salt::LEN]).unwrap();

            assert_eq!(tx.salt(), salt_p);

            tx.storage_slots()
                .iter()
                .enumerate()
                .for_each(|(idx, slot)| {
                    cases.slots = true;

                    let ofs = tx
                        .storage_slots_offset_at(idx)
                        .expect("tx with slots contains offsets");

                    let bytes =
                        Bytes64::from_bytes_ref_checked(&bytes[ofs..ofs + Bytes64::LEN])
                            .unwrap();

                    let slot_p = StorageSlot::from(bytes);

                    assert_eq!(slot, &slot_p);
                });

            chargeable_transaction_parts(&tx, &bytes, &mut cases);
        });

    assert!(cases.salt);
    assert!(cases.slots);
    assert!(cases.utxo_id);
    assert!(cases.owner);
    assert!(cases.asset_id);
    assert!(cases.predicate_coin);
    assert!(cases.predicate_message);
    assert!(cases.predicate_data_coin);
    assert!(cases.predicate_data_message);
    assert!(cases.contract_balance_root);
    assert!(cases.contract_state_root);
    assert!(cases.contract_id);
    assert!(cases.sender);
    assert!(cases.recipient);
    assert!(cases.message_data);
    assert!(cases.message_predicate);
    assert!(cases.message_predicate_data);
    assert!(cases.output_to);
    assert!(cases.output_asset_id);
    assert!(cases.output_balance_root);
    assert!(cases.output_contract_state_root);
    assert!(cases.output_contract_created_state_root);
    assert!(cases.output_contract_created_id);
}

#[test]
fn tx_offset_script() {
    let mut cases = TestedFields::default();
    let number_cases = 100;

    // The seed will define how the transaction factory will generate a new transaction.
    // Different seeds might implicate on how many of the cases we cover - since we
    // assert coverage for all scenarios with the boolean variables above, we need to
    // pick a seed that, with low number of cases, will cover everything.
    TransactionFactory::<_, Script>::from_seed(1295)
        .take(number_cases)
        .for_each(|(tx, _)| {
            let bytes = tx.to_bytes();
            chargeable_transaction_parts(&tx, &bytes, &mut cases);
        });

    assert!(cases.utxo_id);
    assert!(cases.owner);
    assert!(cases.asset_id);
    assert!(cases.predicate_coin);
    assert!(cases.predicate_message);
    assert!(cases.predicate_data_coin);
    assert!(cases.predicate_data_message);
    assert!(cases.contract_balance_root);
    assert!(cases.contract_state_root);
    assert!(cases.contract_id);
    assert!(cases.sender);
    assert!(cases.recipient);
    assert!(cases.message_data);
    assert!(cases.message_predicate);
    assert!(cases.message_predicate_data);
    assert!(cases.output_to);
    assert!(cases.output_asset_id);
    assert!(cases.output_balance_root);
    assert!(cases.output_contract_state_root);
    assert!(cases.output_contract_created_state_root);
    assert!(cases.output_contract_created_id);
}

#[test]
fn tx_offset_upgrade() {
    let mut cases = TestedFields::default();
    let number_cases = 100;

    // The seed will define how the transaction factory will generate a new transaction.
    // Different seeds might implicate on how many of the cases we cover - since we
    // assert coverage for all scenarios with the boolean variables above, we need to
    // pick a seed that, with low number of cases, will cover everything.
    TransactionFactory::<_, Upgrade>::from_seed(1295)
        .take(number_cases)
        .for_each(|(tx, _)| {
            let bytes = tx.to_bytes();
            chargeable_transaction_parts(&tx, &bytes, &mut cases);
            cases.upgrade_purpose = true;

            let ofs = tx.upgrade_purpose_offset();
            let size = tx.upgrade_purpose().size();

            let purpose_p = UpgradePurpose::from_bytes(&bytes[ofs..ofs + size]).unwrap();

            assert_eq!(tx.upgrade_purpose(), &purpose_p);
        });

    // Upgrade parts
    assert!(cases.upgrade_purpose);

    // Chargeable parts
    assert!(cases.utxo_id);
    assert!(cases.owner);
    assert!(cases.asset_id);
    assert!(cases.predicate_coin);
    assert!(cases.predicate_message);
    assert!(cases.predicate_data_coin);
    assert!(cases.predicate_data_message);
    assert!(cases.contract_balance_root);
    assert!(cases.contract_state_root);
    assert!(cases.contract_id);
    assert!(cases.sender);
    assert!(cases.recipient);
    assert!(cases.message_data);
    assert!(cases.message_predicate);
    assert!(cases.message_predicate_data);
    assert!(cases.output_to);
    assert!(cases.output_asset_id);
    assert!(cases.output_balance_root);
    assert!(cases.output_contract_state_root);
    assert!(cases.output_contract_created_state_root);
    assert!(cases.output_contract_created_id);
}

#[test]
fn tx_offset_upload() {
    let mut cases = TestedFields::default();
    let number_cases = 100;

    // The seed will define how the transaction factory will generate a new transaction.
    // Different seeds might implicate on how many of the cases we cover - since we
    // assert coverage for all scenarios with the boolean variables above, we need to
    // pick a seed that, with low number of cases, will cover everything.
    TransactionFactory::<_, Upload>::from_seed(1295)
        .take(number_cases)
        .for_each(|(tx, _)| {
            let bytes = tx.to_bytes();
            chargeable_transaction_parts(&tx, &bytes, &mut cases);
        });

    // Chargeable parts
    assert!(cases.utxo_id);
    assert!(cases.owner);
    assert!(cases.asset_id);
    assert!(cases.predicate_coin);
    assert!(cases.predicate_message);
    assert!(cases.predicate_data_coin);
    assert!(cases.predicate_data_message);
    assert!(cases.contract_balance_root);
    assert!(cases.contract_state_root);
    assert!(cases.contract_id);
    assert!(cases.sender);
    assert!(cases.recipient);
    assert!(cases.message_data);
    assert!(cases.message_predicate);
    assert!(cases.message_predicate_data);
    assert!(cases.output_to);
    assert!(cases.output_asset_id);
    assert!(cases.output_balance_root);
    assert!(cases.output_contract_state_root);
    assert!(cases.output_contract_created_state_root);
    assert!(cases.output_contract_created_id);
}

#[test]
fn tx_offset_blob() {
    let mut cases = TestedFields::default();
    let number_cases = 100;

    // The seed will define how the transaction factory will generate a new transaction.
    // Different seeds might implicate on how many of the cases we cover - since we
    // assert coverage for all scenarios with the boolean variables above, we need to
    // pick a seed that, with low number of cases, will cover everything.
    TransactionFactory::<_, Blob>::from_seed(1295)
        .take(number_cases)
        .for_each(|(tx, _)| {
            let bytes = tx.to_bytes();

            // Blob id
            let offs = tx.blob_id_offset();
            assert_eq!(bytes[offs..offs + BlobId::LEN], **tx.blob_id());

            chargeable_transaction_parts(&tx, &bytes, &mut cases);
        });

    // Chargeable parts
    assert!(cases.utxo_id);
    assert!(cases.owner);
    assert!(cases.asset_id);
    assert!(cases.predicate_coin);
    assert!(cases.predicate_message);
    assert!(cases.predicate_data_coin);
    assert!(cases.predicate_data_message);
    assert!(cases.contract_balance_root);
    assert!(cases.contract_state_root);
    assert!(cases.contract_id);
    assert!(cases.sender);
    assert!(cases.recipient);
    assert!(cases.message_data);
    assert!(cases.message_predicate);
    assert!(cases.message_predicate_data);
    assert!(cases.output_to);
    assert!(cases.output_asset_id);
    assert!(cases.output_balance_root);
    assert!(cases.output_contract_state_root);
    assert!(cases.output_contract_created_state_root);
    assert!(cases.output_contract_created_id);
}

#[test]
fn tx_offset_mint() {
    let number_cases = 100;

    // The seed will define how the transaction factory will generate a new transaction.
    // Different seeds might implicate on how many of the cases we cover - since we
    // assert coverage for all scenarios with the boolean variables above, we need to
    // pick a seed that, with low number of cases, will cover everything.
    TransactionFactory::<_, Mint>::from_seed(1295)
        .take(number_cases)
        .for_each(|tx| {
            let bytes = tx.to_bytes();

            let ofs = tx.tx_pointer_offset();
            let tx_pointer_p = TxPointer::from_bytes(&bytes[ofs..ofs + TxPointer::LEN])
                .expect("Should decode `TxPointer`");

            assert_eq!(*tx.tx_pointer(), tx_pointer_p);

            let ofs = tx.input_contract_offset();
            let size = tx.input_contract().size();
            let input_p = input::contract::Contract::from_bytes(&bytes[ofs..ofs + size])
                .expect("Should decode `input::contract::Contract`");

            assert_eq!(*tx.input_contract(), input_p);

            let ofs = tx.output_contract_offset();
            let size = tx.output_contract().size();
            let output_p =
                output::contract::Contract::from_bytes(&bytes[ofs..ofs + size])
                    .expect("Should decode `output::contract::Contract`");

            assert_eq!(*tx.output_contract(), output_p);

            let ofs = tx.mint_amount_offset();
            let size = tx.mint_amount().size();
            let mint_amount_p =
                Word::from_bytes(&bytes[ofs..ofs + size]).expect("Should decode `Word`");

            assert_eq!(*tx.mint_amount(), mint_amount_p);

            let ofs = tx.mint_asset_id_offset();
            let size = tx.mint_asset_id().size();
            let mint_asset_id_p =
                <AssetId as Deserialize>::from_bytes(&bytes[ofs..ofs + size])
                    .expect("Should encode `AssetId`");

            assert_eq!(*tx.mint_asset_id(), mint_asset_id_p);
        });
}

#[test]
fn iow_offset() {
    let rng = &mut StdRng::seed_from_u64(8586);

    TransactionFactory::<_, Script>::from_seed(3493)
        .take(100)
        .for_each(|(mut tx, _)| {
            let bytes = tx.to_bytes();

            let mut tx_p = tx.clone();
            tx_p.precompute(&ChainId::default())
                .expect("Should be able to calculate cache");

            tx.inputs().iter().enumerate().for_each(|(x, i)| {
                let offset = tx.inputs_offset_at(x).unwrap();
                let offset_p = tx_p.inputs_offset_at(x).unwrap();
                assert_eq!(offset, offset_p);

                let input = Input::from_bytes(&bytes[offset..])
                    .expect("Failed to deserialize input!");

                assert_eq!(i, &input);
            });

            tx.outputs().iter().enumerate().for_each(|(x, o)| {
                let offset = tx.outputs_offset_at(x).unwrap();
                let offset_p = tx_p.outputs_offset_at(x).unwrap();
                assert_eq!(offset, offset_p);

                let output = Output::from_bytes(&bytes[offset..])
                    .expect("Failed to deserialize output!");

                assert_eq!(o, &output);
            });

            tx.witnesses().iter().enumerate().for_each(|(x, w)| {
                let offset = tx.witnesses_offset_at(x).unwrap();
                let offset_p = tx_p.witnesses_offset_at(x).unwrap();
                assert_eq!(offset, offset_p);

                let witness = Witness::from_bytes(&bytes[offset..])
                    .expect("Failed to deserialize witness!");

                assert_eq!(w, &witness);
            });

            let offset = tx.receipts_root_offset();
            let receipts_root = rng.r#gen();

            *tx.receipts_root_mut() = receipts_root;

            let bytes = tx.to_bytes();
            let receipts_root_p = &bytes[offset..offset + Bytes32::LEN];

            assert_eq!(&receipts_root[..], receipts_root_p);
        });
}
