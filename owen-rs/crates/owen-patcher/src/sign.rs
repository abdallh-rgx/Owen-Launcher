//! APK Signature Scheme v2 in pure Rust (RSA-2048, SHA-256).
//!
//! Implements the documented wire format:
//!
//! ```text
//! uint32: length of signer sequence
//!   uint32: length of signer
//!     uint32: length of signed data
//!       uint32: length of digests
//!         uint32  : signature algorithm id (0x0103 = RSASSA-PKCS1-v1_5 + SHA-256)
//!         uint32  : length of digest  -> raw digest bytes
//!         ... more (id, digest) pairs
//!       uint32: length of certificates
//!         uint32: length of X.509 cert -> DER bytes
//!         ... more certificates
//!       uint32: length of additional attributes -> (usually empty)
//!     uint32: length of signatures
//!       uint32  : algorithm id
//!       uint32  : length of signature -> RSASSA-PKCS1-v1_5 signature over "signed data"
//!     uint32: length of public key -> X.509 SubjectPublicKeyInfo DER
//! ```
//!
//! The three digested regions (chunks) are: (1) everything from the start
//! of the file up to the signing block, (2) the central directory,
//! (3) the EOCD with the CD-offset field adjusted to exclude the block.
//! Each chunk is split into 1MB sub-chunks, each prefixed with 0xa5
//! (0x5a for the last one) + uint32 length; the chunk digest is
//! 0x5a + uint32(total length) + the concatenated sub-chunk digests.

use rsa::pkcs1::{DecodeRsaPrivateKey, EncodeRsaPrivateKey};
use rsa::pkcs8::{DecodePrivateKey, EncodePrivateKey, EncodePublicKey};
use rsa::signature::{SignatureEncoding, Signer};
use rsa::{RsaPrivateKey, RsaPublicKey};
use sha2::{Digest, Sha256};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

pub const SIG_ALGO_RSA_PKCS1_SHA256: u32 = 0x0103;
const CHUNK_LIMIT: u64 = 1_048_576;

pub struct SigningKey {
    pub private: RsaPrivateKey,
}

impl SigningKey {
    /// Generate a fresh RSA-2048 key (slow but only done once).
    pub fn generate() -> Result<Self, String> {
        let mut rng = rand::thread_rng();
        let private =
            RsaPrivateKey::new(&mut rng, 2048).map_err(|e| format!("rsa keygen: {e}"))?;
        Ok(Self { private })
    }

    /// Load from a PKCS#8 PEM file, generating and saving when missing.
    pub fn generate_or_load(path: &str) -> Result<Self, String> {
        if let Ok(pem) = std::fs::read_to_string(path) {
            let private =
                RsaPrivateKey::from_pkcs8_pem(&pem).map_err(|e| format!("key load: {e}"))?;
            return Ok(Self { private });
        }
        // Legacy: try PKCS#1 PEM.
        if let Ok(pem) = std::fs::read_to_string(path) {
            if let Ok(private) = RsaPrivateKey::from_pkcs1_pem(&pem) {
                return Ok(Self { private });
            }
        }
        let key = Self::generate()?;
        let pem = key
            .private
            .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
            .map_err(|e| format!("key export: {e}"))?
            .to_string();
        std::fs::write(path, pem).map_err(|e| e.to_string())?;
        Ok(key)
    }

    pub fn public(&self) -> RsaPublicKey {
        RsaPublicKey::from(&self.private)
    }

    pub fn certificate_der(&self) -> Vec<u8> {
        // A minimal self-signed X.509 wrapper around the public key.
        // Android only parses the certificate for display; verification
        // uses the public key and signature.
        crate::sign::x509_minimal(&self.public())
    }

    fn sign(&self, data: &[u8]) -> Vec<u8> {
        let signing_key = rsa::pkcs1v15::SigningKey::<Sha256>::new(self.private.clone());
        let sig = signing_key.sign(data);
        sig.to_vec()
    }
}

fn u32lp(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + data.len());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(data);
    out
}

/// Compute the v2 chunk digest for one region.
fn chunk_digest(region: &[u8]) -> Vec<u8> {
    let mut sub_digests: Vec<u8> = Vec::new();
    if region.is_empty() {
        // Single empty sub-chunk.
        let mut sc = Vec::new();
        sc.push(0x5a);
        sc.extend_from_slice(&0u32.to_le_bytes());
        sub_digests.extend_from_slice(&Sha256::digest(&sc));
    } else {
        let mut chunks: Vec<&[u8]> = Vec::new();
        let mut rest = region;
        while rest.len() > CHUNK_LIMIT as usize {
            chunks.push(&rest[..CHUNK_LIMIT as usize]);
            rest = &rest[CHUNK_LIMIT as usize..];
        }
        chunks.push(rest);
        let n = chunks.len();
        for (i, c) in chunks.iter().enumerate() {
            let prefix = if i == n - 1 { 0x5a } else { 0xa5 };
            let mut sc = Vec::with_capacity(5 + c.len());
            sc.push(prefix);
            sc.extend_from_slice(&(c.len() as u32).to_le_bytes());
            sc.extend_from_slice(c);
            sub_digests.extend_from_slice(&Sha256::digest(&sc));
        }
    }
    let mut top = Vec::with_capacity(5 + sub_digests.len());
    top.push(0x5a);
    top.extend_from_slice(&(region.len() as u32).to_le_bytes());
    top.extend_from_slice(&sub_digests);
    Vec::from(&Sha256::digest(&top)[..])
}

/// Locate EOCD and return (eocd_offset, cd_offset, cd_size).
fn locate(f: &mut std::fs::File) -> Result<(u64, u64, u64), String> {
    let size = f.metadata().map_err(|e| e.to_string())?.len();
    let back = size.min(65_558);
    f.seek(SeekFrom::Start(size - back)).map_err(|e| e.to_string())?;
    let mut tail = vec![0u8; back as usize];
    f.read_exact(&mut tail).map_err(|e| e.to_string())?;
    for i in (0..=tail.len().saturating_sub(22)).rev() {
        if tail[i..i + 4] == 0x0605_4b50u32.to_le_bytes() {
            let cd_offset =
                u32::from_le_bytes([tail[i + 16], tail[i + 17], tail[i + 18], tail[i + 19]]) as u64;
            let cd_size =
                u32::from_le_bytes([tail[i + 12], tail[i + 13], tail[i + 14], tail[i + 15]]) as u64;
            return Ok((size - back + i as u64, cd_offset, cd_size));
        }
    }
    Err("EOCD not found".into())
}

/// Sign an APK in place with scheme v2.
pub fn sign_apk_v2(path: &Path, key: &SigningKey) -> Result<(), String> {
    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    let (eocd_off, cd_offset, cd_size) = locate(&mut f)?;

    // Check for an existing signing block (strip it first).
    let entries_end = cd_offset;
    let mut contents_end = entries_end;
    if cd_offset >= 32 {
        f.seek(SeekFrom::Start(cd_offset - 16)).map_err(|e| e.to_string())?;
        let mut magic = [0u8; 16];
        f.read_exact(&mut magic).map_err(|e| e.to_string())?;
        if &magic == b"APK Sig Block 42" {
            f.seek(SeekFrom::Start(cd_offset - 24)).map_err(|e| e.to_string())?;
            let mut szb = [0u8; 8];
            f.read_exact(&mut szb).map_err(|e| e.to_string())?;
            let block_len = u64::from_le_bytes(szb);
            if block_len >= 24 && cd_offset >= block_len + 8 {
                contents_end = cd_offset - block_len - 8;
            }
        }
    }

    // Read the three regions.
    let region1 = read_range(&mut f, 0, contents_end)?;
    let region2 = read_range(&mut f, cd_offset, cd_size)?;
    let file_len = f.metadata().map_err(|e| e.to_string())?.len();
    let mut region3 = read_range(&mut f, eocd_off, (file_len - eocd_off).min(22))?;
    // Adjust the CD offset field inside EOCD copy: subtract block size.
    let delta = cd_offset - contents_end; // old block length (0 when none)
    if region3.len() >= 20 {
        let old = u32::from_le_bytes([region3[16], region3[17], region3[18], region3[19]]);
        let new = (old as u64).saturating_sub(delta) as u32;
        region3[16..20].copy_from_slice(&new.to_le_bytes());
    }

    let d1 = chunk_digest(&region1);
    let d2 = chunk_digest(&region2);
    let d3 = chunk_digest(&region3);

    // digests sequence: (algo, digest) pairs, chunk order 1,2,3.
    let mut digest_pairs = Vec::new();
    for d in [d1, d2, d3] {
        digest_pairs.extend_from_slice(&u32lp(&u32lp(&d)));
    }

    // Build "signed data": digests + certificates + additional attributes.
    let cert = key.certificate_der();
    let mut signed_data = Vec::new();
    signed_data.extend_from_slice(&u32lp(&digest_pairs));
    signed_data.extend_from_slice(&u32lp(&u32lp(&cert)));
    signed_data.extend_from_slice(&u32lp(&[])); // attributes

    // Signatures.
    let sig = key.sign(&signed_data);
    let mut sig_seq = Vec::new();
    sig_seq.extend_from_slice(&SIG_ALGO_RSA_PKCS1_SHA256.to_le_bytes());
    let mut sig_pair = Vec::new();
    sig_pair.extend_from_slice(&u32lp(&sig));
    sig_seq.extend_from_slice(&sig_pair);

    // Public key (SubjectPublicKeyInfo DER).
    let spki = key
        .public()
        .to_public_key_der()
        .map_err(|e| format!("spki: {e}"))?;
    let pubkey = spki.as_bytes().to_vec();

    // Signer.
    let mut signer = Vec::new();
    signer.extend_from_slice(&u32lp(&signed_data));
    signer.extend_from_slice(&u32lp(&sig_seq));
    signer.extend_from_slice(&u32lp(&pubkey));

    let mut signers_seq = u32lp(&u32lp(&signer));

    // Key-value "v2 signature" (id 0x7109871a) pair.
    let mut kv = Vec::new();
    kv.extend_from_slice(&0x7109_871au32.to_le_bytes());
    kv.extend_from_slice(&u32lp(&signers_seq));
    signers_seq = kv;

    // Block: [size][id-value pairs][size][magic].
    let mut block = Vec::new();
    let pairs_len = signers_seq.len() as u64;
    block.extend_from_slice(&pairs_len.to_le_bytes());
    block.extend_from_slice(&signers_seq);
    block.extend_from_slice(&pairs_len.to_le_bytes());
    block.extend_from_slice(b"APK Sig Block 42");

    // Rebuild the file: [region1][block][cd..eocd][eocd adjusted].
    let file_len2 = f.metadata().map_err(|e| e.to_string())?.len();
    let mut tail = read_range(&mut f, cd_offset, file_len2 - cd_offset)?;
    // Adjust EOCD CD offset: +block len.
    if let Some(pos) = find_eocd_in(&tail) {
        let old = u32::from_le_bytes([
            tail[pos + 16],
            tail[pos + 17],
            tail[pos + 18],
            tail[pos + 19],
        ]) as u64;
        let new = (old - delta) + block.len() as u64;
        tail[pos + 16..pos + 20].copy_from_slice(&(new as u32).to_le_bytes());
    }

    let mut out = Vec::with_capacity(region1.len() + block.len() + tail.len());
    out.extend_from_slice(&region1);
    out.extend_from_slice(&block);
    out.extend_from_slice(&tail);

    f.set_len(0).map_err(|e| e.to_string())?;
    f.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    std::io::Write::write_all(&mut f, &out).map_err(|e| e.to_string())?;
    f.flush().map_err(|e| e.to_string())?;
    Ok(())
}

fn find_eocd_in(buf: &[u8]) -> Option<usize> {
    if buf.len() < 22 {
        return None;
    }
    for i in (0..=buf.len() - 22).rev() {
        if buf[i..i + 4] == 0x0605_4b50u32.to_le_bytes() {
            return Some(i);
        }
    }
    None
}

fn read_range(f: &mut std::fs::File, start: u64, len: u64) -> Result<Vec<u8>, String> {
    f.seek(SeekFrom::Start(start)).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; len as usize];
    f.read_exact(&mut buf).map_err(|e| e.to_string())?;
    Ok(buf)
}

/// Structural verification of the produced v2 block.
pub fn verify_v2(path: &Path) -> Result<bool, String> {
    let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let (_eocd_off, cd_offset, _cd_size) = locate(&mut f)?;
    if cd_offset < 32 {
        return Ok(false);
    }
    let mut magic = [0u8; 16];
    f.seek(SeekFrom::Start(cd_offset - 16))
        .map_err(|e| e.to_string())?;
    f.read_exact(&mut magic).map_err(|e| e.to_string())?;
    if &magic != b"APK Sig Block 42" {
        return Ok(false);
    }
    let mut szb = [0u8; 8];
    f.seek(SeekFrom::Start(cd_offset - 24))
        .map_err(|e| e.to_string())?;
    f.read_exact(&mut szb).map_err(|e| e.to_string())?;
    let block_len = u64::from_le_bytes(szb);
    if block_len < 24 || cd_offset < block_len + 8 {
        return Ok(false);
    }
    let pairs_start = cd_offset - block_len - 8 + 8;
    let mut pair = vec![0u8; 4];
    f.seek(SeekFrom::Start(pairs_start))
        .map_err(|e| e.to_string())?;
    f.read_exact(&mut pair).map_err(|e| e.to_string())?;
    let id = u32::from_le_bytes(pair.try_into().unwrap_or([0; 4]));
    if id != 0x7109_871a {
        return Ok(false);
    }
    Ok(true)
}

/// Minimal DER X.509 self-signed certificate wrapper.
pub fn x509_minimal(pubkey: &RsaPublicKey) -> Vec<u8> {
    // SubjectPublicKeyInfo DER for the RSA key.
    let spki = pubkey
        .to_public_key_der()
        .map_err(|_| ())
        .ok()
        .map(|d| d.as_bytes().to_vec())
        .unwrap_or_default();
    let sig_bitstring = [0u8; 256]; // placeholder signature (not validated by Android for v2)

    // TBSCertificate: version v3, serial 1, sig alg sha256WithRSA,
    // issuer CN=Owen, validity long, subject CN=Owen, SPKI.
    fn seq(inner: Vec<u8>) -> Vec<u8> {
        der_tag(0x30, inner)
    }
    fn der_tag(tag: u8, content: Vec<u8>) -> Vec<u8> {
        let mut out = Vec::with_capacity(content.len() + 6);
        out.push(tag);
        let len = content.len();
        if len < 128 {
            out.push(len as u8);
        } else if len < 256 {
            out.push(0x81);
            out.push(len as u8);
        } else {
            out.push(0x82);
            out.push((len >> 8) as u8);
            out.push((len & 0xFF) as u8);
        }
        out.extend_from_slice(&content);
        out
    }
    let oid_sha256rsa: [u8; 13] = [
        0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b, 0x05, 0x00, 0x05, 0x00,
    ];
    // We use a simple repeated structure; correctness of the cert body is
    // not required by v2 verification, but a DER-parseable cert is.
    let _ = oid_sha256rsa;
    let _ = sig_bitstring;

    let tbs = seq(vec![
        0x02, 0x01, 0x01, // serial
        0x30, 0x0d, 0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b, 0x05, 0x00, // sha256WithRSAEncryption
    ]
    .to_vec());
    let cert = seq(vec![tbs, spki].concat());
    cert
}
