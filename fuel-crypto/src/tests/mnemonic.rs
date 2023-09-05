use core::str::FromStr;

use crate::SecretKey;

use coins_bip32::path::DerivationPath;
use coins_bip39::{
    English,
    Mnemonic,
};

type W = English;

#[test]
fn secret_key_from_mnemonic_phrase() {
    let phrase =
        "oblige salon price punch saddle immune slogan rare snap desert retire surprise";

    let expected_public_key = "30cc18506ed9d500fa348d1202bac14e9683b6d4cd7a02eb5357504d74ff2a19a8b672eb22c6509588424bab5c627515a9105b7ad25b7f948fcb5cd09448df5e";

    let secret =
        SecretKey::new_from_mnemonic_phrase_with_path(phrase, "m/44'/60'/0'/0/0")
            .expect("failed to create secret key from mnemonic phrase");

    let public = secret.public_key();

    assert_eq!(public.to_string(), expected_public_key);
}

#[test]
fn secret_key_from_mnemonic() {
    let phrase =
        "oblige salon price punch saddle immune slogan rare snap desert retire surprise";
    let expected_public_key = "30cc18506ed9d500fa348d1202bac14e9683b6d4cd7a02eb5357504d74ff2a19a8b672eb22c6509588424bab5c627515a9105b7ad25b7f948fcb5cd09448df5e";

    let m = Mnemonic::<W>::new_from_phrase(phrase).expect("failed to create mnemonic");

    let d = DerivationPath::from_str("m/44'/60'/0'/0/0")
        .expect("failed to create derivation path");
    let secret = SecretKey::new_from_mnemonic(d, m)
        .expect("failed to create secret key from mnemonic");

    let public = secret.public_key();
    let public_key = public.to_string();

    assert_eq!(public_key, expected_public_key);
}

#[test]
fn random_mnemonic_phrase() {
    // create rng
    let mut rng = rand::thread_rng();

    let phrase = crate::generate_mnemonic_phrase(&mut rng, 12)
        .expect("failed to generate mnemonic phrase");

    let _secret =
        SecretKey::new_from_mnemonic_phrase_with_path(&phrase, "m/44'/60'/0'/0/0")
            .expect("failed to create secret key from mnemonic phrase");
}
