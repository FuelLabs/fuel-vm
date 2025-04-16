use crate::{
    Message,
    SecretKey,
    Signature,
};
use rand::{
    SeedableRng,
    rngs::StdRng,
};

#[test]
fn serde() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let secret = SecretKey::random(rng);
    let secret_p = bincode::serialize(&secret).expect("Failed to serialize secret");
    let secret_p = bincode::deserialize(&secret_p).expect("Failed to deserialize secret");

    assert_eq!(secret, secret_p);

    let public = secret.public_key();
    let public_p = bincode::serialize(&public).expect("Failed to serialize public");
    let public_p = bincode::deserialize(&public_p).expect("Failed to deserialize public");

    assert_eq!(public, public_p);

    let message = b"Two souls live in me, alas, Irreconcilable with one another.";
    let message = Message::new(message);
    let message_p = bincode::serialize(&message).expect("Failed to serialize message");
    let message_p =
        bincode::deserialize(&message_p).expect("Failed to deserialize message");

    assert_eq!(message, message_p);

    let signature = Signature::sign(&secret, &message);
    let signature_p =
        bincode::serialize(&signature).expect("Failed to serialize signature");
    let signature_p =
        bincode::deserialize(&signature_p).expect("Failed to deserialize signature");

    assert_eq!(signature, signature_p);
}
