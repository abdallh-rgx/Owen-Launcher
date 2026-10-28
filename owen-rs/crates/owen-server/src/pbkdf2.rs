//! Dependency-free PBKDF2-HMAC-SHA256 password hashing.
//!
//! HMAC-SHA256 is implemented manually per RFC 2104 (ipad/opad with a
//! 64-byte block) on top of the `sha2` crate, and PBKDF2 follows RFC 8018
//! with a single 32-byte block (`dkLen == hLen`), so no external crypto
//! dependency is needed. Stored hashes use the format
//! `pbkdf2$<rounds>$<salt-hex-32>$<hash-hex-64>` and verification compares
//! in constant time.

use sha2::{Digest, Sha256};

/// Default PBKDF2 iteration count for new password hashes.
pub const DEFAULT_ROUNDS: u32 = 60_000;

/// Maximum rounds accepted when verifying (guards against absurd values
/// crafted to burn CPU).
const MAX_ROUNDS: u32 = 10_000_000;

/// HMAC-SHA256 (RFC 2104) with the SHA-256 block size of 64 bytes.
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    // Keys longer than the block are hashed first; shorter keys are
    // zero-padded to exactly one block.
    let mut key_block = [0u8; BLOCK];
    if key.len() > BLOCK {
        let digest = Sha256::digest(key);
        key_block[..32].copy_from_slice(&digest);
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; BLOCK];
    let mut opad = [0x5cu8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] ^= key_block[i];
        opad[i] ^= key_block[i];
    }
    let inner = Sha256::new().chain_update(ipad).chain_update(data).finalize();
    let out = Sha256::new().chain_update(opad).chain_update(inner).finalize();
    let mut mac = [0u8; 32];
    mac.copy_from_slice(&out);
    mac
}

/// PBKDF2-HMAC-SHA256 with a 32-byte output (RFC 8018, single block:
/// U1 = HMAC(P, S || INT_32_BE(1)), Uj = HMAC(P, U(j-1)), T = U1 ^ ... ^ Uc).
pub fn pbkdf2_sha256(password: &str, salt: &[u8], iterations: u32) -> [u8; 32] {
    let mut derived = [0u8; 32];
    if iterations == 0 {
        return derived;
    }
    let mut msg = Vec::with_capacity(salt.len() + 4);
    msg.extend_from_slice(salt);
    msg.extend_from_slice(&1u32.to_be_bytes());
    let mut u = hmac_sha256(password.as_bytes(), &msg);
    derived.copy_from_slice(&u);
    for _ in 2..=iterations {
        u = hmac_sha256(password.as_bytes(), &u);
        for (t, ub) in derived.iter_mut().zip(u.iter()) {
            *t ^= *ub;
        }
    }
    derived
}

/// Hash a password with a fresh 16-byte random salt and [`DEFAULT_ROUNDS`].
pub fn hash_password(password: &str) -> String {
    hash_password_with_rounds(password, DEFAULT_ROUNDS)
}

/// Hash a password with an explicit iteration count (used by tests and
/// tooling; production code should prefer [`hash_password`]).
pub fn hash_password_with_rounds(password: &str, rounds: u32) -> String {
    use rand::RngCore;
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    let hash = pbkdf2_sha256(password, &salt, rounds);
    format!("pbkdf2${rounds}${}${}", hex_encode(&salt), hex_encode(&hash))
}

/// Verify a password against a stored `pbkdf2$rounds$salt$hash` string.
/// The comparison is constant-time (XOR fold) over the 32-byte digest.
pub fn verify_password(password: &str, stored: &str) -> bool {
    let parts: Vec<&str> = stored.split('$').collect();
    if parts.len() != 4 || parts[0] != "pbkdf2" {
        return false;
    }
    let Ok(rounds) = parts[1].parse::<u32>() else {
        return false;
    };
    if rounds == 0 || rounds > MAX_ROUNDS {
        return false;
    }
    let Some(salt) = hex_decode(parts[2]) else {
        return false;
    };
    let Some(expected) = hex_decode(parts[3]) else {
        return false;
    };
    if expected.len() != 32 {
        return false;
    }
    let computed = pbkdf2_sha256(password, &salt, rounds);
    constant_time_eq(&computed, &expected)
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> String {
        s.to_string()
    }

    #[test]
    fn hmac_rfc4231_vectors() {
        // RFC 4231 test case 1: key = 0x0b * 20, data = "Hi There"
        let key = [0x0bu8; 20];
        let mac = hmac_sha256(&key, b"Hi There");
        assert_eq!(
            hex_encode(&mac),
            hex("b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7")
        );
        // RFC 4231 test case 2: key = "Jefe"
        let mac = hmac_sha256(b"Jefe", b"what do ya want for nothing?");
        assert_eq!(
            hex_encode(&mac),
            hex("5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843")
        );
    }

    #[test]
    fn pbkdf2_rfc8018_vectors() {
        // Official PBKDF2-HMAC-SHA256 vectors (P = "password", S = "salt"),
        // cross-checked against Python hashlib and Node crypto. (The task
        // notes listed c=4096 ending in "34b"; every reference
        // implementation produces "34a".)
        assert_eq!(
            hex_encode(&pbkdf2_sha256("password", b"salt", 1)),
            hex("120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b")
        );
        assert_eq!(
            hex_encode(&pbkdf2_sha256("password", b"salt", 2)),
            hex("ae4d0c95af6b46d32d0adff928f06dd02a303f8ef3c251dfd6e2d85a95474c43")
        );
        assert_eq!(
            hex_encode(&pbkdf2_sha256("password", b"salt", 4096)),
            hex("c5e478d59288c841aa530db6845c4c8d962893a001ce4e11a4963873aa98134a")
        );
    }

    #[test]
    fn hash_verify_roundtrip() {
        let stored = hash_password("correct horse battery staple");
        assert!(stored.starts_with("pbkdf2$60000$"));
        let parts: Vec<&str> = stored.split('$').collect();
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[2].len(), 32, "16-byte salt as 32 hex chars");
        assert_eq!(parts[3].len(), 64, "32-byte hash as 64 hex chars");
        assert!(verify_password("correct horse battery staple", &stored));
        assert!(!verify_password("Tr0ub4dor&3", &stored));
    }

    #[test]
    fn hash_verify_roundtrip_low_rounds() {
        // Same roundtrip through the shared parsing path, cheap iterations.
        let stored = hash_password_with_rounds("s3cret-pass", 64);
        assert!(verify_password("s3cret-pass", &stored));
        assert!(!verify_password("s3cret-pass ", &stored));
        assert!(!verify_password(" s3cret-pass", &stored));
    }

    #[test]
    fn verify_rejects_malformed_stored() {
        assert!(!verify_password("x", ""));
        assert!(!verify_password("x", "pbkdf2"));
        assert!(!verify_password("x", "pbkdf2$abc$00$00"));
        assert!(!verify_password("x", "pbkdf2$0$00$00"));
        assert!(!verify_password("x", "bcrypt$60000$abcd$ef01"));
        assert!(!verify_password(
            "x",
            "pbkdf2$60000$00112233$00000000000000000000000000000000"
        ));
    }

    #[test]
    fn salt_is_random_per_call() {
        let a = hash_password_with_rounds("same-password", 16);
        let b = hash_password_with_rounds("same-password", 16);
        assert_ne!(a, b, "each hash uses a fresh random salt");
    }
}
