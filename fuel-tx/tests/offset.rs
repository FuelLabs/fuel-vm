use fuel_tx::*;
use fuel_tx_test_helpers::TransactionFactory;
use fuel_types::bytes::{Deserializable, SerializableVec};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[test]
fn tx_offset() {
    // Assert everything is tested. If some of these bools fails, just increase the number of
    // cases
    let mut tested_salt = false;
    let mut tested_slots = false;
    let mut tested_utxo_id = false;
    let mut tested_owner = false;
    let mut tested_asset_id = false;
    let mut tested_predicate_coin = false;
    let mut tested_predicate_message = false;
    let mut tested_predicate_data_coin = false;
    let mut tested_predicate_data_message = false;
    let mut tested_contract_balance_root = false;
    let mut tested_contract_state_root = false;
    let mut tested_contract_id = false;
    let mut tested_message_id = false;
    let mut tested_sender = false;
    let mut tested_recipient = false;
    let mut tested_message_data = false;
    let mut tested_message_predicate = false;
    let mut tested_message_predicate_data = false;
    let mut tested_output_to = false;
    let mut tested_output_asset_id = false;
    let mut tested_output_balance_root = false;
    let mut tested_output_contract_state_root = false;
    let mut tested_output_contract_created_state_root = false;
    let mut tested_output_contract_created_id = false;
    let mut tested_output_recipient = false;

    let cases = 100;

    // The seed will define how the transaction factory will generate a new transaction. Different
    // seeds might implicate on how many of the cases we cover - since we assert coverage for all
    // scenarios with the boolean variables above, we need to pick a seed that, with low number of
    // cases, will cover everything.
    TransactionFactory::from_seed(1295)
        .take(cases)
        .for_each(|(mut tx, _)| {
            let bytes = tx.to_bytes();

            if let Some(salt) = tx.salt() {
                tested_salt = true;

                let ofs = tx.salt_offset().expect("tx with salt is create");
                let salt_p = unsafe { Salt::as_ref_unchecked(&bytes[ofs..ofs + Salt::LEN]) };

                assert_eq!(salt, salt_p);
            }

            if let Some(slots) = tx.storage_slots() {
                slots.iter().enumerate().for_each(|(idx, slot)| {
                    tested_slots = true;

                    let ofs = tx
                        .storage_slot_offset(idx)
                        .expect("tx with slots contains offsets");

                    let bytes =
                        unsafe { Bytes64::as_ref_unchecked(&bytes[ofs..ofs + Bytes64::LEN]) };

                    let slot_p = StorageSlot::from(bytes);

                    assert_eq!(slot, &slot_p);
                })
            }

            tx.inputs().iter().enumerate().for_each(|(idx, i)| {
                let input_ofs = tx.input_offset(idx).expect("failed to fetch input offset");
                let i_p =
                    Input::from_bytes(&bytes[input_ofs..]).expect("failed to deserialize input");

                assert_eq!(i, &i_p);

                if let Some(utxo_id) = i.utxo_id() {
                    tested_utxo_id = true;

                    let ofs = input_ofs + i.repr().utxo_id_offset().expect("input have utxo_id");
                    let utxo_id_p =
                        UtxoId::from_bytes(&bytes[ofs..]).expect("failed to deserialize utxo id");

                    assert_eq!(utxo_id, &utxo_id_p);
                }

                if let Some(owner) = i.input_owner() {
                    tested_owner = true;

                    let ofs = input_ofs + i.repr().owner_offset().expect("input contains owner");
                    let owner_p =
                        unsafe { Address::as_ref_unchecked(&bytes[ofs..ofs + Address::LEN]) };

                    assert_eq!(owner, owner_p);
                }

                if let Some(asset_id) = i.asset_id() {
                    // Message doesn't store `AssetId` explicitly but works with base asset
                    if let Some(offset) = i.repr().asset_id_offset() {
                        tested_asset_id = true;

                        let ofs = input_ofs + offset;
                        let asset_id_p =
                            unsafe { AssetId::as_ref_unchecked(&bytes[ofs..ofs + AssetId::LEN]) };

                        assert_eq!(asset_id, asset_id_p);
                    }
                }

                if let Some(predicate) = i.input_predicate() {
                    tested_predicate_coin =
                        tested_predicate_coin || i.is_coin() && !predicate.is_empty();
                    tested_predicate_message =
                        tested_predicate_message || i.is_message() && !predicate.is_empty();

                    let ofs = input_ofs + i.predicate_offset().expect("input contains predicate");
                    let predicate_p = &bytes[ofs..ofs + predicate.len()];

                    assert_eq!(predicate, predicate_p);
                }

                if let Some(predicate_data) = i.input_predicate_data() {
                    tested_predicate_data_coin =
                        tested_predicate_data_coin || i.is_coin() && !predicate_data.is_empty();
                    tested_predicate_data_message = tested_predicate_data_message
                        || i.is_message() && !predicate_data.is_empty();

                    let ofs = input_ofs
                        + i.predicate_data_offset()
                            .expect("input contains predicate data");
                    let predicate_data_p = &bytes[ofs..ofs + predicate_data.len()];

                    assert_eq!(predicate_data, predicate_data_p);
                }

                if let Some(balance_root) = i.balance_root() {
                    tested_contract_balance_root = true;

                    let ofs = input_ofs
                        + i.repr()
                            .contract_balance_root_offset()
                            .expect("input contains balance root");

                    let balance_root_p =
                        unsafe { Bytes32::as_ref_unchecked(&bytes[ofs..ofs + Bytes32::LEN]) };

                    assert_eq!(balance_root, balance_root_p);
                }

                if let Some(state_root) = i.state_root() {
                    tested_contract_state_root = true;

                    let ofs = input_ofs
                        + i.repr()
                            .contract_state_root_offset()
                            .expect("input contains state root");

                    let state_root_p =
                        unsafe { Bytes32::as_ref_unchecked(&bytes[ofs..ofs + Bytes32::LEN]) };

                    assert_eq!(state_root, state_root_p);
                }

                if let Some(contract_id) = i.contract_id() {
                    tested_contract_id = true;

                    let ofs = input_ofs
                        + i.repr()
                            .contract_id_offset()
                            .expect("input contains contract id");

                    let contract_id_p =
                        unsafe { ContractId::as_ref_unchecked(&bytes[ofs..ofs + ContractId::LEN]) };

                    assert_eq!(contract_id, contract_id_p);
                }

                if let Some(message_id) = i.message_id() {
                    tested_message_id = true;

                    let ofs = input_ofs
                        + i.repr()
                            .message_id_offset()
                            .expect("input contains message id");

                    let message_id_p =
                        unsafe { MessageId::as_ref_unchecked(&bytes[ofs..ofs + MessageId::LEN]) };

                    assert_eq!(message_id, message_id_p);
                }

                if let Some(sender) = i.sender() {
                    tested_sender = true;

                    let ofs = input_ofs
                        + i.repr()
                            .message_sender_offset()
                            .expect("input contains sender");

                    let sender_p =
                        unsafe { Address::as_ref_unchecked(&bytes[ofs..ofs + Address::LEN]) };

                    assert_eq!(sender, sender_p);
                }

                if let Some(recipient) = i.recipient() {
                    tested_recipient = true;

                    let ofs = input_ofs
                        + i.repr()
                            .message_recipient_offset()
                            .expect("input contains recipient");

                    let recipient_p =
                        unsafe { Address::as_ref_unchecked(&bytes[ofs..ofs + Address::LEN]) };

                    assert_eq!(recipient, recipient_p);
                }

                if let Some(data) = i.input_data() {
                    tested_message_data = tested_message_data || !data.is_empty();

                    let ofs = input_ofs + i.repr().data_offset().expect("input contains data");
                    let data_p = &bytes[ofs..ofs + data.len()];

                    assert_eq!(data, data_p);
                }

                if i.is_message() {
                    if let Some(predicate) = i.input_predicate() {
                        tested_message_predicate =
                            tested_message_predicate || !predicate.is_empty();

                        let ofs =
                            input_ofs + i.predicate_offset().expect("input contains predicate");
                        let predicate_p = &bytes[ofs..ofs + predicate.len()];

                        assert_eq!(predicate, predicate_p);
                    }
                }

                if i.is_message() {
                    if let Some(predicate_data) = i.input_predicate_data() {
                        tested_message_predicate_data =
                            tested_message_predicate_data || !predicate_data.is_empty();

                        let ofs = input_ofs
                            + i.predicate_data_offset()
                                .expect("input contains predicate data");
                        let predicate_data_p = &bytes[ofs..ofs + predicate_data.len()];

                        assert_eq!(predicate_data, predicate_data_p);
                    }
                }
            });

            tx.outputs().iter().enumerate().for_each(|(idx, o)| {
                let output_ofs = tx
                    .output_offset(idx)
                    .expect("failed to fetch output offset");
                let o_p =
                    Output::from_bytes(&bytes[output_ofs..]).expect("failed to deserialize output");

                assert_eq!(o, &o_p);

                if let Some(to) = o.to() {
                    tested_output_to = true;

                    let ofs = output_ofs + o.repr().to_offset().expect("output have to");
                    let to_p =
                        unsafe { Address::as_ref_unchecked(&bytes[ofs..ofs + Address::LEN]) };

                    assert_eq!(to, to_p);
                }

                if let Some(asset_id) = o.asset_id() {
                    tested_output_asset_id = true;

                    let ofs =
                        output_ofs + o.repr().asset_id_offset().expect("output have asset id");
                    let asset_id_p =
                        unsafe { AssetId::as_ref_unchecked(&bytes[ofs..ofs + Address::LEN]) };

                    assert_eq!(asset_id, asset_id_p);
                }

                if let Some(balance_root) = o.balance_root() {
                    tested_output_balance_root = true;

                    let ofs = output_ofs
                        + o.repr()
                            .contract_balance_root_offset()
                            .expect("output have balance root");
                    let balance_root_p =
                        unsafe { Bytes32::as_ref_unchecked(&bytes[ofs..ofs + Bytes32::LEN]) };

                    assert_eq!(balance_root, balance_root_p);
                }

                if let Some(state_root) = o.state_root() {
                    let ofs = if o.is_contract() {
                        tested_output_contract_state_root = true;
                        o.repr()
                            .contract_state_root_offset()
                            .expect("output have state root")
                    } else {
                        tested_output_contract_created_state_root = true;
                        o.repr()
                            .contract_created_state_root_offset()
                            .expect("output have state root")
                    };

                    let ofs = output_ofs + ofs;
                    let state_root_p =
                        unsafe { Bytes32::as_ref_unchecked(&bytes[ofs..ofs + Bytes32::LEN]) };

                    assert_eq!(state_root, state_root_p);
                }

                if let Some(contract_id) = o.contract_id() {
                    tested_output_contract_created_id = true;

                    let ofs = output_ofs
                        + o.repr()
                            .contract_id_offset()
                            .expect("output have contract id");
                    let contract_id_p =
                        unsafe { ContractId::as_ref_unchecked(&bytes[ofs..ofs + ContractId::LEN]) };

                    assert_eq!(contract_id, contract_id_p);
                }

                if let Some(recipient) = o.recipient() {
                    tested_output_recipient = true;

                    let ofs =
                        output_ofs + o.repr().recipient_offset().expect("output have recipient");
                    let recipient_p =
                        unsafe { Address::as_ref_unchecked(&bytes[ofs..ofs + Address::LEN]) };

                    assert_eq!(recipient, recipient_p);
                }
            });
        });

    assert!(tested_salt);
    assert!(tested_slots);
    assert!(tested_utxo_id);
    assert!(tested_owner);
    assert!(tested_asset_id);
    assert!(tested_predicate_coin);
    assert!(tested_predicate_message);
    assert!(tested_predicate_data_coin);
    assert!(tested_predicate_data_message);
    assert!(tested_contract_balance_root);
    assert!(tested_contract_state_root);
    assert!(tested_contract_id);
    assert!(tested_message_id);
    assert!(tested_sender);
    assert!(tested_recipient);
    assert!(tested_message_data);
    assert!(tested_message_predicate);
    assert!(tested_message_predicate_data);
    assert!(tested_output_to);
    assert!(tested_output_asset_id);
    assert!(tested_output_balance_root);
    assert!(tested_output_contract_state_root);
    assert!(tested_output_contract_created_state_root);
    assert!(tested_output_contract_created_id);
    assert!(tested_output_recipient);
}

#[test]
fn iow_offset() {
    let rng = &mut StdRng::seed_from_u64(8586);

    TransactionFactory::from_seed(3493)
        .take(100)
        .for_each(|(mut tx, _)| {
            let bytes = tx.to_bytes();

            let mut tx_p = tx.clone();
            tx_p.precompute_metadata();

            tx.inputs().iter().enumerate().for_each(|(x, i)| {
                let offset = tx.input_offset(x).unwrap();
                let offset_p = tx_p.input_offset(x).unwrap();

                let input =
                    Input::from_bytes(&bytes[offset..]).expect("Failed to deserialize input!");

                assert_eq!(i, &input);
                assert_eq!(offset, offset_p);
            });

            tx.outputs().iter().enumerate().for_each(|(x, o)| {
                let offset = tx.output_offset(x).unwrap();
                let offset_p = tx_p.output_offset(x).unwrap();

                let output =
                    Output::from_bytes(&bytes[offset..]).expect("Failed to deserialize output!");

                assert_eq!(o, &output);
                assert_eq!(offset, offset_p);
            });

            tx.witnesses().iter().enumerate().for_each(|(x, w)| {
                let offset = tx.witness_offset(x).unwrap();
                let offset_p = tx_p.witness_offset(x).unwrap();

                let witness =
                    Witness::from_bytes(&bytes[offset..]).expect("Failed to deserialize witness!");

                assert_eq!(w, &witness);
                assert_eq!(offset, offset_p);
            });

            if let Some(offset) = tx.receipts_root_offset() {
                let receipts_root = rng.gen();

                tx.set_receipts_root(receipts_root);

                let bytes = tx.to_bytes();
                let receipts_root_p = &bytes[offset..offset + Bytes32::LEN];

                assert_eq!(&receipts_root[..], receipts_root_p);
            }
        });
}
