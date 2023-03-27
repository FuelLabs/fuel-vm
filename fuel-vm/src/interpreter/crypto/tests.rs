use fuel_crypto::SecretKey;

use crate::context::Context;
use crate::interpreter::memory::Memory;

use super::*;

#[test]
fn test_ecrecover() -> Result<(), RuntimeError> {
    let mut memory: Memory<MEM_SIZE> = vec![1u8; MEM_SIZE].try_into().unwrap();
    let owner = OwnershipRegisters {
        sp: 1000,
        ssp: 1000,
        hp: 2000,
        prev_hp: VM_MAX_RAM - 1,
        context: Context::Call { block_height: 0 },
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

    ecrecover(
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
fn test_keccak256() -> Result<(), RuntimeError> {
    let mut memory: Memory<MEM_SIZE> = vec![1u8; MEM_SIZE].try_into().unwrap();
    let owner = OwnershipRegisters {
        sp: 1000,
        ssp: 1000,
        hp: 2000,
        prev_hp: VM_MAX_RAM - 1,
        context: Context::Call { block_height: 0 },
    };
    let mut pc = 4;
    let hash = 2100;
    let bytes_address = 0;
    let num_bytes = 100;
    keccak256(&mut memory, owner, RegMut::new(&mut pc), hash, bytes_address, num_bytes)?;
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
        context: Context::Call { block_height: 0 },
    };
    let mut pc = 4;
    let hash = 2100;
    let bytes_address = 0;
    let num_bytes = 100;
    sha256(&mut memory, owner, RegMut::new(&mut pc), hash, bytes_address, num_bytes)?;
    assert_eq!(pc, 8);
    assert_ne!(&memory[hash as usize..hash as usize + 32], &[1u8; 32][..]);
    Ok(())
}
