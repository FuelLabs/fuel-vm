#![allow(clippy::cast_possible_truncation)]
use alloc::vec;

use fuel_crypto::SecretKey;
use rand::{
    rngs::StdRng,
    RngCore,
    SeedableRng,
};
use rstest::rstest;

use crate::consts::VM_MAX_RAM;

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
// From https://github.com/bluealloy/revm/blob/main/crates/precompile/src/bn128.rs
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
// From https://github.com/bluealloy/revm/blob/main/crates/precompile/src/bn128.rs
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
// From https://github.com/ethereum/tests/blob/develop/GeneralStateTests/stZeroKnowledge2/ecadd_1145-3932_2969-1336_21000_128.json
#[case(
    hex::decode(
        "\
        17c139df0efee0f766bc0204762b774362e4ded88953a39ce849a8a7fa163fa9\
        01e0559bacb160664764a357af8a9fe70baa9258e0b959273ffc5718c6d4cc7c\
        039730ea8dff1254c0fee9c0ea777d29a9c710b7e616683f194f18c43b43b869\
        073a5ffcc6fc7a28c30723d6e58ce577356982d65b833a5a5c15bf9024b43d98",
    ).unwrap(),
    hex::decode(
        "\
        15bf2bb17880144b5d1cd2b1f46eff9d617bffd1ca57c37fb5a49bd84e53cf66\
        049c797f9ce0d17083deb32b5e36f2ea2a212ee036598dd7624c168993d1355f",
    ).unwrap()
)]
// From https://github.com/matter-labs/era-compiler-tests/blob/2253941334797eb2a997941845fb9eb0d436558b/yul/precompiles/ecadd.yul#L123
#[case(
    hex::decode(
        "\
        17c139df0efee0f766bc0204762b774362e4ded88953a39ce849a8a7fa163fa9\
        01e0559bacb160664764a357af8a9fe70baa9258e0b959273ffc5718c6d4cc7c\
        17c139df0efee0f766bc0204762b774362e4ded88953a39ce849a8a7fa163fa9\
        2e83f8d734803fc370eba25ed1f6b8768bd6d83887b87165fc2434fe11a830cb",
    ).unwrap(),
    hex::decode(
        "\
        0000000000000000000000000000000000000000000000000000000000000000\
        0000000000000000000000000000000000000000000000000000000000000000",
    ).unwrap()
)]
// From https://github.com/poanetwork/parity-ethereum/blob/2ea4265b0083c4148571b21e1079c641d5f31dc2/ethcore/benches/builtin.rs#L486
#[case(
    hex::decode(
        "\
        0000000000000000000000000000000000000000000000000000000000000001\
        0000000000000000000000000000000000000000000000000000000000000002\
        0000000000000000000000000000000000000000000000000000000000000001\
        0000000000000000000000000000000000000000000000000000000000000002",
    ).unwrap(),
    hex::decode(
        "\
        030644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd3\
        15ed738c0e0a7c92e7845f96b2ae9c0a68a6a449e3538fc7ff3ebf7a5a18a2c4",
    ).unwrap()
)]
fn test_ecop_addition(
    #[case] input: Vec<u8>,
    #[case] expected: Vec<u8>,
) -> SimpleResult<()> {
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
    let result = 2100u64;
    // P1(x,y),P2(x,y)
    memory[points_address..points_address + 128].copy_from_slice(&input);

    // When
    ec_operation(
        &mut memory,
        owner,
        RegMut::new(&mut pc),
        result as Word,
        0,
        0,
        points_address as Word,
    )?;

    // Then
    assert_eq!(pc, 8);
    assert_eq!(
        &memory[result as usize..result.checked_add(64).unwrap() as usize],
        &expected
    );
    Ok(())
}

// From https://github.com/bluealloy/revm/blob/main/crates/precompile/src/bn128.rs
#[test]
fn test_ecop_addition_error() -> SimpleResult<()> {
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
    let result = 2100u64;
    // P1(x,y),P2(x,y)
    memory[points_address..points_address + 128].copy_from_slice(&input);

    // When
    let err = ec_operation(
        &mut memory,
        owner,
        RegMut::new(&mut pc),
        result as Word,
        0,
        0,
        points_address as Word,
    )
    .unwrap_err();

    // Then
    assert_eq!(
        err,
        crate::error::PanicOrBug::Panic(fuel_tx::PanicReason::InvalidEllipticCurvePoint)
    );
    Ok(())
}

#[rstest]
// From https://github.com/bluealloy/revm/blob/main/crates/precompile/src/bn128.rs
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
// From https://github.com/bluealloy/revm/blob/main/crates/precompile/src/bn128.rs
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
// From https://github.com/ethereum/tests/blob/develop/GeneralStateTests/stZeroKnowledge/ecmul_7827-6598_1456_21000_96.json
#[case(
    hex::decode(
        "\
        1a87b0584ce92f4593d161480614f2989035225609f08058ccfa3d0f940febe3\
        1a2f3c951f6dadcc7ee9007dff81504b0fcd6d7cf59996efdc33d92bf7f9f8f6\
        0000000000000000000000000000000100000000000000000000000000000000",
    ).unwrap(),
    hex::decode(
        "\
        1051acb0700ec6d42a88215852d582efbaef31529b6fcbc3277b5c1b300f5cf0\
        135b2394bb45ab04b8bd7611bd2dfe1de6a4e6e2ccea1ea1955f577cd66af85b",
    ).unwrap()
)]
// From https://github.com/matter-labs/era-compiler-tests/blob/2253941334797eb2a997941845fb9eb0d436558b/yul/precompiles/ecmul.yul#L185C21-L185C98
#[case(
    hex::decode(
        "\
        1a87b0584ce92f4593d161480614f2989035225609f08058ccfa3d0f940febe3\
        1a2f3c951f6dadcc7ee9007dff81504b0fcd6d7cf59996efdc33d92bf7f9f8f6\
        30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001"
    ).unwrap(),
    hex::decode(
        "\
        0000000000000000000000000000000000000000000000000000000000000000\
        0000000000000000000000000000000000000000000000000000000000000000",
    ).unwrap()
)]
// From https://github.com/poanetwork/parity-ethereum/blob/2ea4265b0083c4148571b21e1079c641d5f31dc2/ethcore/benches/builtin.rs#L516
#[case(
    hex::decode(
        "\
        2bd3e6d0f3b142924f5ca7b49ce5b9d54c4703d7ae5648e61d02268b1a0a9fb7\
        21611ce0a6af85915e2f1d70300909ce2e49dfad4a4619c8390cae66cefdb204\
        00000000000000000000000000000000000000000000000011138ce750fa15c2"
    ).unwrap(),
    hex::decode(
        "\
        070a8d6a982153cae4be29d434e8faef8a47b274a053f5a4ee2a6c9c13c31e5c\
        031b8ce914eba3a9ffb989f9cdd5b0f01943074bf4f0f315690ec3cec6981afc"
    ).unwrap()
)]
fn test_ecop_multiplication(
    #[case] input: Vec<u8>,
    #[case] expected: Vec<u8>,
) -> SimpleResult<()> {
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
    let result = 2100u64;

    // P1(x,y),scalar
    memory[points_address..points_address + 96].copy_from_slice(&input);

    // When
    ec_operation(
        &mut memory,
        owner,
        RegMut::new(&mut pc),
        result as Word,
        0,
        1,
        points_address as Word,
    )?;

    // Then
    assert_eq!(pc, 8);
    assert_eq!(
        &memory[result as usize..result.checked_add(64).unwrap() as usize],
        &expected
    );
    Ok(())
}

// From https://github.com/bluealloy/revm/blob/main/crates/precompile/src/bn128.rs
#[test]
fn test_ecop_multiplication_error() -> SimpleResult<()> {
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
    let result = 2100u64;
    // P1(x,y),scalar
    memory[points_address..points_address + 96].copy_from_slice(&input);

    // When
    let err = ec_operation(
        &mut memory,
        owner,
        RegMut::new(&mut pc),
        result as Word,
        0,
        1,
        points_address as Word,
    )
    .unwrap_err();

    // Then
    assert_eq!(
        err,
        crate::error::PanicOrBug::Panic(fuel_tx::PanicReason::InvalidEllipticCurvePoint)
    );
    Ok(())
}

#[test]
fn test_ecop_read_memory_not_accessible() -> SimpleResult<()> {
    // Given
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let owner = OwnershipRegisters {
        sp: 1000,
        ssp: 1000,
        hp: 2000,
        prev_hp: VM_MAX_RAM - 1,
    };
    let mut pc = 4;
    let points_address = VM_MAX_RAM;
    let result = 2100u64;

    // When
    let err = ec_operation(
        &mut memory,
        owner,
        RegMut::new(&mut pc),
        result as Word,
        0,
        1,
        points_address as Word,
    )
    .unwrap_err();

    // Then
    assert_eq!(
        err,
        crate::error::PanicOrBug::Panic(fuel_tx::PanicReason::MemoryOverflow)
    );
    Ok(())
}

#[test]
fn test_ecop_write_memory_not_accessible() -> SimpleResult<()> {
    // Given
    let input = hex::decode(
        "\
        2bd3e6d0f3b142924f5ca7b49ce5b9d54c4703d7ae5648e61d02268b1a0a9fb7\
        21611ce0a6af85915e2f1d70300909ce2e49dfad4a4619c8390cae66cefdb204\
        00000000000000000000000000000000000000000000000011138ce750fa15c2",
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
    let result = 0u64;
    // P1(x,y),scalar
    memory[points_address..points_address + 96].copy_from_slice(&input);

    // When
    let err = ec_operation(
        &mut memory,
        owner,
        RegMut::new(&mut pc),
        result as Word,
        0,
        1,
        points_address as Word,
    )
    .unwrap_err();

    // Then
    assert_eq!(
        err,
        crate::error::PanicOrBug::Panic(fuel_tx::PanicReason::MemoryOwnership)
    );
    Ok(())
}

#[rstest]
// From https://github.com/bluealloy/revm/blob/main/crates/precompile/src/bn128.rs
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
    1u64
)]
// From https://github.com/ethereum/tests/blob/develop/GeneralStateTests/stZeroKnowledge/ecpairing_three_point_match_1.json
#[case(
    hex::decode(
        "\
        105456a333e6d636854f987ea7bb713dfd0ae8371a72aea313ae0c32c0bf1016\
        0cf031d41b41557f3e7e3ba0c51bebe5da8e6ecd855ec50fc87efcdeac168bcc\
        0476be093a6d2b4bbf907172049874af11e1b6267606e00804d3ff0037ec57fd\
        3010c68cb50161b7d1d96bb71edfec9880171954e56871abf3d93cc94d745fa1\
        14c059d74e5b6c4ec14ae5864ebe23a71781d86c29fb8fb6cce94f70d3de7a21\
        01b33461f39d9e887dbb100f170a2345dde3c07e256d1dfa2b657ba5cd030427\
        0000000000000000000000000000000000000000000000000000000000000001\
        0000000000000000000000000000000000000000000000000000000000000002\
        1a2c3013d2ea92e13c800cde68ef56a294b883f6ac35d25f587c09b1b3c635f7\
        290158a80cd3d66530f74dc94c94adb88f5cdb481acca997b6e60071f08a115f\
        2f997f3dbd66a7afe07fe7862ce239edba9e05c5afff7f8a1259c9733b2dfbb9\
        29d1691530ca701b4a106054688728c9972c8512e9789e9567aae23e302ccd75"
    ).unwrap(),
    1u64
)]
// From https://github.com/ethereum/tests/blob/develop/GeneralStateTests/stZeroKnowledge/ecpairing_three_point_fail_1.json
#[case(
    hex::decode(
        "\
        105456a333e6d636854f987ea7bb713dfd0ae8371a72aea313ae0c32c0bf1016\
        0cf031d41b41557f3e7e3ba0c51bebe5da8e6ecd855ec50fc87efcdeac168bcc\
        0476be093a6d2b4bbf907172049874af11e1b6267606e00804d3ff0037ec57fd\
        3010c68cb50161b7d1d96bb71edfec9880171954e56871abf3d93cc94d745fa1\
        14c059d74e5b6c4ec14ae5864ebe23a71781d86c29fb8fb6cce94f70d3de7a21\
        01b33461f39d9e887dbb100f170a2345dde3c07e256d1dfa2b657ba5cd030427\
        0000000000000000000000000000000000000000000000000000000000000001\
        0000000000000000000000000000000000000000000000000000000000000002\
        1a2c3013d2ea92e13c800cde68ef56a294b883f6ac35d25f587c09b1b3c635f7\
        290158a80cd3d66530f74dc94c94adb88f5cdb481acca997b6e60071f08a115f\
        00cacf3523caf879d7d05e30549f1e6fdce364cbb8724b0329c6c2a39d4f018e\
        0692e55db067300e6e3fe56218fa2f940054e57e7ef92bf7d475a9d8a8502fd2"
    ).unwrap(),
    0u64
)]
// From https://github.com/poanetwork/parity-ethereum/blob/2ea4265b0083c4148571b21e1079c641d5f31dc2/ethcore/benches/builtin.rs#L686
#[case(
    hex::decode(
        "\
        0000000000000000000000000000000000000000000000000000000000000001\
        0000000000000000000000000000000000000000000000000000000000000002\
        198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c2\
        1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed\
        090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b\
        12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa\
        0000000000000000000000000000000000000000000000000000000000000001\
        0000000000000000000000000000000000000000000000000000000000000002\
        198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c2\
        1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed\
        275dc4a288d1afb3cbb1ac09187524c7db36395df7be3b99e673b13a075a65ec\
        1d9befcd05a5323e6da4d435f3b617cdb3af83285c2df711ef39c01571827f9d\
        0000000000000000000000000000000000000000000000000000000000000001\
        0000000000000000000000000000000000000000000000000000000000000002\
        198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c2\
        1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed\
        090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b\
        12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa\
        0000000000000000000000000000000000000000000000000000000000000001\
        0000000000000000000000000000000000000000000000000000000000000002\
        198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c2\
        1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed\
        275dc4a288d1afb3cbb1ac09187524c7db36395df7be3b99e673b13a075a65ec\
        1d9befcd05a5323e6da4d435f3b617cdb3af83285c2df711ef39c01571827f9d\
        0000000000000000000000000000000000000000000000000000000000000001\
        0000000000000000000000000000000000000000000000000000000000000002\
        198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c2\
        1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed\
        090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b\
        12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa\
        0000000000000000000000000000000000000000000000000000000000000001\
        0000000000000000000000000000000000000000000000000000000000000002\
        198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c2\
        1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed\
        275dc4a288d1afb3cbb1ac09187524c7db36395df7be3b99e673b13a075a65ec\
        1d9befcd05a5323e6da4d435f3b617cdb3af83285c2df711ef39c01571827f9d\
        0000000000000000000000000000000000000000000000000000000000000001\
        0000000000000000000000000000000000000000000000000000000000000002\
        198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c2\
        1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed\
        090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b\
        12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa\
        0000000000000000000000000000000000000000000000000000000000000001\
        0000000000000000000000000000000000000000000000000000000000000002\
        198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c2\
        1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed\
        275dc4a288d1afb3cbb1ac09187524c7db36395df7be3b99e673b13a075a65ec\
        1d9befcd05a5323e6da4d435f3b617cdb3af83285c2df711ef39c01571827f9d\
        0000000000000000000000000000000000000000000000000000000000000001\
        0000000000000000000000000000000000000000000000000000000000000002\
        198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c2\
        1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed\
        090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b\
        12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa\
        0000000000000000000000000000000000000000000000000000000000000001\
        0000000000000000000000000000000000000000000000000000000000000002\
        198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c2\
        1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed\
        275dc4a288d1afb3cbb1ac09187524c7db36395df7be3b99e673b13a075a65ec\
        1d9befcd05a5323e6da4d435f3b617cdb3af83285c2df711ef39c01571827f9d"
    ).unwrap(),
    1u64
)]
fn test_epar(#[case] input: Vec<u8>, #[case] expected: u64) -> SimpleResult<()> {
    // Given
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let mut pc = 4;
    let points_address: usize = 0;
    let mut result = 0;

    // Length
    let nb_elements = input
        .len()
        .checked_div(128usize.checked_add(64).unwrap())
        .unwrap();
    // P1(x,y),G2(p1(x,y), p2(x,y))
    memory[points_address..points_address.checked_add(input.len()).unwrap()]
        .copy_from_slice(&input);

    // When
    ec_pairing(
        &mut memory,
        RegMut::new(&mut pc),
        &mut result,
        0,
        nb_elements as Word,
        0 as Word,
    )?;

    // Then
    assert_eq!(pc, 8);
    assert_eq!(result, expected);
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
    let mut pc = 4;
    let points_address = 0;
    let mut result = 0;
    // Length
    let nb_elements = input
        .len()
        .checked_div(128usize.checked_add(64).unwrap())
        .unwrap();
    // P1(x,y),G2(p1(x,y), p2(x,y))
    memory[points_address..points_address + 192].copy_from_slice(&input);

    // When
    let err = ec_pairing(
        &mut memory,
        RegMut::new(&mut pc),
        &mut result,
        0,
        nb_elements as Word,
        0 as Word,
    )
    .unwrap_err();

    // Then
    assert_eq!(
        err,
        crate::error::PanicOrBug::Panic(fuel_tx::PanicReason::InvalidEllipticCurvePoint)
    );
    Ok(())
}

#[test]
fn test_epar_read_memory_not_accessible() -> SimpleResult<()> {
    // Given
    let mut memory: MemoryInstance = vec![1u8; MEM_SIZE].try_into().unwrap();
    let mut pc = 4;
    let points_address = VM_MAX_RAM;
    let mut result = 0;
    // Length
    let nb_elements = 1;
    // When
    let err = ec_pairing(
        &mut memory,
        RegMut::new(&mut pc),
        &mut result,
        0,
        nb_elements as Word,
        points_address as Word,
    )
    .unwrap_err();

    // Then
    assert_eq!(
        err,
        crate::error::PanicOrBug::Panic(fuel_tx::PanicReason::MemoryOverflow)
    );
    Ok(())
}
