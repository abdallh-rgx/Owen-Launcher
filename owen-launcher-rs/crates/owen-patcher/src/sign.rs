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

use rsa::pkcs1::DecodeRsaPrivateKey;
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
    /// Optional externally-provided signing certificate (e.g. the original
    /// `ogfn` debug keystore pair). `None` keeps today's self-signed
    /// behavior (v1: [`crate::sign_v1::self_signed_cert`], v2:
    /// [`x509_minimal`]).
    pub cert_der: Option<Vec<u8>>,
}

impl SigningKey {
    /// Generate a fresh RSA-2048 key (slow but only done once).
    pub fn generate() -> Result<Self, String> {
        let mut rng = rand::thread_rng();
        let private =
            RsaPrivateKey::new(&mut rng, 2048).map_err(|e| format!("rsa keygen: {e}"))?;
        Ok(Self { private, cert_der: None })
    }

    /// Load from a PKCS#8 PEM file, generating and saving when missing.
    pub fn generate_or_load(path: &str) -> Result<Self, String> {
        if let Ok(pem) = std::fs::read_to_string(path) {
            let private =
                RsaPrivateKey::from_pkcs8_pem(&pem).map_err(|e| format!("key load: {e}"))?;
            return Ok(Self { private, cert_der: None });
        }
        // Legacy: try PKCS#1 PEM.
        if let Ok(pem) = std::fs::read_to_string(path) {
            if let Ok(private) = RsaPrivateKey::from_pkcs1_pem(&pem) {
                return Ok(Self { private, cert_der: None });
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

    /// Load a pre-existing key/certificate pair (PKCS#8 PEM private key +
    /// DER X.509 certificate) — the `ogfn` pair extracted from the original
    /// `ogfn-debug.keystore`. Every signature block (v1 `CERT.RSA` and the
    /// v2 signed-data) then embeds the ORIGINAL certificate, so the output
    /// installs as an update over the Kotlin app.
    pub fn load_pair(key_pem_path: &str, cert_der_path: &str) -> Result<Self, String> {
        let text = std::fs::read_to_string(key_pem_path)
            .map_err(|e| format!("key read {key_pem_path}: {e}"))?;
        // The `ogfn` key file is a `openssl pkcs12` bag dump: `Bag Attributes`
        // preamble + the PRIVATE KEY block + an extra CERTIFICATE block. Pull
        // just the private-key PEM section out.
        let pem = pem_section(&text, "PRIVATE KEY")
            .or_else(|| pem_section(&text, "RSA PRIVATE KEY"))
            .ok_or_else(|| format!("no PRIVATE KEY PEM section in {key_pem_path}"))?;
        let private = RsaPrivateKey::from_pkcs8_pem(&pem)
            .or_else(|_| RsaPrivateKey::from_pkcs1_pem(&pem))
            .map_err(|e| format!("key parse {key_pem_path}: {e}"))?;
        let cert = std::fs::read(cert_der_path)
            .map_err(|e| format!("cert read {cert_der_path}: {e}"))?;
        // Sanity: the certificate must wrap exactly this key's public key
        // (Android's v2 verifier checks cert-SPKI == embedded public key).
        let embedded = crate::sign_v1::extract_cert_public_key(&cert)
            .ok_or_else(|| format!("cert parse {cert_der_path}: not an X.509 DER cert"))?;
        let mine = RsaPublicKey::from(&private);
        if embedded != mine {
            return Err(format!(
                "key/cert mismatch: {cert_der_path} does not wrap {key_pem_path}'s public key"
            ));
        }
        Ok(Self { private, cert_der: Some(cert) })
    }

    pub fn public(&self) -> RsaPublicKey {
        RsaPublicKey::from(&self.private)
    }

    pub fn certificate_der(&self) -> Vec<u8> {
        match &self.cert_der {
            Some(der) => der.clone(),
            // A FULLY PARSEABLE self-signed X.509. The old `x509_minimal`
            // fallback (no issuer/serial/validity) sailed past our own
            // verify_v2 but Android's conscrypt rejected it on-device with
            // `INSTALL_PARSE_FAILED_NO_CERTIFICATES … ASN.1 FIELD_MISSING`
            // (API 30 emulator, ogs-1241 round 17) — v2 verification DOES
            // parse the signer certificate; a placeholder is not an option.
            // This was the "الـ APK ليس APK صالح" root cause.
            None => crate::sign_v1::self_signed_cert(&self.private),
        }
    }

    /// The certificate embedded into signature blocks by the v1 signer:
    /// unlike [`Self::certificate_der`] (whose fallback `x509_minimal` has no
    /// issuer/serial/validity structure), `CERT.RSA` requires a fully
    /// parseable certificate — so the fallback here is the proper
    /// self-signed one (today's behavior).
    pub fn cert_for_v1(&self) -> Vec<u8> {
        match &self.cert_der {
            Some(der) => der.clone(),
            None => crate::sign_v1::self_signed_cert(&self.private),
        }
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

/// Extract one `-----BEGIN <label>----- … -----END <label>-----` section
/// from a (possibly noisy) PEM file. The `ogfn` key file is an
/// `openssl pkcs12` bag dump — `Bag Attributes` preamble, the private-key
/// block, then an extra CERTIFICATE block — so a plain
/// `RsaPrivateKey::from_pkcs8_pem(file)` chokes on it.
pub fn pem_section(text: &str, label: &str) -> Option<String> {
    let begin = format!("-----BEGIN {label}-----");
    let end = format!("-----END {label}-----");
    let start = text.find(&begin)?;
    let stop = start + text[start..].find(&end)? + end.len();
    Some(text[start..stop].to_string())
}

fn u64le(v: u64) -> [u8; 8] {
    v.to_le_bytes()
}

/// Spec-exact top-level digest: the whole APK contents are split into 1MB
/// chunks in order (region 1, then the central directory, then the adjusted
/// EOCD); every chunk is prefixed with 0xa5 (0x5a for the last one) + uint32
/// length and SHA-256'd; the top-level digest is SHA-256 over
/// `0x5a || uint32(total length) || concatenation of chunk digests`.
fn top_digest(regions: &[&[u8]]) -> Vec<u8> {
    let mut total_len: u64 = 0;
    let mut chunks: Vec<&[u8]> = Vec::new();
    for r in regions {
        total_len += r.len() as u64;
        if r.is_empty() {
            continue;
        }
        let mut rest: &[u8] = r;
        while rest.len() > CHUNK_LIMIT as usize {
            chunks.push(&rest[..CHUNK_LIMIT as usize]);
            rest = &rest[CHUNK_LIMIT as usize..];
        }
        if !rest.is_empty() {
            chunks.push(rest);
        }
    }
    if chunks.is_empty() {
        // Degenerate: one empty chunk (still prefixed + digested).
        chunks.push(&[]);
        total_len = 0;
    }
    let n = chunks.len();
    let mut sub_digests: Vec<u8> = Vec::with_capacity(32 * n);
    for (i, c) in chunks.iter().enumerate() {
        let prefix: u8 = if i == n - 1 { 0x5a } else { 0xa5 };
        let mut sc = Vec::with_capacity(5 + c.len());
        sc.push(prefix);
        sc.extend_from_slice(&(c.len() as u32).to_le_bytes());
        sc.extend_from_slice(c);
        sub_digests.extend_from_slice(&Sha256::digest(&sc));
    }
    let mut top = Vec::with_capacity(5 + sub_digests.len());
    top.push(0x5a);
    top.extend_from_slice(&u64le(total_len)[..4].to_vec());
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

    let d = top_digest(&[&region1, &region2, &region3]);

    // signed data:
    //   [u32 len] digests sequence — one record: [u32 len][u32 algo][u32 len][digest]
    //   [u32 len] certificates — one record: [u32 len][cert DER]
    //   [u32 0]   additional attributes
    let mut digest_record = Vec::new();
    digest_record.extend_from_slice(&SIG_ALGO_RSA_PKCS1_SHA256.to_le_bytes());
    digest_record.extend_from_slice(&u32lp(&d));
    let digests_field = u32lp(&u32lp(&digest_record));

    let cert = key.certificate_der();
    let certificates_field = u32lp(&u32lp(&cert));
    let attributes_field: Vec<u8> = 0u32.to_le_bytes().to_vec();

    let mut signed_data = Vec::new();
    signed_data.extend_from_slice(&digests_field);
    signed_data.extend_from_slice(&certificates_field);
    signed_data.extend_from_slice(&attributes_field);

    // signatures: [u32 len] one record: [u32 len][u32 algo][u32 len][signature]
    let sig = key.sign(&signed_data);
    let mut sig_record = Vec::new();
    sig_record.extend_from_slice(&SIG_ALGO_RSA_PKCS1_SHA256.to_le_bytes());
    sig_record.extend_from_slice(&u32lp(&sig));
    let signatures_field = u32lp(&u32lp(&sig_record));

    // Public key (SubjectPublicKeyInfo DER).
    let spki = key
        .public()
        .to_public_key_der()
        .map_err(|e| format!("spki: {e}"))?;
    let pubkey = spki.as_bytes().to_vec();

    // signer: length-prefixed signed data + signatures + public key.
    // NOTE: signed_data and pubkey are raw here; signatures_field is ALREADY
    // length-prefixed (u32lp of the record sequence) — no extra wrapper.
    let mut signer = Vec::new();
    signer.extend_from_slice(&u32lp(&signed_data));
    signer.extend_from_slice(&signatures_field);
    signer.extend_from_slice(&u32lp(&pubkey));

    // value = length-prefixed sequence of length-prefixed signers (one signer).
    let value = u32lp(&u32lp(&signer));

    // ID-value pair: [u64 pair length][uint32 id][value]
    let pair_len = (4 + value.len()) as u64;
    let mut pair = Vec::new();
    pair.extend_from_slice(&u64le(pair_len));
    pair.extend_from_slice(&0x7109_871au32.to_le_bytes());
    pair.extend_from_slice(&value);

    // Block: [u64 size][pairs][u64 size][magic]; size excludes the leading
    // field itself → pairs + trailing size + magic.
    let size_of_block = (pair.len() + 8 + 16) as u64;
    let mut block = Vec::new();
    block.extend_from_slice(&u64le(size_of_block));
    block.extend_from_slice(&pair);
    block.extend_from_slice(&u64le(size_of_block));
    block.extend_from_slice(b"APK Sig Block 42");

    // Rebuild the file: [region1][block][cd..eocd][eocd adjusted].
    let file_len2 = f.metadata().map_err(|e| e.to_string())?.len();
    let mut tail = read_range(&mut f, cd_offset, file_len2 - cd_offset)?;
    // Adjust EOCD CD offset: remove the old block, add the new one.
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

/// Full cryptographic verification of the v2 block: recompute the top-level
/// digest over the three regions (signing block excluded, EOCD CD-offset
/// adjusted) and check the RSA signature against the embedded public key.
pub fn verify_v2(path: &Path) -> Result<bool, String> {
    let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let (eocd_off, cd_offset, _cd_size) = locate(&mut f)?;
    if cd_offset < 32 {
        return Ok(false);
    }
    // magic + trailing size
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
    let size_of_block = u64::from_le_bytes(szb);
    if size_of_block < 24 || cd_offset < size_of_block + 8 {
        return Ok(false);
    }
    let block_start = cd_offset - size_of_block - 8;

    // Parse the pair: [u64 pair len][u32 id][value]
    let pairs_len = (size_of_block - 24) as usize;
    let pairs = read_range(&mut f, block_start + 8, pairs_len as u64)?;
    if pairs.len() < 12 {
        return Ok(false);
    }
    let _pair_len = u64::from_le_bytes([
        pairs[0], pairs[1], pairs[2], pairs[3], pairs[4], pairs[5], pairs[6], pairs[7],
    ]);
    let id = u32::from_le_bytes([pairs[8], pairs[9], pairs[10], pairs[11]]);
    if id != 0x7109_871a {
        return Ok(false);
    }
    let value = &pairs[12..];

    // value: [u32 seq len][u32 signer len][signer]
    if value.len() < 8 {
        return Ok(false);
    }
    let seq_len = u32::from_le_bytes([value[0], value[1], value[2], value[3]]) as usize;
    if 4 + seq_len > value.len() {
        return Ok(false);
    }
    let signers = &value[4..4 + seq_len];
    if signers.len() < 4 {
        return Ok(false);
    }
    let signer_len = u32::from_le_bytes([signers[0], signers[1], signers[2], signers[3]]) as usize;
    if 4 + signer_len > signers.len() {
        return Ok(false);
    }
    let mut signer = &signers[4..4 + signer_len];

    // signer: [u32 len] signed_data, [u32 len] signatures, [u32 len] pubkey
    fn take<'a>(buf: &mut &'a [u8]) -> Option<&'a [u8]> {
        if buf.len() < 4 {
            return None;
        }
        let l = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
        if 4 + l > buf.len() {
            return None;
        }
        let out = &buf[4..4 + l];
        *buf = &buf[4 + l..];
        Some(out)
    }
    let Some(signed_data) = take(&mut signer) else {
        return Ok(false);
    };
    let Some(signatures) = take(&mut signer) else {
        return Ok(false);
    };
    let Some(pubkey_der) = take(&mut signer) else {
        return Ok(false);
    };

    // signed_data: [u32] digests, [u32] certs, [u32] attrs
    let mut sd: &[u8] = signed_data;
    let Some(digests_seq) = take(&mut sd) else {
        return Ok(false);
    };
    let Some(_certs) = take(&mut sd) else {
        return Ok(false);
    };
    // digests: [u32 record len][u32 algo][u32 digest len][digest]
    if digests_seq.len() < 12 {
        return Ok(false);
    }
    let algo = u32::from_le_bytes([digests_seq[4], digests_seq[5], digests_seq[6], digests_seq[7]]);
    if algo != SIG_ALGO_RSA_PKCS1_SHA256 {
        return Ok(false);
    }
    let dlen =
        u32::from_le_bytes([digests_seq[8], digests_seq[9], digests_seq[10], digests_seq[11]]) as usize;
    if 12 + dlen > digests_seq.len() || dlen != 32 {
        return Ok(false);
    }
    let claimed_digest = &digests_seq[12..12 + dlen];

    // signatures: [u32 record len][u32 algo][u32 sig len][signature]
    if signatures.len() < 12 {
        return Ok(false);
    }
    let sig_algo =
        u32::from_le_bytes([signatures[4], signatures[5], signatures[6], signatures[7]]);
    if sig_algo != SIG_ALGO_RSA_PKCS1_SHA256 {
        return Ok(false);
    }
    let slen =
        u32::from_le_bytes([signatures[8], signatures[9], signatures[10], signatures[11]]) as usize;
    if 12 + slen > signatures.len() {
        return Ok(false);
    }
    let signature = &signatures[12..12 + slen];

    // Recompute the three regions for a hypothetical blockless file.
    let region1 = read_range(&mut f, 0, block_start)?;
    let region2 = read_range(&mut f, cd_offset, eocd_off - cd_offset)?;
    let file_len = f.metadata().map_err(|e| e.to_string())?.len();
    let mut region3 = read_range(&mut f, eocd_off, (file_len - eocd_off).min(22))?;
    if region3.len() >= 20 {
        let old = u32::from_le_bytes([
            region3[16], region3[17], region3[18], region3[19],
        ]) as u64;
        let new = (old - (size_of_block + 8)) as u32;
        region3[16..20].copy_from_slice(&new.to_le_bytes());
    }
    let actual_digest = top_digest(&[&region1, &region2, &region3]);
    if actual_digest != claimed_digest {
        return Ok(false);
    }

    // Verify the RSA-PKCS1v15-SHA256 signature with the embedded key.
    use rsa::pkcs8::DecodePublicKey;
    let public = RsaPublicKey::from_public_key_der(pubkey_der)
        .map_err(|e| format!("embedded pubkey: {e}"))?;
    use rsa::signature::Verifier;
    let verifying = rsa::pkcs1v15::VerifyingKey::<Sha256>::new(public);
    let signature = rsa::pkcs1v15::Signature::try_from(signature)
        .map_err(|_| "bad signature length".to_string())?;
    let ok = verifying.verify(signed_data, &signature).is_ok();
    Ok(ok)
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

