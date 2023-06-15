use fuel_asm::{
    op,
    GTFArgs,
    PanicReason::{
        ArithmeticOverflow,
        ErrorFlag,
        MemoryOverflow,
    },
    RegId,
};
use fuel_crypto::Hasher;
use fuel_tx::TransactionBuilder;
use rand::{
    rngs::StdRng,
    SeedableRng,
};
use sha3::{
    Digest,
    Keccak256,
};

use crate::{
    prelude::*,
    util::test_helpers::check_expected_reason_for_instructions,
};

#[test]
fn ecrecover() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = Default::default();
    let height = Default::default();
    let params = ConsensusParameters::default();
    let gas_costs = GasCosts::default();

    let secret = SecretKey::random(rng);
    let public = secret.public_key();

    let message = b"The gift of words is the gift of deception and illusion.";
    let message = Message::new(message);

    let signature = Signature::sign(&secret, &message);

    #[rustfmt::skip]
    let script = vec![
        op::gtf_args(0x20, 0x00, GTFArgs::ScriptData),
        op::addi(0x21, 0x20, signature.as_ref().len() as Immediate12),
        op::addi(0x22, 0x21, message.as_ref().len() as Immediate12),
        op::movi(0x10, PublicKey::LEN as Immediate18),
        op::aloc(0x10),
        op::move_(0x11, RegId::HP),
        op::ecr(0x11, 0x20, 0x21),
        op::meq(0x12, 0x22, 0x11, 0x10),
        op::log(0x12, 0x00, 0x00, 0x00),
        op::ret(RegId::ONE),
    ].into_iter().collect();

    let script_data = signature
        .as_ref()
        .iter()
        .copied()
        .chain(message.as_ref().iter().copied())
        .chain(public.as_ref().iter().copied())
        .collect();

    let tx = TransactionBuilder::script(script, script_data)
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .maturity(maturity)
        .with_params(params)
        .add_random_fee_input()
        .finalize_checked(height, &gas_costs);

    let receipts = client.transact(tx);
    let success = receipts
        .iter()
        .any(|r| matches!(r, Receipt::Log{ ra, .. } if *ra == 1));

    assert!(success);
}

#[test]
fn ecrecover_tx_id() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = Default::default();
    let height = Default::default();
    let params = ConsensusParameters::default();
    let gas_costs = GasCosts::default();

    let secret = SecretKey::random(rng);
    let public = secret.public_key();

    #[rustfmt::skip]
    let script = vec![
        // 0x21 is a address of the singer of the witness
        op::gtf_args(0x20, 0x00, GTFArgs::ScriptData),
        op::move_(0x21, 0x20),
        // 0x22 is a witness - signature
        op::gtf_args(0x22, 0x00, GTFArgs::WitnessData),
        // TxId is stored in the first 32 bytes of the memory
        // Store it into register 0x23
        op::movi(0x23, 0),
        // Allocate space for the recovered public key
        // 0x10 contains the size of the public key = PublicKey::LEN
        op::movi(0x10, PublicKey::LEN as Immediate18),
        op::aloc(0x10),
        op::move_(0x11, RegId::HP),
        // Recover public key into `0x11` from `0x22` signature and TxId `0x23`
        op::ecr(0x11, 0x22, 0x23),
        // Compare address `0x21` from script data with with recovered `0x11`
        // for length `0x10` = PublicKey::LEN
        op::meq(0x12, 0x21, 0x11, 0x10),
        op::ret(0x12),
    ].into_iter().collect();

    let script_data = public.as_ref().to_vec();

    let mut tx = TransactionBuilder::script(script, script_data)
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .maturity(maturity)
        .with_params(params)
        .add_random_fee_input()
        .finalize();

    tx.sign_inputs(&secret, &params.chain_id);
    let tx = tx.into_checked(height, &params, &gas_costs).unwrap();

    let receipts = client.transact(tx);
    let success = receipts
        .iter()
        .any(|r| matches!(r, Receipt::Return{ val, .. } if *val == 1));

    assert!(success);
}

#[test]
fn ecrecover_tx_id_predicate() {
    use crate::checked_transaction::EstimatePredicates;
    use rand::Rng;
    let rng = &mut StdRng::seed_from_u64(1234u64);

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = Default::default();
    let params = ConsensusParameters::default();
    let gas_costs = GasCosts::default();

    let secret = SecretKey::random(rng);
    let public = secret.public_key();

    #[rustfmt::skip]
    let predicate = vec![
        // 0x21 is a address of the singer of the witness
        op::gtf_args(0x20, 0x00, GTFArgs::ScriptData),
        op::move_(0x21, 0x20),
        // 0x22 is a witness - signature
        op::gtf_args(0x22, 0x00, GTFArgs::WitnessData),
        // TxId is stored in the first 32 bytes of the memory
        // Store it into register 0x23
        op::movi(0x23, 0),
        // Allocate space for the recovered public key
        // 0x10 contains the size of the public key = PublicKey::LEN
        op::movi(0x10, PublicKey::LEN as Immediate18),
        op::aloc(0x10),
        op::move_(0x11, RegId::HP),
        // Recover public key into `0x11` from `0x22` signature and TxId `0x23`
        op::ecr(0x11, 0x22, 0x23),
        // Compare address `0x21` from script data with with recovered `0x11`
        // for length `0x10` = PublicKey::LEN
        op::meq(0x12, 0x21, 0x11, 0x10),
        op::ret(0x12),
    ].into_iter().collect();

    let script_data = public.as_ref().to_vec();

    let input = Input::coin_predicate(
        rng.gen(),
        Input::predicate_owner(&predicate, &params.chain_id),
        1000,
        rng.gen(),
        Default::default(),
        rng.gen(),
        0,
        predicate,
        vec![],
    );

    let mut tx = TransactionBuilder::script(vec![], script_data)
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .maturity(maturity)
        .with_params(params)
        .add_input(input)
        .add_unsigned_coin_input(
            secret,
            rng.gen(),
            rng.gen(),
            rng.gen(),
            Default::default(),
            Default::default(),
        )
        .finalize();

    tx.estimate_predicates(&params, &gas_costs)
        .expect("Should estimate predicate successfully");
    tx.into_checked(maturity, &params, &gas_costs)
        .expect("Should check predicate successfully");
}

#[test]
fn ecrecover_error() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let secret = SecretKey::random(rng);

    let message = b"The gift of words is the gift of deception and illusion.";
    let message = Message::new(message);

    let signature = Signature::sign(&secret, &message);

    #[rustfmt::skip]
    let script = vec![
        // op::gtf_args(0x20, 0x00, GTFArgs::ScriptData),
        op::addi(0x21, 0x20, signature.as_ref().len() as Immediate12),
        op::addi(0x22, 0x21, message.as_ref().len() as Immediate12),
        op::movi(0x10, PublicKey::LEN as Immediate18),
        op::aloc(0x10),
        op::move_(0x11, RegId::HP),
        op::ecr(0x11, 0x20, 0x21),
    ];

    check_expected_reason_for_instructions(script, ErrorFlag)
}

#[test]
fn ecrecover_a_gt_vmaxram_sub_64() {
    let reg_a = 0x20;
    let reg_b = 0x21;

    #[rustfmt::skip]
    let script = vec![
        op::xor(reg_a, reg_a, reg_a),
        op::xor(reg_b, reg_b, reg_b),
        op::not(reg_a, reg_a),
        op::subi(reg_a, reg_a, 63),
        op::ecr(reg_a, reg_b, reg_b),
    ];

    check_expected_reason_for_instructions(script, MemoryOverflow);
}

#[test]
fn ecrecover_b_gt_vmaxram_sub_64() {
    let reg_a = 0x20;
    let reg_b = 0x21;

    #[rustfmt::skip]
    let script = vec![
        op::xor(reg_a, reg_a, reg_a),
        op::xor(reg_b, reg_b, reg_b),
        op::not(reg_a, reg_a),
        op::subi(reg_a, reg_a, 63),
        op::ecr(reg_b, reg_a, reg_b),
    ];

    check_expected_reason_for_instructions(script, ArithmeticOverflow);
}

#[test]
fn ecrecover_c_gt_vmaxram_sub_32() {
    let reg_a = 0x20;
    let reg_b = 0x21;

    #[rustfmt::skip]
    let script = vec![
        op::xor(reg_a, reg_a, reg_a),
        op::xor(reg_b, reg_b, reg_b),
        op::not(reg_a, reg_a),
        op::subi(reg_a, reg_a, 31),
        op::ecr(reg_b, reg_b, reg_a),
    ];

    check_expected_reason_for_instructions(script, ArithmeticOverflow);
}

#[test]
fn sha256() {
    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = Default::default();
    let height = Default::default();
    let params = ConsensusParameters::default();
    let gas_costs = GasCosts::default();

    let message = b"I say let the world go to hell, but I should always have my tea.";
    let hash = Hasher::hash(message);

    #[rustfmt::skip]
    let script = vec![
        op::gtf_args(0x20, 0x00, GTFArgs::ScriptData),
        op::addi(0x21, 0x20, message.len() as Immediate12),
        op::movi(0x10, Bytes32::LEN as Immediate18),
        op::aloc(0x10),
        op::move_(0x11, RegId::HP),
        op::movi(0x12, message.len() as Immediate18),
        op::s256(0x11, 0x20, 0x12),
        op::meq(0x13, 0x11, 0x21, 0x10),
        op::log(0x13, 0x00, 0x00, 0x00),
        op::ret(RegId::ONE),
    ].into_iter().collect();

    let script_data = message
        .iter()
        .copied()
        .chain(hash.as_ref().iter().copied())
        .collect();

    let tx = TransactionBuilder::script(script, script_data)
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .maturity(maturity)
        .add_random_fee_input()
        .with_params(params)
        .finalize_checked(height, &gas_costs);

    let receipts = client.transact(tx);
    let success = receipts
        .iter()
        .any(|r| matches!(r, Receipt::Log{ ra, .. } if *ra == 1));

    assert!(success);
}

#[test]
fn s256_a_gt_vmaxram_sub_32() {
    let reg_a = 0x20;
    let reg_b = 0x21;

    #[rustfmt::skip]
        let script = vec![
        op::xor(reg_a, reg_a, reg_a),
        op::xor(reg_b, reg_b, reg_b),
        op::not(reg_a, reg_a),
        op::s256(reg_a, reg_b, reg_b),
    ];

    check_expected_reason_for_instructions(script, MemoryOverflow);
}

#[test]
fn s256_c_gt_mem_max() {
    let reg_a = 0x20;
    let reg_b = 0x21;

    #[rustfmt::skip]
        let script = vec![
        op::xor(reg_a, reg_a, reg_a),
        op::xor(reg_b, reg_b, reg_b),
        op::not(reg_a, reg_a),
        op::s256(reg_b, reg_b, reg_a),
    ];

    check_expected_reason_for_instructions(script, MemoryOverflow);
}

#[test]
fn s256_b_gt_vmaxram_sub_c() {
    let reg_a = 0x20;
    let reg_b = 0x21;

    #[rustfmt::skip]
        let script = vec![
        op::xor(reg_a, reg_a, reg_a),
        op::xor(reg_b, reg_b, reg_b),
        op::not(reg_a, reg_a),
        op::s256(reg_b, reg_a, reg_b),
    ];

    check_expected_reason_for_instructions(script, MemoryOverflow);
}

#[test]
fn keccak256() {
    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = Default::default();
    let height = Default::default();
    let params = ConsensusParameters::default();

    let message = b"...and, moreover, I consider it my duty to warn you that the cat is an ancient, inviolable animal.";

    let mut hasher = Keccak256::new();
    hasher.update(message);
    let hash = hasher.finalize();

    #[rustfmt::skip]
    let script = vec![
        op::gtf_args(0x20, 0x00, GTFArgs::ScriptData),
        op::addi(0x21, 0x20, message.len() as Immediate12),
        op::movi(0x10, Bytes32::LEN as Immediate18),
        op::aloc(0x10),
        op::move_(0x11, RegId::HP),
        op::movi(0x12, message.len() as Immediate18),
        op::k256(0x11, 0x20, 0x12),
        op::meq(0x13, 0x11, 0x21, 0x10),
        op::log(0x13, 0x00, 0x00, 0x00),
        op::ret(RegId::ONE),
    ].into_iter().collect();

    let script_data = message
        .iter()
        .copied()
        .chain(hash.iter().copied())
        .collect();

    let tx = TransactionBuilder::script(script, script_data)
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .maturity(maturity)
        .add_random_fee_input()
        .with_params(params)
        .finalize_checked(height, client.gas_costs());

    let receipts = client.transact(tx);
    let success = receipts
        .iter()
        .any(|r| matches!(r, Receipt::Log{ ra, .. } if *ra == 1));

    assert!(success);
}

#[test]
fn k256_a_gt_vmaxram_sub_32() {
    let reg_a = 0x20;
    let reg_b = 0x21;

    #[rustfmt::skip]
        let script = vec![
        op::xor(reg_a, reg_a, reg_a),
        op::xor(reg_b, reg_b, reg_b),
        op::not(reg_a, reg_a),
        op::k256(reg_a, reg_b, reg_b),
    ];

    check_expected_reason_for_instructions(script, MemoryOverflow);
}

#[test]
fn k256_c_gt_mem_max() {
    let reg_a = 0x20;
    let reg_b = 0x21;

    #[rustfmt::skip]
    let script = vec![
        op::xor(reg_a, reg_a, reg_a),
        op::xor(reg_b, reg_b, reg_b),
        op::not(reg_a, reg_a),
        op::k256(reg_b, reg_b, reg_a),
    ];

    check_expected_reason_for_instructions(script, MemoryOverflow);
}

#[test]
fn k256_b_gt_vmaxram_sub_c() {
    let reg_a = 0x20;
    let reg_b = 0x21;

    #[rustfmt::skip]
        let script = vec![
        op::xor(reg_a, reg_a, reg_a),
        op::xor(reg_b, reg_b, reg_b),
        op::not(reg_a, reg_a),
        op::k256(reg_b, reg_a, reg_b),
    ];

    check_expected_reason_for_instructions(script, MemoryOverflow);
}
