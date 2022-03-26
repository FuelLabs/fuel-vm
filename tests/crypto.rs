use fuel_crypto::Hasher;
use fuel_vm::consts::*;
use fuel_vm::crypto;
use fuel_vm::prelude::*;

use std::str::FromStr;

#[test]
fn ecrecover() {
    use secp256k1::{PublicKey, Secp256k1, SecretKey};

    let storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let maturity = 0;

    let secp = Secp256k1::new();
    let secret = SecretKey::from_str("3b940b5586823dfd02ae3b461bb4336b5ecbaefd6627aa922efc048fec0c881c").unwrap();
    let public = PublicKey::from_secret_key(&secp, &secret).serialize_uncompressed();
    let public = Bytes64::try_from(&public[1..]).expect("Failed to parse public key!");

    let message = b"The gift of words is the gift of deception and illusion.";
    let e = Hasher::hash(&message[..]);
    let sig =
        crypto::secp256k1_sign_compact_recoverable(secret.as_ref(), e.as_ref()).expect("Failed to generate signature");

    let alloc = e.len() + sig.len() + public.len() + public.len(); // Computed public key

    let mut script = vec![Opcode::MOVI(0x20, alloc as Immediate18), Opcode::ALOC(0x20)];

    e.iter()
        .chain(sig.iter())
        .chain(public.iter())
        .enumerate()
        .for_each(|(i, b)| {
            script.push(Opcode::MOVI(0x21, *b as Immediate18));
            script.push(Opcode::SB(REG_HP, 0x21, (i + 1) as Immediate12));
        });

    // Set `e` address to 0x30
    script.push(Opcode::ADDI(0x30, REG_HP, 1));

    // Set `sig` address to 0x31
    script.push(Opcode::ADDI(0x31, 0x30, e.len() as Immediate12));

    // Set `public` address to 0x32
    script.push(Opcode::ADDI(0x32, 0x31, sig.len() as Immediate12));

    // Set computed public key address to 0x33
    script.push(Opcode::ADDI(0x33, 0x32, public.len() as Immediate12));

    // Set public key length to 0x34
    script.push(Opcode::MOVI(0x34, public.len() as Immediate18));

    // Compute the ECRECOVER
    // m[computed public key] := ecrecover(sig, e)
    // r[0x10] := m[public] == m[computed public key]
    script.push(Opcode::ECR(0x33, 0x31, 0x30));
    script.push(Opcode::MEQ(0x10, 0x32, 0x33, 0x34));
    script.push(Opcode::LOG(0x10, 0, 0, 0));

    // Corrupt the signature
    // m[sig][0] := !m[sig][0]
    script.push(Opcode::LB(0x10, 0x30, 0));
    script.push(Opcode::NOT(0x10, 0x10));
    script.push(Opcode::SB(0x30, 0x10, 0));

    // Compute the corrupted ECRECOVER
    // m[computed public key] := ecrecover(sig', e)
    // r[0x10] := m[public] == m[computed public key]
    script.push(Opcode::ECR(0x33, 0x31, 0x30));
    script.push(Opcode::MEQ(0x10, 0x32, 0x33, 0x34));
    script.push(Opcode::LOG(0x10, 0, 0, 0));

    script.push(Opcode::RET(REG_ONE));

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        script.into_iter().collect(),
        vec![],
        vec![],
        vec![],
        vec![],
    );

    let receipts = Transactor::new(storage)
        .transact(tx)
        .receipts()
        .expect("Failed to execute script!")
        .to_owned();

    assert_eq!(receipts[0].ra().expect("$ra expected in receipt"), 1);
    assert_eq!(receipts[1].ra().expect("$ra expected in receipt"), 0);
}

#[test]
fn sha256() {
    let storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let maturity = 0;

    let message = b"I say let the world go to hell, but I should always have my tea.";
    let length = message.len() as Immediate12;
    let hash = Hasher::hash(message);

    let alloc = length  // message
        + 32 // reference hash
        + 32; // computed hash

    let mut script = vec![Opcode::MOVI(0x20, alloc as Immediate18), Opcode::ALOC(0x20)];

    message.iter().chain(hash.iter()).enumerate().for_each(|(i, b)| {
        script.push(Opcode::MOVI(0x21, *b as Immediate18));
        script.push(Opcode::SB(REG_HP, 0x21, (i + 1) as Immediate12));
    });

    // Set message address to 0x30
    script.push(Opcode::ADDI(0x30, REG_HP, 1));

    // Set hash address to 0x31
    script.push(Opcode::ADDI(0x31, 0x30, length));

    // Set computed hash address to 0x32
    script.push(Opcode::ADDI(0x32, 0x31, 32));

    // Set message length to 0x33
    script.push(Opcode::MOVI(0x33, length as Immediate18));

    // Set hash length to 0x34
    script.push(Opcode::MOVI(0x34, 32));

    // Compute the Keccak256
    // m[computed hash] := keccack256(m[message, length])
    // r[0x10] := m[hash] == m[computed hash]
    script.push(Opcode::S256(0x32, 0x30, 0x33));
    script.push(Opcode::MEQ(0x10, 0x31, 0x32, 0x34));
    script.push(Opcode::LOG(0x10, 0, 0, 0));

    // Corrupt the message
    // m[message][0] := !m[message][0]
    script.push(Opcode::LB(0x10, 0x30, 0));
    script.push(Opcode::NOT(0x10, 0x10));
    script.push(Opcode::SB(0x30, 0x10, 0));

    // Compute the Keccak256
    // m[computed hash] := keccack256(m[message, length])
    // r[0x10] := m[hash] == m[computed hash]
    script.push(Opcode::K256(0x32, 0x30, 0x33));
    script.push(Opcode::MEQ(0x10, 0x31, 0x32, 0x34));
    script.push(Opcode::LOG(0x10, 0, 0, 0));

    script.push(Opcode::RET(REG_ONE));

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        script.into_iter().collect(),
        vec![],
        vec![],
        vec![],
        vec![],
    );

    let receipts = Transactor::new(storage)
        .transact(tx)
        .receipts()
        .expect("Failed to execute script!")
        .to_owned();

    assert_eq!(receipts[0].ra().expect("$ra expected in receipt"), 1);
    assert_eq!(receipts[1].ra().expect("$ra expected in receipt"), 0);
}

#[test]
fn keccak256() {
    use sha3::{Digest, Keccak256};

    let storage = MemoryStorage::default();

    let gas_price = 0;
    let gas_limit = 1_000_000;
    let byte_price = 0;
    let maturity = 0;

    let message = b"...and, moreover, I consider it my duty to warn you that the cat is an ancient, inviolable animal.";
    let length = message.len() as Immediate12;

    let mut hasher = Keccak256::new();
    hasher.update(message);
    let hash = hasher.finalize();

    let alloc = length  // message
        + 32 // reference hash
        + 32; // computed hash

    let mut script = vec![Opcode::MOVI(0x20, alloc as Immediate18), Opcode::ALOC(0x20)];

    message.iter().chain(hash.iter()).enumerate().for_each(|(i, b)| {
        script.push(Opcode::MOVI(0x21, *b as Immediate18));
        script.push(Opcode::SB(REG_HP, 0x21, (i + 1) as Immediate12));
    });

    // Set message address to 0x30
    script.push(Opcode::ADDI(0x30, REG_HP, 1));

    // Set hash address to 0x31
    script.push(Opcode::ADDI(0x31, 0x30, length));

    // Set computed hash address to 0x32
    script.push(Opcode::ADDI(0x32, 0x31, 32));

    // Set message length to 0x33
    script.push(Opcode::MOVI(0x33, length as Immediate18));

    // Set hash length to 0x34
    script.push(Opcode::MOVI(0x34, 32));

    // Compute the Keccak256
    // m[computed hash] := keccack256(m[message, length])
    // r[0x10] := m[hash] == m[computed hash]
    script.push(Opcode::K256(0x32, 0x30, 0x33));
    script.push(Opcode::MEQ(0x10, 0x31, 0x32, 0x34));
    script.push(Opcode::LOG(0x10, 0, 0, 0));

    // Corrupt the message
    // m[message][0] := !m[message][0]
    script.push(Opcode::LB(0x10, 0x30, 0));
    script.push(Opcode::NOT(0x10, 0x10));
    script.push(Opcode::SB(0x30, 0x10, 0));

    // Compute the Keccak256
    // m[computed hash] := keccack256(m[message, length])
    // r[0x10] := m[hash] == m[computed hash]
    script.push(Opcode::K256(0x32, 0x30, 0x33));
    script.push(Opcode::MEQ(0x10, 0x31, 0x32, 0x34));
    script.push(Opcode::LOG(0x10, 0, 0, 0));

    script.push(Opcode::RET(REG_ONE));

    let tx = Transaction::script(
        gas_price,
        gas_limit,
        byte_price,
        maturity,
        script.into_iter().collect(),
        vec![],
        vec![],
        vec![],
        vec![],
    );

    let receipts = Transactor::new(storage)
        .transact(tx)
        .receipts()
        .expect("Failed to execute script!")
        .to_owned();

    assert_eq!(receipts[0].ra().expect("$ra expected in receipt"), 1);
    assert_eq!(receipts[1].ra().expect("$ra expected in receipt"), 0);
}
