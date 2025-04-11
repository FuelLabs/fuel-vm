#![allow(clippy::arithmetic_side_effects)]

use fuel_types::*;
use rand::{
    rngs::StdRng,
    Rng,
    RngCore,
    SeedableRng,
};

use core::{
    fmt,
    str,
};

macro_rules! check_consistency {
    ($i:ident, $r:expr, $b:expr) => {
        let n = $i::LEN;
        let s = $r.gen_range(0..$b.len() - n);
        let e = $r.gen_range(s + n..$b.len());
        let r = $r.gen_range(1..n - 1);
        let i = &$b[s..s + n];

        let a = $i::from_bytes_ref(i.try_into().unwrap());
        let b = $i::from_bytes_ref_checked(&$b[s..e]).unwrap();
        let c = $i::try_from(i).expect("Memory conversion");

        assert!($i::from_bytes_ref_checked(&i[..i.len() - r]).is_none());

        let e = $i::from_bytes_ref_checked(i).unwrap();

        assert_eq!(e.as_ptr(), i.as_ptr());

        assert_eq!(a, b);
        assert_eq!(*a, c);
        assert_eq!(a, e);
        assert_eq!(a.len(), $i::LEN);
        assert_eq!(b.len(), $i::LEN);
        assert_eq!(c.len(), $i::LEN);
        assert_eq!(e.len(), $i::LEN);
    };
}

#[test]
fn from_slice_unchecked_safety() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let mut bytes = [0u8; 257];
    rng.fill_bytes(&mut bytes);

    for _ in 0..100 {
        check_consistency!(Address, rng, bytes);
        check_consistency!(AssetId, rng, bytes);
        check_consistency!(ContractId, rng, bytes);
        check_consistency!(TxId, rng, bytes);
        check_consistency!(Bytes4, rng, bytes);
        check_consistency!(Bytes8, rng, bytes);
        check_consistency!(Bytes20, rng, bytes);
        check_consistency!(Bytes32, rng, bytes);
        check_consistency!(Nonce, rng, bytes);
        check_consistency!(MessageId, rng, bytes);
        check_consistency!(Salt, rng, bytes);
        check_consistency!(Bytes64, rng, bytes);
    }
}

#[test]
fn hex_encoding() {
    fn encode_decode<T>(t: T)
    where
        T: fmt::LowerHex + fmt::UpperHex + str::FromStr + Eq + fmt::Debug + AsRef<[u8]>,
        <T as str::FromStr>::Err: fmt::Debug,
    {
        let lower = format!("{t:x}");
        let lower_w0 = format!("{t:0x}");
        let lower_alternate = format!("{t:#x}");
        let lower_alternate_w0 = format!("{t:#0x}");
        let upper = format!("{t:X}");
        let upper_w0 = format!("{t:0X}");
        let upper_alternate = format!("{t:#X}");
        let upper_alternate_w0 = format!("{t:#X}");

        assert_ne!(lower, lower_alternate);
        assert_ne!(lower, upper);
        assert_ne!(lower, upper_alternate);
        assert_ne!(lower_alternate, upper);
        assert_ne!(lower_alternate, upper_alternate);
        assert_ne!(upper, upper_alternate);

        assert_eq!(lower, lower_w0);
        assert_eq!(lower_alternate, lower_alternate_w0);
        assert_eq!(upper, upper_w0);
        assert_eq!(upper_alternate, upper_alternate_w0);

        let lower = T::from_str(lower.as_str()).expect("Failed to parse lower");
        let lower_alternate = T::from_str(lower_alternate.as_str())
            .expect("Failed to parse lower alternate");
        let upper = T::from_str(upper.as_str()).expect("Failed to parse upper");
        let upper_alternate = T::from_str(upper_alternate.as_str())
            .expect("Failed to parse upper alternate");

        assert_eq!(t, lower);
        assert_eq!(t, lower_alternate);
        assert_eq!(t, upper);
        assert_eq!(t, upper_alternate);

        let reduced = t.as_ref().iter().fold(0u8, |acc, x| acc ^ x);

        let x = hex::encode([reduced]);
        let y = format!("{t:2x}");

        assert_eq!(x, y);

        let x = format!("{t:0x}").len();
        let y = format!("{t:2x}").len();
        let z = format!("{t:4x}").len();

        assert_eq!(t.as_ref().len() * 2, x);
        assert_eq!(2, y);
        assert_eq!(4, z);
    }

    let rng = &mut StdRng::seed_from_u64(8586);

    encode_decode::<Address>(rng.r#gen());
    encode_decode::<AssetId>(rng.r#gen());
    encode_decode::<ContractId>(rng.r#gen());
    encode_decode::<TxId>(rng.r#gen());
    encode_decode::<Bytes4>(rng.r#gen());
    encode_decode::<Bytes8>(rng.r#gen());
    encode_decode::<Bytes20>(rng.r#gen());
    encode_decode::<Bytes32>(rng.r#gen());
    encode_decode::<Nonce>(rng.r#gen());
    encode_decode::<MessageId>(rng.r#gen());
    encode_decode::<Salt>(rng.r#gen());
    encode_decode::<Bytes64>(rng.r#gen());
}

#[test]
#[cfg(feature = "serde")]
fn test_key_serde() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let adr: Address = rng.r#gen();
    let ast_id: AssetId = rng.r#gen();
    let contract_id: ContractId = rng.r#gen();
    let tx_id: TxId = rng.r#gen();
    let bytes4: Bytes4 = rng.r#gen();
    let bytes8: Bytes8 = rng.r#gen();
    let bytes20: Bytes20 = rng.r#gen();
    let bytes32: Bytes32 = rng.r#gen();
    let nonce: Nonce = rng.r#gen();
    let message_id: MessageId = rng.r#gen();
    let salt: Salt = rng.r#gen();
    let bytes64: Bytes64 = rng.r#gen();

    let adr_t = bincode::serialize(&adr).expect("Failed to serialize Address");
    let adr_t: Address =
        bincode::deserialize(&adr_t).expect("Failed to deserialize Address");
    assert_eq!(adr, adr_t);

    let ast_id_t = bincode::serialize(&ast_id).expect("Failed to serialize AssetId");
    let ast_id_t: AssetId =
        bincode::deserialize(&ast_id_t).expect("Failed to deserialize AssetId");
    assert_eq!(ast_id, ast_id_t);

    let contract_id_t =
        bincode::serialize(&contract_id).expect("Failed to serialize ContractId");
    let contract_id_t: ContractId =
        bincode::deserialize(&contract_id_t).expect("Failed to deserialize ContractId");
    assert_eq!(contract_id, contract_id_t);

    let tx_id_t = bincode::serialize(&tx_id).expect("Failed to serialize TxId");
    let tx_id_t: TxId =
        bincode::deserialize(&tx_id_t).expect("Failed to deserialize TxId");
    assert_eq!(tx_id, tx_id_t);

    let bytes4_t = bincode::serialize(&bytes4).expect("Failed to serialize Bytes4");
    let bytes4_t: Bytes4 =
        bincode::deserialize(&bytes4_t).expect("Failed to deserialize Bytes4");
    assert_eq!(bytes4, bytes4_t);

    let bytes8_t = bincode::serialize(&bytes8).expect("Failed to serialize Bytes8");
    let bytes8_t: Bytes8 =
        bincode::deserialize(&bytes8_t).expect("Failed to deserialize Bytes8");
    assert_eq!(bytes8, bytes8_t);

    let bytes20_t = bincode::serialize(&bytes20).expect("Failed to serialize Bytes20");
    let bytes20_t: Bytes20 =
        bincode::deserialize(&bytes20_t).expect("Failed to deserialize Bytes20");
    assert_eq!(bytes20, bytes20_t);

    let bytes32_t = bincode::serialize(&bytes32).expect("Failed to serialize Bytes32");
    let bytes32_t: Bytes32 =
        bincode::deserialize(&bytes32_t).expect("Failed to deserialize Bytes32");
    assert_eq!(bytes32, bytes32_t);

    let nonce_t = bincode::serialize(&nonce).expect("Failed to serialize Nonce");
    let nonce_t: Nonce =
        bincode::deserialize(&nonce_t).expect("Failed to deserialize Nonce");
    assert_eq!(nonce, nonce_t);

    let message_id_t =
        bincode::serialize(&message_id).expect("Failed to serialize MessageId");
    let message_id_t: MessageId =
        bincode::deserialize(&message_id_t).expect("Failed to deserialize MessageId");
    assert_eq!(message_id, message_id_t);

    let salt_t = bincode::serialize(&salt).expect("Failed to serialize Salt");
    let salt_t: Salt = bincode::deserialize(&salt_t).expect("Failed to deserialize Salt");
    assert_eq!(salt, salt_t);

    let bytes64_t = bincode::serialize(&bytes64).expect("Failed to serialize Bytes64");
    let bytes64_t: Bytes64 =
        bincode::deserialize(&bytes64_t).expect("Failed to deserialize Bytes64");
    assert_eq!(bytes64, bytes64_t);
}

#[test]
#[cfg(feature = "serde")]
fn test_key_types_hex_serialization() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let adr: Address = rng.r#gen();
    let adr_to_string =
        serde_json::to_string(&adr).expect("serde_json::to_string failed on Address");
    assert_eq!(format!("\"{adr}\""), adr_to_string);

    let ast_id: AssetId = rng.r#gen();
    let ast_id_to_string =
        serde_json::to_string(&ast_id).expect("serde_json::to_string failed on AssetId");
    assert_eq!(format!("\"{ast_id}\""), ast_id_to_string);

    let contract_id: ContractId = rng.r#gen();
    let contract_id_to_string = serde_json::to_string(&contract_id)
        .expect("serde_json::to_string failed on ContractId");
    assert_eq!(format!("\"{contract_id}\""), contract_id_to_string);

    let tx_id: TxId = rng.r#gen();
    let tx_id_to_string =
        serde_json::to_string(&tx_id).expect("serde_json::to_string failed on TxId");
    assert_eq!(format!("\"{tx_id}\""), tx_id_to_string);

    let bytes4: Bytes4 = rng.r#gen();
    let bytes4_to_string =
        serde_json::to_string(&bytes4).expect("serde_json::to_string failed on Bytes4");
    assert_eq!(format!("\"{bytes4}\""), bytes4_to_string);

    let bytes8: Bytes8 = rng.r#gen();
    let bytes8_to_string =
        serde_json::to_string(&bytes8).expect("serde_json::to_string failed on Bytes8");
    assert_eq!(format!("\"{bytes8}\""), bytes8_to_string);

    let bytes20: Bytes20 = rng.r#gen();
    let bytes20_to_string =
        serde_json::to_string(&bytes20).expect("serde_json::to_string failed on Bytes20");
    assert_eq!(format!("\"{bytes20}\""), bytes20_to_string);

    let bytes32: Bytes32 = rng.r#gen();
    let bytes32_to_string =
        serde_json::to_string(&bytes32).expect("serde_json::to_string failed on Bytes32");
    assert_eq!(format!("\"{bytes32}\""), bytes32_to_string);

    let message_id: MessageId = rng.r#gen();
    let message_id_to_string = serde_json::to_string(&message_id)
        .expect("serde_json::to_string failed on MessageId");
    assert_eq!(format!("\"{message_id}\""), message_id_to_string);

    let salt: Salt = rng.r#gen();
    let salt_to_string =
        serde_json::to_string(&salt).expect("serde_json::to_string failed on Salt");
    assert_eq!(format!("\"{salt}\""), salt_to_string);

    let bytes64: Bytes64 = rng.r#gen();
    let bytes64_to_string =
        serde_json::to_string(&bytes64).expect("Failed to serialize Bytes64");
    assert_eq!(format!("\"{bytes64}\""), bytes64_to_string);
}
