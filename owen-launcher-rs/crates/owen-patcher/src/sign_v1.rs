//! APK v1 (JAR) signing — `MANIFEST.MF` + `CERT.SF` + `CERT.RSA`, signed with
//! the SAME key as the v2 signer, the Rust port of the v1 half of
//! `ApkSignerTool.kt` (`setV1SigningEnabled(true)` + the shared `ogfn` key).
//!
//! ## Kotlin source map
//!
//! | Kotlin (`ApkSignerTool.kt`)                       | Rust (here)                        |
//! |---------------------------------------------------|------------------------------------|
//! | `ApkSigner.Builder…setV1SigningEnabled(true)`     | [`sign_apk_v1`]                    |
//! | `ApkSigner.SignerConfig(KEY_ALIAS, key, [cert])`  | [`sign_apk_v1`] with [`sign::SigningKey`] |
//! | `ApkVerifier.Builder(apk).build().verify()`       | [`verify_v1`]                      |
//! | `KEYSTORE_ASSET`/`KEYSTORE_PASS`/`KEY_ALIAS`      | key generated/loaded by `sign.rs` (`generate_or_load`) |
//!
//! The digest algorithm is SHA-256 (`SHA-256-Digest`, min-SDK 21+ — SHA-1 is
//! not needed, so no `sha1` dependency was added). `CERT.RSA` is a PKCS#7
//! SignedData built on a real self-signed X.509 certificate so both v1 and
//! v2 embed the identical certificate.
//!
//! Source: Owen-Launcher app/src/main/kotlin/com/abdallhrgx/owenlauncher/core/apk/ApkSignerTool.kt

use crate::sign::SigningKey;
use crate::ziprw::ZipArchive;
use rsa::pkcs8::EncodePublicKey;
use rsa::signature::{SignatureEncoding, Signer, Verifier};
use rsa::{RsaPrivateKey, RsaPublicKey};
use sha2::{Digest, Sha256};
use std::path::Path;

pub const MANIFEST_MF: &str = "META-INF/MANIFEST.MF";
pub const CERT_SF: &str = "META-INF/CERT.SF";
pub const CERT_RSA: &str = "META-INF/CERT.RSA";

const CREATED_BY: &str = "1.0 (Android OwenSigner)";
/// Maximum bytes per JAR header line (spec: 72 including CRLF).
const LINE_LIMIT: usize = 72;

/* ------------------------------- base64 --------------------------------- */

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { B64[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { B64[n as usize & 63] as char } else { '=' });
    }
    out
}

pub fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let mut vals = Vec::with_capacity(s.len());
    for c in s.bytes() {
        if c == b'=' {
            vals.push(0xFF);
        } else {
            let v = B64.iter().position(|&b| b == c)? as u32;
            vals.push(v);
        }
    }
    let mut out = Vec::with_capacity(vals.len() * 3 / 4);
    for chunk in vals.chunks(4) {
        if chunk.len() < 2 || chunk[0] == 0xFF || chunk[1] == 0xFF {
            break;
        }
        let n = (chunk[0] << 18) | (chunk[1] << 12)
            | (chunk.get(2).map(|&v| if v == 0xFF { 0 } else { v << 6 }).unwrap_or(0))
            | (chunk.get(3).map(|&v| if v == 0xFF { 0 } else { v }).unwrap_or(0));
        out.push((n >> 16) as u8);
        if chunk.len() > 2 && chunk[2] != 0xFF {
            out.push((n >> 8) as u8);
        }
        if chunk.len() > 3 && chunk[3] != 0xFF {
            out.push(n as u8);
        }
    }
    Some(out)
}

/* ------------------------------ DER builders ---------------------------- */

fn der_len(n: usize) -> Vec<u8> {
    if n < 0x80 {
        vec![n as u8]
    } else if n < 0x100 {
        vec![0x81, n as u8]
    } else if n < 0x1_0000 {
        vec![0x82, (n >> 8) as u8, n as u8]
    } else {
        vec![0x83, (n >> 16) as u8, (n >> 8) as u8, n as u8]
    }
}

fn tlv(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(content.len() + 5);
    out.push(tag);
    out.extend_from_slice(&der_len(content.len()));
    out.extend_from_slice(content);
    out
}

fn der_seq(parts: &[&[u8]]) -> Vec<u8> {
    let mut content = Vec::new();
    for p in parts {
        content.extend_from_slice(p);
    }
    tlv(0x30, &content)
}

fn der_set(parts: &[&[u8]]) -> Vec<u8> {
    let mut content = Vec::new();
    for p in parts {
        content.extend_from_slice(p);
    }
    tlv(0x31, &content)
}

fn der_int_bytes(bytes: &[u8]) -> Vec<u8> {
    // Minimal two's-complement INTEGER (positive values: prepend 0 when the
    // high bit is set).
    let mut b = bytes;
    while b.len() > 1 && b[0] == 0 {
        b = &b[1..];
    }
    let content = if b[0] & 0x80 != 0 {
        let mut c = vec![0u8];
        c.extend_from_slice(b);
        c
    } else {
        b.to_vec()
    };
    tlv(0x02, &content)
}

fn der_int(v: u64) -> Vec<u8> {
    der_int_bytes(&v.to_be_bytes())
}

fn der_oid(encoded: &[u8]) -> Vec<u8> {
    tlv(0x06, encoded)
}

fn der_octet(data: &[u8]) -> Vec<u8> {
    tlv(0x04, data)
}

fn der_bit_string(data: &[u8]) -> Vec<u8> {
    let mut content = vec![0u8]; // no unused bits
    content.extend_from_slice(data);
    tlv(0x03, &content)
}

fn der_utf8(s: &str) -> Vec<u8> {
    tlv(0x0C, s.as_bytes())
}

fn der_utc(s: &str) -> Vec<u8> {
    tlv(0x17, s.as_bytes())
}

const OID_SIGNED_DATA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x02];
const OID_DATA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x01];
const OID_SHA256: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01];
const OID_RSA_ENCRYPTION: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x01];
const OID_SHA256_RSA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0B];
const OID_CN: &[u8] = &[0x55, 0x04, 0x03];

const NULL: &[u8] = &[0x05, 0x00];

fn alg_sha256() -> Vec<u8> {
    der_seq(&[&der_oid(OID_SHA256), NULL])
}

fn alg_sha256_rsa() -> Vec<u8> {
    der_seq(&[&der_oid(OID_SHA256_RSA), NULL])
}

fn alg_rsa() -> Vec<u8> {
    der_seq(&[&der_oid(OID_RSA_ENCRYPTION), NULL])
}

/// `Name` (CN=Owen) — issuer and subject of the signing certificate.
fn name_cn() -> Vec<u8> {
    // X.509 Name = SEQUENCE OF (RDN) ; RDN = SET OF (ATV) ; ATV = SEQUENCE
    // { type OID, value }. Verified byte-for-byte against `openssl req -x509`
    // output — this exact triple nesting is what conscrypt demands (the
    // "APK غير صالح" / INSTALL_PARSE_FAILED_NO_CERTIFICATES family).
    der_seq(&[&der_set(&[&der_seq(&[&der_oid(OID_CN), &der_utf8("Owen")])])])
}

/// The certificate serial (Kotlin used the `ogfn` keystore alias; the value
/// only has to be stable across the v1/v2 blocks).
const CERT_SERIAL: u64 = 1;

/// Build a real self-signed X.509 v3 certificate for the signing key — the
/// same certificate is embedded by both the v1 (`CERT.RSA`) and v2 blocks.
pub fn self_signed_cert(private: &RsaPrivateKey) -> Vec<u8> {
    let public = RsaPublicKey::from(private);
    let spki = public
        .to_public_key_der()
        .map(|d| d.as_bytes().to_vec())
        .unwrap_or_default();
    let validity = der_seq(&[&der_utc("250101000000Z"), &der_utc("491231235959Z")]);
    let tbs = der_seq(&[
        &tlv(0xA0, &der_int(2)), // version v3, [0] EXPLICIT
        &der_int(CERT_SERIAL),
        &alg_sha256_rsa(),
        &name_cn(),
        &validity,
        &name_cn(),
        &spki,
    ]);
    let signature = sign_raw(private, &tbs);
    der_seq(&[&tbs, &alg_sha256_rsa(), &der_bit_string(&signature)])
}

fn sign_raw(private: &RsaPrivateKey, data: &[u8]) -> Vec<u8> {
    let signing = rsa::pkcs1v15::SigningKey::<Sha256>::new(private.clone());
    signing.sign(data).to_vec()
}

/* --------------------------- JAR manifest files -------------------------- */

fn sha256(data: &[u8]) -> Vec<u8> {
    Sha256::digest(data).to_vec()
}

/// Emit one header line with 72-byte wrapping (first line keeps the
/// `name: ` prefix, continuation lines start with a single space).
fn header_line(out: &mut Vec<u8>, name: &str, value: &str) {
    let prefix = format!("{name}: ");
    let value = value.as_bytes();
    let first_capacity = LINE_LIMIT.saturating_sub(prefix.len());
    let mut at = 0usize;
    let mut first = true;
    while at < value.len() {
        let take = if first {
            first_capacity.min(value.len() - at)
        } else {
            (LINE_LIMIT - 1).min(value.len() - at)
        };
        if first {
            out.extend_from_slice(prefix.as_bytes());
            first = false;
        } else {
            out.push(b' ');
        }
        out.extend_from_slice(&value[at..at + take]);
        out.extend_from_slice(b"\r\n");
        at += take;
    }
    if value.is_empty() {
        out.extend_from_slice(prefix.as_bytes());
        out.extend_from_slice(b"\r\n");
    }
}

/// One MANIFEST.MF section: `Name:` + `SHA-256-Digest:` + blank line
/// (the section bytes INCLUDE the terminating blank line, which is what the
/// CERT.SF section digests are computed over).
fn build_section(name: &str, digest_b64: &str) -> Vec<u8> {
    let mut out = Vec::new();
    header_line(&mut out, "Name", name);
    header_line(&mut out, "SHA-256-Digest", digest_b64);
    out.extend_from_slice(b"\r\n");
    out
}

/// Sign `unsigned` (a freshly repacked APK with no META-INF) with scheme v1,
/// writing the result to `output`.
pub fn sign_apk_v1(unsigned: &Path, output: &Path, key: &SigningKey) -> Result<(), String> {
    let zip = ZipArchive::open(unsigned)?;

    // MANIFEST.MF: main attributes + one section per (non META-INF) entry.
    let mut manifest = Vec::new();
    header_line(&mut manifest, "Manifest-Version", "1.0");
    header_line(&mut manifest, "Created-By", CREATED_BY);
    manifest.extend_from_slice(b"\r\n");
    let mut sections: Vec<(String, Vec<u8>)> = Vec::new();
    for entry in zip.list() {
        if entry.name.starts_with("META-INF/") || entry.name.ends_with('/') {
            continue;
        }
        let data = zip
            .read_entry(&entry.name)
            .ok_or_else(|| format!("v1: cannot read {}", entry.name))?;
        let digest = base64_encode(&sha256(&data));
        let section = build_section(&entry.name, &digest);
        manifest.extend_from_slice(&section);
        sections.push((entry.name.clone(), section));
    }

    // CERT.SF: digests OF the manifest (whole file + per section).
    let mut sf = Vec::new();
    header_line(&mut sf, "Signature-Version", "1.0");
    header_line(&mut sf, "Created-By", CREATED_BY);
    header_line(&mut sf, "SHA-256-Digest-Manifest", &base64_encode(&sha256(&manifest)));
    sf.extend_from_slice(b"\r\n");
    for (name, section) in &sections {
        let mut sec = Vec::new();
        header_line(&mut sec, "Name", name);
        header_line(&mut sec, "SHA-256-Digest", &base64_encode(&sha256(section)));
        sec.extend_from_slice(b"\r\n");
        sf.extend_from_slice(&sec);
    }

    // CERT.RSA: PKCS#7 SignedData over CERT.SF, same key as v2. The
    // embedded certificate comes from the key: the loaded `ogfn` pair when
    // present, the self-signed one otherwise.
    let cert_rsa = build_pkcs7(&sf, key);

    let mut out_zip = ZipArchive::open(unsigned)?;
    out_zip.remove_meta_inf();
    out_zip.add_deflated(MANIFEST_MF, &manifest);
    out_zip.add_deflated(CERT_SF, &sf);
    out_zip.add_deflated(CERT_RSA, &cert_rsa);
    out_zip.write_to(output)
}

/// Build the PKCS#7 `SignedData` DER for `CERT.RSA` (no signedAttrs — the
/// signature covers the content directly). The embedded certificate is
/// `key.cert_for_v1()` — the loaded `ogfn` DER when the pair was provided,
/// the self-signed one otherwise — and the SignerInfo's
/// issuerAndSerialNumber is taken from that same certificate so real
/// verifiers can match the signer to its cert.
fn build_pkcs7(content: &[u8], key: &SigningKey) -> Vec<u8> {
    let cert = key.cert_for_v1();
    let (issuer, serial) = cert_issuer_and_serial(&cert)
        .unwrap_or_else(|| (name_cn(), der_int(CERT_SERIAL)));
    let signature = sign_raw(&key.private, content);

    let encap = der_seq(&[
        &der_oid(OID_DATA),
        &tlv(0xA0, &der_octet(content)), // [0] EXPLICIT OCTET STRING
    ]);
    let certificates = tlv(0xA0, &cert); // [0] IMPLICIT SET OF Certificate
    let signer_info = der_seq(&[
        &der_int(1),
        &der_seq(&[&issuer, &serial]), // issuerAndSerialNumber
        &alg_sha256(),
        &alg_rsa(),
        &der_octet(&signature),
    ]);
    let signed_data = der_seq(&[
        &der_int(1), // version
        &der_set(&[&alg_sha256()]),
        &encap,
        &certificates,
        &der_set(&[&signer_info]),
    ]);
    der_seq(&[&der_oid(OID_SIGNED_DATA), &tlv(0xA0, &signed_data)])
}

/// Extract `(issuer Name TLV, serialNumber INTEGER TLV)` from a DER X.509
/// certificate, so the PKCS#7 SignerInfo can reference the embedded cert
/// exactly (Android locates the signer by issuer + serial).
fn cert_issuer_and_serial(cert: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    let mut outer = DerWalker::new(cert);
    let (t, cert_body) = outer.next()?;
    if t != 0x30 {
        return None;
    }
    // cert_body = [TBSCertificate TLV, signatureAlgorithm TLV, signature TLV]
    let mut cert_w = DerWalker::new(cert_body);
    let (t, tbs_body) = cert_w.next()?; // the TBSCertificate SEQUENCE
    if t != 0x30 {
        return None;
    }
    // tbs_body = [version [0]?, serialNumber, signature, issuer, validity, …]
    let mut tbs = DerWalker::new(tbs_body);
    let (first_tag, first) = tbs.next()?;
    let serial_content = if first_tag == 0xA0 {
        // version [0] EXPLICIT INTEGER — the serial follows it
        let (_, serial) = tbs.next()?;
        serial
    } else {
        first
    };
    let serial = tlv(0x02, serial_content);
    let (_t, _sig_alg) = tbs.next()?;
    let (t, issuer) = tbs.next()?;
    if t != 0x30 {
        return None;
    }
    Some((tlv(0x30, issuer), serial))
}

/* ------------------------------ verification ---------------------------- */

struct DerWalker<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> DerWalker<'a> {
    fn new(buf: &'a [u8]) -> Self {
        DerWalker { buf, pos: 0 }
    }

    /// Read one TLV; returns (tag, content).
    fn next(&mut self) -> Option<(u8, &'a [u8])> {
        let tag = *self.buf.get(self.pos)?;
        let len_byte = *self.buf.get(self.pos + 1)?;
        let (len, hdr) = if len_byte < 0x80 {
            (len_byte as usize, 2)
        } else {
            let n = (len_byte & 0x7F) as usize;
            if n == 0 || n > 4 {
                return None;
            }
            let mut len = 0usize;
            for i in 0..n {
                len = (len << 8) | *self.buf.get(self.pos + 2 + i)? as usize;
            }
            (len, 2 + n)
        };
        let start = self.pos + hdr;
        let content = self.buf.get(start..start + len)?;
        self.pos = start + len;
        Some((tag, content))
    }
}

/// Verify the v1 signature of an APK: manifest digests + SF digests + the
/// PKCS#7 RSA signature over CERT.SF.
pub fn verify_v1(path: &Path) -> Result<bool, String> {
    let zip = ZipArchive::open(path)?;
    let manifest = zip
        .read_entry(MANIFEST_MF)
        .ok_or("v1: no META-INF/MANIFEST.MF")?;
    let sf = zip.read_entry(CERT_SF).ok_or("v1: no META-INF/CERT.SF")?;
    let rsa_block = zip.read_entry(CERT_RSA).ok_or("v1: no META-INF/CERT.RSA")?;

    let (main_attrs, sections) = parse_jar_file(&manifest).ok_or("v1: bad MANIFEST.MF")?;
    let _ = main_attrs;

    // Every non-META-INF entry needs a section with a matching digest.
    for entry in zip.list() {
        if entry.name.starts_with("META-INF/") || entry.name.ends_with('/') {
            continue;
        }
        let Some(section) = sections.iter().find(|(n, _, _)| *n == entry.name) else {
            return Ok(false);
        };
        let data = zip.read_entry(&entry.name).ok_or("v1: read entry")?;
        if section.1 != base64_encode(&sha256(&data)) {
            return Ok(false);
        }
    }

    // CERT.SF: whole-manifest digest + per-section digests.
    let (sf_main, sf_sections) = parse_jar_file(&sf).ok_or("v1: bad CERT.SF")?;
    let Some(digest_manifest) = sf_main.iter().find(|(k, _)| k == "SHA-256-Digest-Manifest") else {
        return Ok(false);
    };
    if digest_manifest.1 != base64_encode(&sha256(&manifest)) {
        return Ok(false);
    }
    // CERT.SF section digests cover the MANIFEST.MF section BYTES (not the
    // file data — that is the MANIFEST.MF digest checked above).
    for (name, _digest, section_bytes) in &sections {
        let Some(sf_section) = sf_sections.iter().find(|(n, _, _)| n == name) else {
            return Ok(false);
        };
        if sf_section.1 != base64_encode(&sha256(section_bytes)) {
            return Ok(false);
        }
    }

    // PKCS#7: RSA signature over CERT.SF with the embedded certificate.
    verify_pkcs7(&rsa_block, &sf)
}

/// Parse a JAR manifest-style file: (main attributes, sections).
/// A section is (name, digest, raw section bytes including the blank line).
/// Continuation lines (a leading space) are unfolded into the previous
/// header's value, mirroring the JAR spec's 72-byte line wrapping.
fn parse_jar_file(data: &[u8]) -> Option<(Vec<(String, String)>, Vec<(String, String, Vec<u8>)>)> {
    // records: (attributes, raw_start, raw_end_including_the_blank_line)
    let mut records: Vec<(Vec<(String, String)>, usize, usize)> = Vec::new();
    let mut cur: Vec<(String, String)> = Vec::new();
    let mut cur_start: Option<usize> = None;
    let mut at = 0usize;
    while at < data.len() {
        let start = at;
        let crlf = find_crlf(data, at);
        let line_end = match crlf {
            Some(e) => e + 2,
            None => data.len(),
        };
        let raw_end = crlf.unwrap_or(data.len());
        let line = String::from_utf8_lossy(&data[start..raw_end]).into_owned();
        at = line_end;
        if line.is_empty() {
            let end = at;
            let s = cur_start.take().unwrap_or(end);
            records.push((std::mem::take(&mut cur), s, end));
            continue;
        }
        if cur_start.is_none() {
            cur_start = Some(start);
        }
        if let Some(rest) = line.strip_prefix(' ') {
            if let Some(last) = cur.last_mut() {
                last.1.push_str(rest);
                continue;
            }
        }
        if let Some((k, v)) = parse_header(&line) {
            cur.push((k, v));
        }
    }
    if !cur.is_empty() {
        let s = cur_start.unwrap_or(data.len());
        records.push((cur, s, data.len()));
    }

    let mut main = Vec::new();
    let mut sections = Vec::new();
    for (i, (attrs, start, end)) in records.into_iter().enumerate() {
        if i == 0 {
            main = attrs;
            continue;
        }
        let name = attrs.iter().find(|(k, _)| k == "Name").map(|(_, v)| v.clone())?;
        let digest = attrs
            .iter()
            .find(|(k, _)| k == "SHA-256-Digest")
            .map(|(_, v)| v.clone())?;
        sections.push((name, digest, data[start..end].to_vec()));
    }
    Some((main, sections))
}

fn find_crlf(data: &[u8], from: usize) -> Option<usize> {
    let mut i = from;
    while i + 1 < data.len() {
        if data[i] == b'\r' && data[i + 1] == b'\n' {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// One header line (continuation lines already unfolded by being on one line
/// in the buffers WE produce; wrapped values from other signers are joined
/// by `verify` readers — this parser handles the raw single-line form and
/// the 72-byte wrapped form by joining continuation lines).
fn parse_header(line: &str) -> Option<(String, String)> {
    let colon = line.find(':')?;
    let key = line[..colon].to_string();
    let value = line[colon + 1..].strip_prefix(' ').unwrap_or(&line[colon + 1..]).to_string();
    Some((key, value))
}

fn verify_pkcs7(block: &[u8], expected_content: &[u8]) -> Result<bool, String> {
    let mut walker = DerWalker::new(block);
    let (tag, content_info) = walker.next().ok_or("v1: pkcs7 outer")?;
    if tag != 0x30 {
        return Ok(false);
    }
    let mut ci = DerWalker::new(content_info);
    let (t, oid) = ci.next().ok_or("v1: pkcs7 oid")?;
    if t != 0x06 || oid != OID_SIGNED_DATA {
        return Ok(false);
    }
    let (t, sd_raw) = ci.next().ok_or("v1: pkcs7 body")?;
    if t != 0xA0 {
        return Ok(false);
    }
    // [0] EXPLICIT wraps the full SignedData SEQUENCE TLV — unwrap it.
    let mut sd_outer = DerWalker::new(sd_raw);
    let (t, sd_seq) = sd_outer.next().ok_or("v1: signedData seq")?;
    if t != 0x30 {
        return Ok(false);
    }
    let mut sd = DerWalker::new(sd_seq);
    let (_t, _version) = sd.next().ok_or("v1: version")?;
    let (_t, _digest_algs) = sd.next().ok_or("v1: digestAlgs")?;
    let (t, encap) = sd.next().ok_or("v1: encapContentInfo")?;
    if t != 0x30 {
        return Ok(false);
    }
    let mut ec = DerWalker::new(encap);
    let (t, _oid) = ec.next().ok_or("v1: eContentType")?;
    if t != 0x06 {
        return Ok(false);
    }
    let (t, content_tlv) = ec.next().ok_or("v1: eContent")?;
    if t != 0xA0 {
        return Ok(false);
    }
    let mut cw = DerWalker::new(content_tlv);
    let (t, content) = cw.next().ok_or("v1: content octets")?;
    if t != 0x04 {
        return Ok(false);
    }
    if content != expected_content {
        return Ok(false);
    }
    let (_t, _certs) = sd.next().ok_or("v1: certificates")?;
    let (t, signer_infos) = sd.next().ok_or("v1: signerInfos")?;
    if t != 0x31 {
        return Ok(false);
    }
    let mut si_set = DerWalker::new(signer_infos);
    let (t, signer_info) = si_set.next().ok_or("v1: signerInfo")?;
    if t != 0x30 {
        return Ok(false);
    }
    let mut si = DerWalker::new(signer_info);
    let (_t, _ver) = si.next().ok_or("v1: si version")?;
    let (_t, _sid) = si.next().ok_or("v1: sid")?;
    let (_t, _digest_alg) = si.next().ok_or("v1: si digest alg")?;
    let (_t, _sig_alg) = si.next().ok_or("v1: si sig alg")?;
    let (t, signature) = si.next().ok_or("v1: si signature")?;
    if t != 0x04 {
        return Ok(false);
    }

    // The public key comes from the embedded certificate's SPKI.
    let public = extract_cert_public_key(_certs).ok_or("v1: cert parse")?;
    let verifying = rsa::pkcs1v15::VerifyingKey::<Sha256>::new(public);
    let Ok(sig) = rsa::pkcs1v15::Signature::try_from(signature) else {
        return Ok(false);
    };
    Ok(verifying.verify(content, &sig).is_ok())
}

pub(crate) fn extract_cert_public_key(certs: &[u8]) -> Option<RsaPublicKey> {
    let mut walker = DerWalker::new(certs);
    let (t, _cert) = walker.next()?;
    if t != 0x30 {
        return None;
    }
    let mut c = DerWalker::new(_cert);
    let (_t, tbs) = c.next()?;
    let mut t = DerWalker::new(tbs);
    // version [0], serial, signature, issuer, validity, subject, SPKI
    let _ = t.next()?; // [0] version
    let _ = t.next()?; // serial
    let _ = t.next()?; // signature alg
    let _ = t.next()?; // issuer
    let _ = t.next()?; // validity
    let _ = t.next()?; // subject
    let (tag, spki_content) = t.next()?;
    if tag != 0x30 {
        return None;
    }
    // `spki_content` is the INSIDE of the SPKI SEQUENCE; the decoder wants
    // the full TLV.
    use rsa::pkcs8::DecodePublicKey;
    RsaPublicKey::from_public_key_der(&tlv(0x30, spki_content)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sign;
    use crate::ziprw;
    use std::io::Write;

    fn build_zip(path: &std::path::Path) {
        let mut out: Vec<u8> = Vec::new();
        let mut central: Vec<u8> = Vec::new();
        let mut offset: u32 = 0;
        // (name, data, stored?) — assets/x.txt is STORED so the tamper test
        // can flip its raw payload byte directly.
        for (name, data, stored) in [
            ("classes.dex", b"dex-payload".to_vec(), false),
            ("assets/x.txt", b"hello v1".to_vec(), true),
        ] {
            let crc = ziprw::crc32(&data);
            let name_b = name.as_bytes();
            let (raw, method): (Vec<u8>, u16) = if stored {
                (data.clone(), 0)
            } else {
                let mut enc = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::new(6));
                enc.write_all(&data).unwrap();
                (enc.finish().unwrap(), 8)
            };
            let mut lh = vec![0u8; 30];
            lh[0..4].copy_from_slice(&0x0403_4b50u32.to_le_bytes());
            lh[8..10].copy_from_slice(&method.to_le_bytes());
            lh[14..18].copy_from_slice(&crc.to_le_bytes());
            lh[18..22].copy_from_slice(&(raw.len() as u32).to_le_bytes());
            lh[22..26].copy_from_slice(&(data.len() as u32).to_le_bytes());
            lh[26..28].copy_from_slice(&(name_b.len() as u16).to_le_bytes());
            out.extend_from_slice(&lh);
            out.extend_from_slice(name_b);
            out.extend_from_slice(&raw);
            let mut ch = vec![0u8; 46];
            ch[0..4].copy_from_slice(&0x0201_4b50u32.to_le_bytes());
            ch[10..12].copy_from_slice(&method.to_le_bytes());
            ch[16..20].copy_from_slice(&crc.to_le_bytes());
            ch[20..24].copy_from_slice(&(raw.len() as u32).to_le_bytes());
            ch[24..28].copy_from_slice(&(data.len() as u32).to_le_bytes());
            ch[28..30].copy_from_slice(&(name_b.len() as u16).to_le_bytes());
            ch[42..46].copy_from_slice(&offset.to_le_bytes());
            central.extend_from_slice(&ch);
            central.extend_from_slice(name_b);
            offset += (30 + name_b.len() + raw.len()) as u32;
        }
        let cd_offset = offset;
        let cd_size = central.len() as u32;
        out.extend_from_slice(&central);
        let mut eocd = [0u8; 22];
        eocd[0..4].copy_from_slice(&0x0605_4b50u32.to_le_bytes());
        eocd[8..10].copy_from_slice(&2u16.to_le_bytes());
        eocd[10..12].copy_from_slice(&2u16.to_le_bytes());
        eocd[12..16].copy_from_slice(&cd_size.to_le_bytes());
        eocd[16..20].copy_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&eocd);
        std::fs::write(path, out).unwrap();
    }

    #[test]
    fn base64_round_trip() {
        for data in [&b""[..], b"f", b"fo", b"foo", b"foob", b"fooba", b"foobar"] {
            let enc = base64_encode(data);
            assert_eq!(base64_decode(&enc).as_deref(), Some(data), "{enc}");
        }
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
    }

    #[test]
    fn sign_and_verify_v1() {
        let dir = std::env::temp_dir().join(format!("owen-v1-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let unsigned = dir.join("unsigned.apk");
        build_zip(&unsigned);

        let key = SigningKey::generate().expect("keygen");
        let signed = dir.join("signed.apk");
        sign_apk_v1(&unsigned, &signed, &key).expect("sign v1");

        let zip = ZipArchive::open(&signed).unwrap();
        assert!(zip.has(MANIFEST_MF) && zip.has(CERT_SF) && zip.has(CERT_RSA));
        let mf = zip.read_entry(MANIFEST_MF).unwrap();
        let text = String::from_utf8_lossy(&mf);
        assert!(text.contains("Manifest-Version: 1.0"));
        assert!(text.contains("Name: classes.dex"));
        assert!(text.contains("Name: assets/x.txt"));

        assert!(verify_v1(&signed).unwrap_or(false), "v1 must verify");

        // Tamper with an entry payload → digests mismatch.
        // Tamper with the STORED entry payload → digest mismatch.
        let mut raw = std::fs::read(&signed).unwrap();
        let needle = b"hello v1";
        let pos = raw
            .windows(needle.len())
            .position(|w| w == needle)
            .expect("stored payload present");
        raw[pos] ^= 0xFF;
        let tampered = dir.join("tampered.apk");
        std::fs::write(&tampered, raw).unwrap();
        assert!(!verify_v1(&tampered).unwrap_or(false), "tampered v1 must fail");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cert_is_parseable_der() {
        let key = SigningKey::generate().unwrap();
        let cert = self_signed_cert(&key.private);
        let mut walker = DerWalker::new(&cert);
        let (t, _inner) = walker.next().unwrap();
        assert_eq!(t, 0x30, "certificate must be a DER SEQUENCE");
        assert!(extract_cert_public_key(&cert).is_some());
        // the self-signed cert's issuer/serial extract cleanly too
        let (issuer, serial) = cert_issuer_and_serial(&cert).unwrap();
        assert_eq!(issuer, name_cn());
        assert_eq!(serial, der_int(CERT_SERIAL));
    }

    /// The `ogfn` pair (assets/signing/) must sign + verify under BOTH
    /// schemes, with `CERT.RSA` embedding the original 873-byte certificate
    /// (not the self-signed one) and the v2 block carrying the same cert.
    #[test]
    fn sign_and_verify_with_ogfn_pair() {
        let key_pem = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/signing/ogfn-key.pem");
        let cert_der_path =
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/signing/ogfn-cert.der");
        if !std::path::Path::new(key_pem).exists() || !std::path::Path::new(cert_der_path).exists()
        {
            eprintln!("skip: {key_pem} pair not present");
            return;
        }
        let ogfn_cert = std::fs::read(cert_der_path).unwrap();
        let key = SigningKey::load_pair(key_pem, cert_der_path).expect("ogfn pair loads");
        assert_eq!(key.certificate_der(), ogfn_cert, "v2 embeds the ogfn cert");
        assert_eq!(key.cert_for_v1(), ogfn_cert, "v1 embeds the ogfn cert");

        let dir = std::env::temp_dir().join(format!("owen-ogfn-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let unsigned = dir.join("unsigned.apk");
        build_zip(&unsigned);

        // v1 with the ogfn pair
        let signed = dir.join("ogfn-signed.apk");
        sign_apk_v1(&unsigned, &signed, &key).expect("sign v1 (ogfn)");
        let zip = ZipArchive::open(&signed).unwrap();
        let cert_rsa_ogfn = zip.read_entry(CERT_RSA).unwrap();
        assert!(
            cert_rsa_ogfn.windows(ogfn_cert.len()).any(|w| w == ogfn_cert),
            "CERT.RSA embeds the original ogfn certificate DER"
        );

        // Same zip signed with a self-signed key: different CERT.RSA length
        // (873-byte ogfn cert vs the self-signed one) and no ogfn bytes.
        let self_key = SigningKey::generate().unwrap();
        let signed_self = dir.join("self-signed.apk");
        sign_apk_v1(&unsigned, &signed_self, &self_key).expect("sign v1 (self)");
        let zip_self = ZipArchive::open(&signed_self).unwrap();
        let cert_rsa_self = zip_self.read_entry(CERT_RSA).unwrap();
        assert_ne!(
            cert_rsa_ogfn.len(),
            cert_rsa_self.len(),
            "CERT.RSA length changes with the ogfn cert ({} vs {})",
            cert_rsa_ogfn.len(),
            cert_rsa_self.len()
        );
        assert!(!cert_rsa_self.windows(ogfn_cert.len()).any(|w| w == ogfn_cert));

        // v2 on top of the ogfn v1, then verify both schemes
        sign::sign_apk_v2(&signed, &key).expect("sign v2 (ogfn)");
        assert!(verify_v1(&signed).unwrap_or(false), "ogfn v1 must verify");
        assert!(sign::verify_v2(&signed).unwrap_or(false), "ogfn v2 must verify");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// A mismatched key/cert pair must be rejected by `load_pair`.
    #[test]
    fn load_pair_rejects_mismatched_cert() {
        let key_pem = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/signing/ogfn-key.pem");
        let cert_der_path =
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/signing/ogfn-cert.der");
        if !std::path::Path::new(key_pem).exists() || !std::path::Path::new(cert_der_path).exists()
        {
            eprintln!("skip: ogfn pair not present");
            return;
        }
        let wrong = SigningKey::generate().unwrap();
        let dir = std::env::temp_dir().join(format!("owen-mismatch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let wrong_cert = dir.join("wrong.der");
        std::fs::write(&wrong_cert, self_signed_cert(&wrong.private)).unwrap();
        assert!(SigningKey::load_pair(key_pem, wrong_cert.to_str().unwrap()).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}

#[cfg(test)]
mod cert_dump {
    #[test]
    fn dump_self_signed_for_asn1_inspection() {
        let key = crate::sign::SigningKey::generate_or_load("/tmp/owen-inspect-key.pem").unwrap();
        let der = super::self_signed_cert(&key.private);
        std::fs::write("/tmp/owen-selfsigned.der", &der).unwrap();
        eprintln!("wrote /tmp/owen-selfsigned.der ({} bytes)", der.len());
    }
}
