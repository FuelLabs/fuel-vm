use fuel_crypto::Hasher;
use fuel_tx::TransactionBuilder;
use rand::rngs::StdRng;
use rand::SeedableRng;
use sha3::{Digest, Keccak256};

use fuel_vm::consts::*;
use fuel_vm::prelude::*;

#[test]
fn ecrecover() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::default();

    let secret = SecretKey::random(rng);
    let public = secret.public_key();

    let message = b"The gift of words is the gift of deception and illusion.";
    let message = Message::new(message);

    let signature = Signature::sign(&secret, &message);

    #[rustfmt::skip]
    let script = vec![
        Opcode::gtf(0x20, 0x00, GTFArgs::ScriptData),
        Opcode::ADDI(0x21, 0x20, signature.as_ref().len() as Immediate12),
        Opcode::ADDI(0x22, 0x21, message.as_ref().len() as Immediate12),
        Opcode::MOVI(0x10, PublicKey::LEN as Immediate18),
        Opcode::ALOC(0x10),
        Opcode::ADDI(0x11, REG_HP, 1),
        Opcode::ECR(0x11, 0x20, 0x21),
        Opcode::MEQ(0x12, 0x22, 0x11, 0x10),
        Opcode::LOG(0x12, 0x00, 0x00, 0x00),
        Opcode::RET(REG_ONE),
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
        .finalize_checked(height, &params);

    let receipts = client.transact(tx);
    let success = receipts.iter().any(|r| matches!(r, Receipt::Log{ ra, .. } if *ra == 1));

    assert!(success);
}

#[test]
fn sha256() {
    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::default();

    let message = b"I say let the world go to hell, but I should always have my tea.";
    let hash = Hasher::hash(message);

    #[rustfmt::skip]
    let script = vec![
        Opcode::gtf(0x20, 0x00, GTFArgs::ScriptData),
        Opcode::ADDI(0x21, 0x20, message.len() as Immediate12),
        Opcode::MOVI(0x10, Bytes32::LEN as Immediate18),
        Opcode::ALOC(0x10),
        Opcode::ADDI(0x11, REG_HP, 1),
        Opcode::MOVI(0x12, message.len() as Immediate18),
        Opcode::S256(0x11, 0x20, 0x12),
        Opcode::MEQ(0x13, 0x11, 0x21, 0x10),
        Opcode::LOG(0x13, 0x00, 0x00, 0x00),
        Opcode::RET(REG_ONE),
    ].into_iter().collect();

    let script_data = message.iter().copied().chain(hash.as_ref().iter().copied()).collect();

    let tx = TransactionBuilder::script(script, script_data)
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .maturity(maturity)
        .finalize_checked(height, &params);

    let receipts = client.transact(tx);
    let success = receipts.iter().any(|r| matches!(r, Receipt::Log{ ra, .. } if *ra == 1));

    assert!(success);
}

#[test]
fn keccak256() {
    let mut client = MemoryClient::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let maturity = 0;
    let height = 0;
    let params = ConsensusParameters::default();

    let message = b"...and, moreover, I consider it my duty to warn you that the cat is an ancient, inviolable animal.";

    let mut hasher = Keccak256::new();
    hasher.update(message);
    let hash = hasher.finalize();

    #[rustfmt::skip]
    let script = vec![
        Opcode::gtf(0x20, 0x00, GTFArgs::ScriptData),
        Opcode::ADDI(0x21, 0x20, message.len() as Immediate12),
        Opcode::MOVI(0x10, Bytes32::LEN as Immediate18),
        Opcode::ALOC(0x10),
        Opcode::ADDI(0x11, REG_HP, 1),
        Opcode::MOVI(0x12, message.len() as Immediate18),
        Opcode::K256(0x11, 0x20, 0x12),
        Opcode::MEQ(0x13, 0x11, 0x21, 0x10),
        Opcode::LOG(0x13, 0x00, 0x00, 0x00),
        Opcode::RET(REG_ONE),
    ].into_iter().collect();

    let script_data = message.iter().copied().chain(hash.iter().copied()).collect();

    let tx = TransactionBuilder::script(script, script_data)
        .gas_price(gas_price)
        .gas_limit(gas_limit)
        .maturity(maturity)
        .finalize_checked(height, &params);

    let receipts = client.transact(tx);
    let success = receipts.iter().any(|r| matches!(r, Receipt::Log{ ra, .. } if *ra == 1));

    assert!(success);
}
