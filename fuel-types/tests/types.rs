use fuel_types::*;
use rand::rngs::StdRng;
use rand::{Rng, RngCore, SeedableRng};

use core::{fmt, str};

macro_rules! check_consistency {
    ($i:ident,$r:expr,$b:expr) => {
        unsafe {
            let n = $i::LEN;
            let s = $r.gen_range(0..$b.len() - n);
            let e = $r.gen_range(s + n..$b.len());
            let r = $r.gen_range(1..n - 1);
            let i = &$b[s..s + n];

            let a = $i::from_slice_unchecked(i);
            let b = $i::from_slice_unchecked(&$b[s..e]);
            let c = $i::try_from(i).expect("Memory conversion");

            // `d` will create random smaller slices and expect the value to be parsed correctly
            //
            // However, this is not the expected usage of the function
            let d = $i::from_slice_unchecked(&i[..i.len() - r]);

            let e = $i::as_ref_unchecked(i);

            // Assert `from_slice_unchecked` will not create two references to the same owned
            // memory
            assert_ne!(a.as_ptr(), b.as_ptr());

            // Assert `as_ref_unchecked` is copy-free
            assert_ne!(e.as_ptr(), a.as_ptr());
            assert_eq!(e.as_ptr(), i.as_ptr());

            assert_eq!(a, b);
            assert_eq!(a, c);
            assert_eq!(a, d);
            assert_eq!(&a, e);
        }
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
        check_consistency!(Bytes4, rng, bytes);
        check_consistency!(Bytes8, rng, bytes);
        check_consistency!(Bytes32, rng, bytes);
        check_consistency!(Bytes64, rng, bytes);
        check_consistency!(Salt, rng, bytes);
    }
}

#[test]
fn hex_encoding() {
    fn encode_decode<T>(t: T)
    where
        T: fmt::LowerHex + fmt::UpperHex + str::FromStr + Eq + fmt::Debug,
        <T as str::FromStr>::Err: fmt::Debug,
    {
        let lower = format!("{:x}", t);
        let lower_alternate = format!("{:#x}", t);
        let upper = format!("{:X}", t);
        let upper_alternate = format!("{:#X}", t);

        assert_ne!(lower, lower_alternate);
        assert_ne!(lower, upper);
        assert_ne!(lower, upper_alternate);
        assert_ne!(lower_alternate, upper);
        assert_ne!(lower_alternate, upper_alternate);
        assert_ne!(upper, upper_alternate);

        let lower = T::from_str(lower.as_str()).expect("Failed to parse lower");
        let lower_alternate =
            T::from_str(lower_alternate.as_str()).expect("Failed to parse lower alternate");
        let upper = T::from_str(upper.as_str()).expect("Failed to parse upper");
        let upper_alternate =
            T::from_str(upper_alternate.as_str()).expect("Failed to parse upper alternate");

        assert_eq!(t, lower);
        assert_eq!(t, lower_alternate);
        assert_eq!(t, upper);
        assert_eq!(t, upper_alternate);
    }

    let rng = &mut StdRng::seed_from_u64(8586);

    encode_decode(rng.gen::<Address>());
    encode_decode(rng.gen::<AssetId>());
    encode_decode(rng.gen::<ContractId>());
    encode_decode(rng.gen::<Bytes4>());
    encode_decode(rng.gen::<Bytes8>());
    encode_decode(rng.gen::<Bytes32>());
    encode_decode(rng.gen::<Bytes64>());
    encode_decode(rng.gen::<Salt>());
}

#[test]
#[cfg(feature = "serde-types-minimal")]
fn test_key_with_big_array() {
    let rng = &mut StdRng::seed_from_u64(8586);
    let s: Bytes64 = rng.gen();
    let j = serde_json::to_string(&s).unwrap();
    let s_back = serde_json::from_str(&j).unwrap();
    assert!(&s == &s_back);
}
