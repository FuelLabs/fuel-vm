use fuel_crypto::SecretKey;

use crate::context::Context;

use super::*;

#[test]
fn test_ecrecover() -> Result<(), RuntimeError> {
    let mut memory = VmMemory::fully_allocated();
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

    memory.force_write_bytes(sig_address, &signature);
    memory.force_write_bytes(msg_address, &message);

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
    let mem_public_key: [u8; PublicKey::LEN] = memory.read_bytes(recovered as usize).unwrap();
    assert_eq!(&mem_public_key, public_key.as_ref());
    Ok(())
}

#[test]
fn test_keccak256() -> Result<(), RuntimeError> {
    let mut memory = VmMemory::fully_allocated();
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
    keccak256(&mut memory, owner, RegMut::new(&mut pc), hash, bytes_address, num_bytes)?;
    assert_eq!(pc, 8);
    let hash_bytes: [u8; 32] = memory.read_bytes(hash as usize).unwrap();
    assert_ne!(&hash_bytes, &[1u8; 32][..]);
    Ok(())
}

#[test]
fn test_sha256() -> Result<(), RuntimeError> {
    let mut memory = VmMemory::fully_allocated();
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
    sha256(&mut memory, owner, RegMut::new(&mut pc), hash, bytes_address, num_bytes)?;
    assert_eq!(pc, 8);
    let hash_bytes: [u8; 32] = memory.read_bytes(hash as usize).unwrap();
    assert_ne!(&hash_bytes, &[1u8; 32][..]);
    Ok(())
}
