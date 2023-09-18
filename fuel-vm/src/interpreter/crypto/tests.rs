use fuel_crypto::SecretKey;
use rand::{
    rngs::StdRng,
    SeedableRng,
};

use crate::{
    context::Context,
    interpreter::memory::Memory,
};

use super::*;

#[test]
fn test_recover_secp256k1() -> Result<(), RuntimeError> {
    let mut memory: Memory<MEM_SIZE> = vec![1u8; MEM_SIZE].try_into().unwrap();
    let owner = OwnershipRegisters {
        sp: 1000,
        ssp: 1000,
        hp: 2000,
        prev_hp: VM_MAX_RAM - 1,
        context: Context::Call {
            block_height: Default::default(),
        },
    };
    let mut err = 0;
    let mut pc = 4;

    let recovered = 2100;
    let sig_address = 0;
    let msg_address = 64;

    let secret = SecretKey::try_from(&[2u8; 32][..]).unwrap();
    let public_key = PublicKey::from(&secret);
    let message = Message::new([3u8; 100]);
    let signature = Signature::sign(&secret, &message);

    memory[sig_address..sig_address + Signature::LEN].copy_from_slice(signature.as_ref());
    memory[msg_address..msg_address + Message::LEN].copy_from_slice(message.as_ref());

    secp256k1_recover(
        &mut memory,
        owner,
        RegMut::new(&mut err),
        RegMut::new(&mut pc),
        recovered,
        sig_address as Word,
        msg_address as Word,
    )?;
    assert_eq!(pc, 8);
    assert_eq!(err, 0);
    assert_eq!(
        &memory[recovered as usize..recovered as usize + PublicKey::LEN],
        public_key.as_ref()
    );
    Ok(())
}

#[test]
fn test_recover_secp256r1() -> Result<(), RuntimeError> {
    use fuel_crypto::secp256r1::encode_pubkey;
    use p256::ecdsa::SigningKey;

    let mut rng = &mut StdRng::seed_from_u64(8586);

    let mut memory: Memory<MEM_SIZE> = vec![1u8; MEM_SIZE].try_into().unwrap();
    let owner = OwnershipRegisters {
        sp: 1000,
        ssp: 1000,
        hp: 2000,
        prev_hp: VM_MAX_RAM - 1,
        context: Context::Call {
            block_height: Default::default(),
        },
    };
    let mut err = 0;
    let mut pc = 4;

    let recovered = 2100;
    let sig_address = 0;
    let msg_address = 64;

    let signing_key = SigningKey::random(&mut rng);
    let verifying_key = signing_key.verifying_key();

    let message = Message::new([3u8; 100]);
    let signature = fuel_crypto::secp256r1::sign_prehashed(&signing_key, &message)
        .expect("Signing failed");

    memory[sig_address..sig_address + Bytes64::LEN].copy_from_slice(&*signature);
    memory[msg_address..msg_address + Message::LEN].copy_from_slice(message.as_ref());

    secp256r1_recover(
        &mut memory,
        owner,
        RegMut::new(&mut err),
        RegMut::new(&mut pc),
        recovered,
        sig_address as Word,
        msg_address as Word,
    )?;
    assert_eq!(pc, 8);
    assert_eq!(err, 0);
    assert_eq!(
        &memory[recovered as usize..recovered as usize + Bytes64::LEN],
        &encode_pubkey(*verifying_key)
    );
    Ok(())
}

#[test]
fn test_verify_ed25519() -> Result<(), RuntimeError> {
    use ed25519_dalek::Signer;

    let mut memory: Memory<MEM_SIZE> = vec![1u8; MEM_SIZE].try_into().unwrap();
    let mut err = 0;
    let mut pc = 4;

    let sig_address = 0;
    let msg_address = 64;
    let pubkey_address = 64 + 32;

    let mut rng = rand::rngs::OsRng;
    let signing_key = ed25519_dalek::SigningKey::generate(&mut rng);

    let message = Message::new([3u8; 100]);
    let signature = signing_key.sign(&*message);

    memory[sig_address..sig_address + Signature::LEN]
        .copy_from_slice(&signature.to_bytes());
    memory[msg_address..msg_address + Message::LEN].copy_from_slice(message.as_ref());
    memory[pubkey_address..pubkey_address + Bytes32::LEN]
        .copy_from_slice(signing_key.verifying_key().as_ref());

    ed25519_verify(
        &mut memory,
        RegMut::new(&mut err),
        RegMut::new(&mut pc),
        pubkey_address as Word,
        sig_address as Word,
        msg_address as Word,
    )?;
    assert_eq!(pc, 8);
    assert_eq!(err, 0);
    Ok(())
}

#[test]
fn test_keccak256() -> Result<(), RuntimeError> {
    let mut memory: Memory<MEM_SIZE> = vec![1u8; MEM_SIZE].try_into().unwrap();
    let owner = OwnershipRegisters {
        sp: 1000,
        ssp: 1000,
        hp: 2000,
        prev_hp: VM_MAX_RAM - 1,
        context: Context::Call {
            block_height: Default::default(),
        },
    };
    let mut pc = 4;
    let hash = 2100;
    let bytes_address = 0;
    let num_bytes = 100;
    keccak256(
        &mut memory,
        owner,
        RegMut::new(&mut pc),
        hash,
        bytes_address,
        num_bytes,
    )?;
    assert_eq!(pc, 8);
    assert_ne!(&memory[hash as usize..hash as usize + 32], &[1u8; 32][..]);
    Ok(())
}

#[test]
fn test_sha256() -> Result<(), RuntimeError> {
    let mut memory: Memory<MEM_SIZE> = vec![1u8; MEM_SIZE].try_into().unwrap();
    let owner = OwnershipRegisters {
        sp: 1000,
        ssp: 1000,
        hp: 2000,
        prev_hp: VM_MAX_RAM - 1,
        context: Context::Call {
            block_height: Default::default(),
        },
    };
    let mut pc = 4;
    let hash = 2100;
    let bytes_address = 0;
    let num_bytes = 100;
    sha256(
        &mut memory,
        owner,
        RegMut::new(&mut pc),
        hash,
        bytes_address,
        num_bytes,
    )?;
    assert_eq!(pc, 8);
    assert_ne!(&memory[hash as usize..hash as usize + 32], &[1u8; 32][..]);
    Ok(())
}
