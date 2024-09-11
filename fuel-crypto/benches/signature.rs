use criterion::{
    black_box,
    criterion_group,
    criterion_main,
    Criterion,
};
use sha2::digest::Update;

fn signatures(c: &mut Criterion) {
    let message = b"New opinions are always suspected, and usually opposed, without any other reason but because they are not common.";

    // fuel-crypto
    let (fc_key, fc_public, fc_message, fc_signature) = {
        use fuel_crypto::{
            Message,
            SecretKey,
            Signature,
        };
        use rand::{
            rngs::StdRng,
            SeedableRng,
        };

        let rng = &mut StdRng::seed_from_u64(8586);

        let message = Message::new(message);
        let key = SecretKey::random(rng);
        let public = key.public_key();
        let signature = Signature::sign(&key, &message);

        signature
            .verify(&public, &message)
            .expect("verification failed");

        let x = signature.recover(&message).expect("failed to recover");

        assert_eq!(x, public);

        (key, public, message, signature)
    };

    // secp256k1
    let (
        s2_secp,
        s2_secp_signing,
        s2_secp_verification,
        s2_key,
        s2_public,
        s2_message,
        s2_signature,
        s2_recoverable,
    ) = {
        use secp256k1::{
            Message,
            PublicKey,
            Secp256k1,
            SecretKey,
        };

        let secp = Secp256k1::new();
        let secp_signing = Secp256k1::signing_only();
        let secp_verification = Secp256k1::verification_only();

        let key = [
            0x3b, 0x94, 0xb, 0x55, 0x86, 0x82, 0x3d, 0xfd, 0x2, 0xae, 0x3b, 0x46, 0x1b,
            0xb4, 0x33, 0x6b, 0x5e, 0xcb, 0xae, 0xfd, 0x66, 0x27, 0xaa, 0x92, 0x2e, 0xfc,
            0x4, 0x8f, 0xec, 0xc, 0x88, 0x1c,
        ];
        let key = SecretKey::from_slice(&key).expect("Failed to create secret key");

        let public = PublicKey::from_secret_key(&secp, &key);
        let message = fuel_crypto::Message::new(message);
        let message = Message::from_digest_slice(message.as_ref())
            .expect("failed to create secp message");
        let signature = secp_signing.sign_ecdsa(&message, &key);
        let recoverable = secp.sign_ecdsa_recoverable(&message, &key);

        secp_verification
            .verify_ecdsa(&message, &signature, &public)
            .expect("failed to verify secp");

        let x = secp
            .recover_ecdsa(&message, &recoverable)
            .expect("failed to recover");

        assert_eq!(public, x);

        (
            secp,
            secp_signing,
            secp_verification,
            key,
            public,
            message,
            signature,
            recoverable,
        )
    };

    // k256
    let (k2_key, k2_verifying, k2_digest, k2_signature, k2_recovery_id) = {
        use k256::ecdsa::{
            signature::DigestVerifier,
            SigningKey,
            VerifyingKey,
        };
        use sha2::{
            Digest,
            Sha256,
        };

        let digest = Sha256::new().chain(message);

        let key = [
            0x0c, 0xbf, 0xdc, 0xe0, 0xb6, 0xa6, 0x88, 0x89, 0x1a, 0x43, 0xea, 0xfb, 0x43,
            0x68, 0x2e, 0xde, 0x02, 0xc9, 0x1d, 0x61, 0xa9, 0x89, 0xd0, 0xb4, 0x39, 0x16,
            0x25, 0xec, 0x80, 0x93, 0xfb, 0xa1,
        ];
        let key = SigningKey::from_bytes((&key).into()).expect("failed to create key");
        let verifying = VerifyingKey::from(key.clone());

        let (signature, recovery_id) = key
            .sign_digest_recoverable(digest.clone())
            .expect("Failed to sign");

        verifying
            .verify_digest(digest.clone(), &signature)
            .expect("failed to verify");

        let recovered =
            VerifyingKey::recover_from_digest(digest.clone(), &signature, recovery_id)
                .expect("failed to recover");

        assert_eq!(recovered, verifying);

        (key, verifying, digest, signature, recovery_id)
    };

    let mut group_sign = c.benchmark_group("sign");

    group_sign.bench_with_input(
        "fuel-crypto-sign",
        &(fc_key, fc_message),
        |b, (key, message)| {
            b.iter(|| fuel_crypto::Signature::sign(black_box(key), black_box(message)))
        },
    );

    group_sign.bench_with_input(
        "fuel-crypto-digest",
        &(fc_key, message),
        |b, (key, message)| {
            b.iter(|| {
                let message = fuel_crypto::Message::new(black_box(message));

                fuel_crypto::Signature::sign(black_box(key), black_box(&message))
            })
        },
    );

    group_sign.bench_with_input(
        "secp256k1",
        &(s2_secp_signing, s2_key, s2_message),
        |b, (secp_signing, key, message)| {
            b.iter(|| secp_signing.sign_ecdsa(black_box(message), black_box(key)))
        },
    );

    group_sign.bench_with_input(
        "secp256k1-recoverable",
        &(s2_secp.clone(), s2_key, s2_message),
        |b, (secp, key, message)| {
            b.iter(|| secp.sign_ecdsa_recoverable(black_box(message), black_box(key)))
        },
    );

    group_sign.bench_with_input(
        "k256",
        &(k2_key, k2_digest.clone()),
        |b, (key, digest)| {
            b.iter(|| {
                use k256::ecdsa::signature::DigestSigner;

                let sig: k256::ecdsa::Signature =
                    key.sign_digest(black_box(digest.clone()));

                sig
            })
        },
    );

    group_sign.finish();

    let mut group_verify = c.benchmark_group("verify");

    group_verify.bench_with_input(
        "fuel-crypto-verify",
        &(fc_public, fc_signature, fc_message),
        |b, (public, signature, message)| {
            b.iter(|| signature.verify(black_box(public), black_box(message)))
        },
    );

    group_verify.bench_with_input(
        "secp256k1",
        &(s2_secp_verification, s2_public, s2_signature, s2_message),
        |b, (secp_verification, public, signature, message)| {
            b.iter(|| {
                secp_verification.verify_ecdsa(
                    black_box(message),
                    black_box(signature),
                    black_box(public),
                )
            })
        },
    );

    group_verify.bench_with_input(
        "k256",
        &(k2_verifying, k2_digest.clone(), k2_signature),
        |b, (verifying, digest, signature)| {
            b.iter(|| {
                use k256::ecdsa::signature::DigestVerifier;

                verifying.verify_digest(black_box(digest.clone()), black_box(signature))
            })
        },
    );

    group_verify.finish();

    let mut group_recover = c.benchmark_group("recover");

    group_recover.bench_with_input(
        "fuel-crypto-recover",
        &(fc_signature, fc_message),
        |b, (signature, message)| b.iter(|| signature.recover(black_box(message))),
    );

    group_recover.bench_with_input(
        "secp256k1",
        &(s2_secp, s2_recoverable, s2_message),
        |b, (secp, recoverable, message)| {
            b.iter(|| secp.recover_ecdsa(black_box(message), black_box(recoverable)))
        },
    );

    group_recover.bench_with_input(
        "k256",
        &(k2_signature, k2_recovery_id, k2_digest),
        |b, (signature, recovery_id, digest)| {
            b.iter(|| {
                k256::ecdsa::VerifyingKey::recover_from_digest(
                    digest.clone(),
                    signature,
                    *recovery_id,
                )
            })
        },
    );

    group_recover.finish();
}

criterion_group!(benches, signatures);
criterion_main!(benches);
