use super::source_may_need_passphrase;
use ssh_key::{Algorithm, LineEnding, PrivateKey, rand_core::OsRng};

#[test]
fn only_an_openssh_key_without_a_cipher_skips_the_passphrase() {
    let plain = PrivateKey::random(&mut OsRng, Algorithm::Ed25519).unwrap();
    let encrypted = plain.encrypt(&mut OsRng, "secret").unwrap();
    let plain_text = plain.to_openssh(LineEnding::LF).unwrap().to_string();
    let cases: Vec<(&str, Vec<u8>, bool)> =
        vec![
        ("plain", plain_text.clone().into_bytes(), false),
        (
            "plain with CRLF",
            plain_text.replace('\n', "\r\n").into_bytes(),
            false,
        ),
        (
            "plain after leading whitespace",
            format!("\n  {plain_text}").into_bytes(),
            false,
        ),
        (
            "encrypted",
            encrypted.to_openssh(LineEnding::LF).unwrap().as_bytes().to_vec(),
            true,
        ),
        (
            "legacy PEM",
            b"-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA\n-----END RSA PRIVATE KEY-----\n"
                .to_vec(),
            true,
        ),
        ("empty", Vec::new(), true),
        ("garbage", b"not a key at all".to_vec(), true),
        (
            "truncated after the header",
            b"-----BEGIN OPENSSH PRIVATE KEY-----\nb3Bl".to_vec(),
            true,
        ),
    ];
    let directory = tempfile::tempdir().unwrap();
    for (name, bytes, expected) in cases {
        let path = directory.path().join("key");
        std::fs::write(&path, bytes).unwrap();
        assert_eq!(source_may_need_passphrase(&path), expected, "{name}");
    }
    assert!(source_may_need_passphrase(
        &directory.path().join("missing")
    ));
}
