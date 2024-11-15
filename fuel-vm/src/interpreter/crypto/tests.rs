#![allow(clippy::cast_possible_truncation)]
use alloc::vec;

use fuel_crypto::SecretKey;
use rand::{
    rngs::StdRng,
    RngCore,
    SeedableRng,
};
use rstest::rstest;

use super::*;
use fuel_vm::consts::*;

#[cfg(feature = "random")]
#[test]
fn test_recover_secp256k1() -> SimpleResult<()> {
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let owner = OwnershipRegisters {
        sp: 1000,
        ssp: 1000,
        hp: 2000,
        prev_hp: VM_MAX_RAM - 1,
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
fn test_recover_secp256r1() -> SimpleResult<()> {
    use fuel_crypto::secp256r1::encode_pubkey;
    use p256::ecdsa::SigningKey;

    let mut rng = &mut StdRng::seed_from_u64(8586);

    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let owner = OwnershipRegisters {
        sp: 1000,
        ssp: 1000,
        hp: 2000,
        prev_hp: VM_MAX_RAM - 1,
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
fn test_verify_ed25519() -> SimpleResult<()> {
    use ed25519_dalek::Signer;

    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let mut err = 0;
    let mut pc = 4;

    let pubkey_address = 0;
    let sig_address = pubkey_address + 32;
    let msg_address = sig_address + 64;

    let mut rng = rand::rngs::OsRng;
    let signing_key = ed25519_dalek::SigningKey::generate(&mut rng);

    let mut message = [0u8; 100];
    rng.fill_bytes(&mut message);
    let signature = signing_key.sign(&message);

    memory[pubkey_address..pubkey_address + Bytes32::LEN]
        .copy_from_slice(signing_key.verifying_key().as_ref());
    memory[sig_address..sig_address + Signature::LEN]
        .copy_from_slice(&signature.to_bytes());
    memory[msg_address..msg_address + message.len()].copy_from_slice(message.as_ref());

    ed25519_verify(
        &mut memory,
        RegMut::new(&mut err),
        RegMut::new(&mut pc),
        pubkey_address as Word,
        sig_address as Word,
        msg_address as Word,
        message.len() as Word,
    )?;
    assert_eq!(pc, 8);
    assert_eq!(err, 0);
    Ok(())
}

#[test]
fn test_keccak256() -> SimpleResult<()> {
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let owner = OwnershipRegisters {
        sp: 1000,
        ssp: 1000,
        hp: 2000,
        prev_hp: VM_MAX_RAM - 1,
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
fn test_sha256() -> SimpleResult<()> {
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let owner = OwnershipRegisters {
        sp: 1000,
        ssp: 1000,
        hp: 2000,
        prev_hp: VM_MAX_RAM - 1,
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

#[rstest]
#[case(
    hex::decode(
        "\
        18b18acfb4c2c30276db5411368e7185b311dd124691610c5d3b74034e093dc9\
        063c909c4720840cb5134cb9f59fa749755796819658d32efc0d288198f37266\
        07c2b7f58a84bd6145f00c9c2bc0bb1a187f20ff2c92963a88019e7c6a014eed\
        06614e20c147e940f2d70da3f74c9a17df361706a4485c742bd6788478fa17d7",
    ).unwrap(),
    hex::decode(
        "\
        2243525c5efd4b9c3d3c45ac0ca3fe4dd85e830a4ce6b65fa1eeaee202839703\
        301d1d33be6da8e509df21cc35964723180eed7532537db9ae5e7d48f195c915",
    ).unwrap()
)]
#[case(
    hex::decode(
        "\
        0000000000000000000000000000000000000000000000000000000000000000\
        0000000000000000000000000000000000000000000000000000000000000000\
        0000000000000000000000000000000000000000000000000000000000000000\
        0000000000000000000000000000000000000000000000000000000000000000",
    ).unwrap(),
    hex::decode(
        "\
        0000000000000000000000000000000000000000000000000000000000000000\
        0000000000000000000000000000000000000000000000000000000000000000",
    )
    .unwrap()
)]
fn test_eadd(#[case] input: Vec<u8>, #[case] expected: Vec<u8>) -> SimpleResult<()> {
    // Given
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let owner = OwnershipRegisters {
        sp: 1000,
        ssp: 1000,
        hp: 2000,
        prev_hp: VM_MAX_RAM - 1,
    };
    let mut pc = 4;
    let points_address = 0;
    let result = 2100;
    // P1(x,y),P2(x,y)
    memory[points_address..points_address + 128].copy_from_slice(&input);

    // When
    ec_add(
        &mut memory,
        owner,
        RegMut::new(&mut pc),
        result as Word,
        0,
        points_address as Word,
        (points_address + 64) as Word,
    )?;

    // Then
    assert_eq!(pc, 8);
    assert_eq!(&memory[result as usize..result as usize + 64], &expected);
    Ok(())
}

#[test]
fn test_eadd_error() -> SimpleResult<()> {
    // Given
    let input = hex::decode(
        "\
        1111111111111111111111111111111111111111111111111111111111111111\
        1111111111111111111111111111111111111111111111111111111111111111\
        1111111111111111111111111111111111111111111111111111111111111111\
        1111111111111111111111111111111111111111111111111111111111111111",
    )
    .unwrap();
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let owner = OwnershipRegisters {
        sp: 1000,
        ssp: 1000,
        hp: 2000,
        prev_hp: VM_MAX_RAM - 1,
    };
    let mut pc = 4;
    let points_address = 0;
    let result = 2100;
    // P1(x,y),P2(x,y)
    memory[points_address..points_address + 128].copy_from_slice(&input);

    // When
    let err = ec_add(
        &mut memory,
        owner,
        RegMut::new(&mut pc),
        result as Word,
        0,
        points_address as Word,
        (points_address + 64) as Word,
    )
    .unwrap_err();

    // Then
    assert_eq!(
        err,
        crate::error::PanicOrBug::Panic(fuel_tx::PanicReason::InvalidAltBn128Point)
    );
    Ok(())
}

#[rstest]
#[case(
    hex::decode(
        "\
        2bd3e6d0f3b142924f5ca7b49ce5b9d54c4703d7ae5648e61d02268b1a0a9fb7\
        21611ce0a6af85915e2f1d70300909ce2e49dfad4a4619c8390cae66cefdb204\
        00000000000000000000000000000000000000000000000011138ce750fa15c2",
    )
    .unwrap(),
    hex::decode(
        "\
        070a8d6a982153cae4be29d434e8faef8a47b274a053f5a4ee2a6c9c13c31e5c\
        031b8ce914eba3a9ffb989f9cdd5b0f01943074bf4f0f315690ec3cec6981afc",
    ).unwrap()
)]
#[case(
    hex::decode(
        "\
        0000000000000000000000000000000000000000000000000000000000000000\
        0000000000000000000000000000000000000000000000000000000000000000\
        0200000000000000000000000000000000000000000000000000000000000000"
    ).unwrap(),
    hex::decode(
        "\
        0000000000000000000000000000000000000000000000000000000000000000\
        0000000000000000000000000000000000000000000000000000000000000000",
    )
    .unwrap()
)]
fn test_emul(#[case] input: Vec<u8>, #[case] expected: Vec<u8>) -> SimpleResult<()> {
    // Given
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let owner = OwnershipRegisters {
        sp: 1000,
        ssp: 1000,
        hp: 2000,
        prev_hp: VM_MAX_RAM - 1,
    };
    let mut pc = 4;
    let points_address = 0;
    let result = 2100;

    // P1(x,y),scalar
    memory[points_address..points_address + 96].copy_from_slice(&input);

    // When
    ec_mul(
        &mut memory,
        owner,
        RegMut::new(&mut pc),
        result as Word,
        0,
        points_address as Word,
        (points_address + 64) as Word,
    )?;

    // Then
    assert_eq!(pc, 8);
    assert_eq!(&memory[result as usize..result as usize + 64], &expected);
    Ok(())
}

#[test]
fn test_emul_error() -> SimpleResult<()> {
    // Given
    let input = hex::decode(
        "\
        1111111111111111111111111111111111111111111111111111111111111111\
        1111111111111111111111111111111111111111111111111111111111111111\
        0f00000000000000000000000000000000000000000000000000000000000000",
    )
    .unwrap();
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let owner = OwnershipRegisters {
        sp: 1000,
        ssp: 1000,
        hp: 2000,
        prev_hp: VM_MAX_RAM - 1,
    };
    let mut pc = 4;
    let points_address = 0;
    let result = 2100;
    // P1(x,y),scalar
    memory[points_address..points_address + 96].copy_from_slice(&input);

    // When
    let err = ec_mul(
        &mut memory,
        owner,
        RegMut::new(&mut pc),
        result as Word,
        0,
        points_address as Word,
        (points_address + 64) as Word,
    )
    .unwrap_err();

    // Then
    assert_eq!(
        err,
        crate::error::PanicOrBug::Panic(fuel_tx::PanicReason::InvalidAltBn128Point)
    );
    Ok(())
}

#[rstest]
#[case(
    hex::decode(
        "\
        1c76476f4def4bb94541d57ebba1193381ffa7aa76ada664dd31c16024c43f59\
        3034dd2920f673e204fee2811c678745fc819b55d3e9d294e45c9b03a76aef41\
        209dd15ebff5d46c4bd888e51a93cf99a7329636c63514396b4a452003a35bf7\
        04bf11ca01483bfa8b34b43561848d28905960114c8ac04049af4b6315a41678\
        2bb8324af6cfc93537a2ad1a445cfd0ca2a71acd7ac41fadbf933c2a51be344d\
        120a2a4cf30c1bf9845f20c6fe39e07ea2cce61f0c9bb048165fe5e4de877550\
        111e129f1cf1097710d41c4ac70fcdfa5ba2023c6ff1cbeac322de49d1b6df7c\
        2032c61a830e3c17286de9462bf242fca2883585b93870a73853face6a6bf411\
        198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c2\
        1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed\
        090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b\
        12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa",
    ).unwrap(),
    hex::decode(
        "0000000000000000000000000000000000000000000000000000000000000001"
    ).unwrap()
)]
fn test_epar(#[case] input: Vec<u8>, #[case] expected: Vec<u8>) -> SimpleResult<()> {
    // Given
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let owner = OwnershipRegisters {
        sp: 1000,
        ssp: 1000,
        hp: 2000,
        prev_hp: VM_MAX_RAM - 1,
    };
    let mut pc = 4;
    let points_address = 0;
    let result = 2100;

    // P1(x,y),G2(p1(x,y), p2(x,y))
    memory[points_address..points_address + 384].copy_from_slice(&input);

    // When
    ec_pairing(
        &mut memory,
        owner,
        RegMut::new(&mut pc),
        result as Word,
        0,
        2,
        points_address as Word,
    )?;

    // Then
    assert_eq!(pc, 8);
    assert_eq!(&memory[result as usize..result as usize + 32], &expected);
    Ok(())
}

#[test]
fn test_epar_error() -> SimpleResult<()> {
    // Given
    let input = hex::decode(
        "\
        1111111111111111111111111111111111111111111111111111111111111111\
        1111111111111111111111111111111111111111111111111111111111111111\
        1111111111111111111111111111111111111111111111111111111111111111\
        1111111111111111111111111111111111111111111111111111111111111111\
        1111111111111111111111111111111111111111111111111111111111111111\
        1111111111111111111111111111111111111111111111111111111111111111",
    )
    .unwrap();
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let owner = OwnershipRegisters {
        sp: 1000,
        ssp: 1000,
        hp: 2000,
        prev_hp: VM_MAX_RAM - 1,
    };
    let mut pc = 4;
    let points_address = 0;
    let result = 2100;
    // P1(x,y),G2(p1(x,y), p2(x,y))
    memory[points_address..points_address + 192].copy_from_slice(&input);

    // When
    let err = ec_pairing(
        &mut memory,
        owner,
        RegMut::new(&mut pc),
        result as Word,
        0,
        2,
        points_address as Word,
    )
    .unwrap_err();

    // Then
    assert_eq!(
        err,
        crate::error::PanicOrBug::Panic(fuel_tx::PanicReason::InvalidAltBn128Point)
    );
    Ok(())
}
