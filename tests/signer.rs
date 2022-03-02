use fuel_crypto::borrown::Borrown;
use fuel_crypto::{Keystore, Message, SecretKey, Signer};
use rand::rngs::StdRng;
use rand::SeedableRng;

use std::io;

#[derive(Debug, Default, Clone)]
struct TestKeystore {
    keys: Vec<SecretKey>,
}

impl TestKeystore {
    pub fn generate_key<R>(&mut self, rng: &mut R) -> usize
    where
        R: rand::Rng + ?Sized,
    {
        let n = self.keys.len();

        let secret = SecretKey::random(rng);

        self.keys.push(secret);

        n
    }
}

impl Keystore for TestKeystore {
    type Error = io::Error;
    type KeyId = usize;

    fn secret(&self, id: &usize) -> Result<Option<Borrown<'_, SecretKey>>, io::Error> {
        Ok(self.keys.get(*id).map(Borrown::from))
    }
}

impl AsRef<TestKeystore> for TestKeystore {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl Signer for TestKeystore {
    type Error = io::Error;
    type Keystore = Self;

    fn keystore(&self) -> Result<&Self, Self::Error> {
        Ok(self)
    }
}

#[test]
fn signer() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let mut keystore = TestKeystore::default();

    let message = b"It is amazing how complete is the delusion that beauty is goodness.";
    let message = Message::new(message);

    let key = keystore.generate_key(rng);
    let key_p = keystore.generate_key(rng);

    assert_ne!(key, key_p);

    keystore
        .public(&key)
        .expect("Test keystore is infallible")
        .expect("PK was inserted");

    keystore
        .public(&key_p)
        .expect("Test keystore is infallible")
        .expect("PK was inserted");

    let signature = keystore.sign(&key, &message).expect("Failed to sign");
    let signature_p = keystore.sign(&key_p, &message).expect("Failed to sign");

    let public = keystore
        .public(&key)
        .expect("Failed to access keystore")
        .expect("Key not found");

    let public_p = keystore
        .public(&key_p)
        .expect("Failed to access keystore")
        .expect("Key not found");

    signature
        .verify(public.as_ref(), &message)
        .expect("Failed to verify signature");

    signature_p
        .verify(public_p.as_ref(), &message)
        .expect("Failed to verify signature");

    signature
        .verify(public_p.as_ref(), &message)
        .err()
        .expect("Wrong key should fail verification");

    signature_p
        .verify(public.as_ref(), &message)
        .err()
        .expect("Wrong key should fail verification");
}
