#!/usr/bin/env python3
"""AOSP-accurate APK Signature Scheme v2 verifier (independent oracle).

Ported from frameworks/base ApkSigningBlockUtils.computeContentDigestsPer1MbChunk
+ ApkSignatureSchemeV2Verifier. Used to validate owen-patcher's v2 output BEFORE
pushing it at a device (each CI round-trip costs ~20 minutes).

usage: python3 tools/ci/v2oracle.py <apk>
exit 0 = the v2 block verifies per AOSP rules; prints the signer details.
"""
import hashlib
import struct
import sys

CHUNK = 1 << 20
V2_ID = 0x7109871A


def u16(b, o): return struct.unpack_from("<H", b, o)[0]
def u32(b, o): return struct.unpack_from("<I", b, o)[0]
def u64(b, o): return struct.unpack_from("<Q", b, o)[0]


def find_eocd(d):
    for i in range(len(d) - 22, -1, -1):
        if d[i:i + 4] == b"PK\x05\x06":
            return i
    raise SystemExit("no EOCD")


def digest_chunk(c):
    return hashlib.sha256(b"\xa5" + struct.pack("<I", len(c)) + c).digest()


def content_digest(regions):
    """AOSP: per-segment 1MiB chunks; top = sha256(0x5a||u32(count)||digests)."""
    chunks = []
    for r in regions:
        off = 0
        while len(r) - off > CHUNK:
            chunks.append(r[off:off + CHUNK]); off += CHUNK
        if len(r) - off:
            chunks.append(r[off:])
    top = b"\x5a" + struct.pack("<I", len(chunks))
    for c in chunks:
        top += digest_chunk(c)
    return hashlib.sha256(top).digest()


def verify(path):
    d = open(path, "rb").read()
    eocd = find_eocd(d)
    cd_off = u32(d, eocd + 16)
    cd_size = u32(d, eocd + 12)
    if cd_off < 32 or d[cd_off - 16:cd_off] != b"APK Sig Block 42":
        raise SystemExit(f"{path}: no APK Signing Block (cd_off=0x{cd_off:x})")
    block_size = u64(d, cd_off - 24)          # footer size (excludes leading 8)
    block_start = cd_off - 8 - block_size     # start of the leading size field
    # v2 pair
    pair_off = block_start + 8
    pair_len = u64(d, pair_off)
    pair_end = pair_off + pair_len
    o = pair_off + 8
    if u32(d, o) != V2_ID:
        raise SystemExit(f"first pair is not v2 (id=0x{u32(d, o):08x})")
    o += 4
    signers_len = u32(d, o); o += 4
    end = o + signers_len
    o += 4  # first signer length
    # signed data
    sd_len = u32(d, o); o += 4
    sd = d[o:o + sd_len]; o += sd_len
    # signatures
    sigs_len = u32(d, o); o += 4
    sigs = d[o:o + sigs_len]; o += sigs_len
    # pubkey
    pk_len = u32(d, o); o += 4
    pk = d[o:o + pk_len]; o += pk_len

    # signed data: [digests seq][certificates seq][attrs]
    cur = 0
    L = u32(sd, cur); cur += 4
    digests_blob = sd[cur:cur + L]; cur += L
    L = u32(sd, cur); cur += 4
    certs_blob = sd[cur:cur + L]; cur += L

    # first digest record inside digests_blob: [u32 rec_len][u32 algo][u32 dlen][digest]
    r = 4
    algo = u32(digests_blob, r); r += 4
    dlen = u32(digests_blob, r); r += 4
    expected = digests_blob[r:r + dlen]

    # first certificate: [u32 c1len][DER]
    c1 = u32(certs_blob, 0)
    cert = certs_blob[4:4 + c1]

    # AOSP regions: [0, blockStart) + [cdOff, eocd) + EOCD with cd-offset = blockStart
    import io
    eocd_bytes = bytearray(d[eocd:eocd + 22])
    struct.pack_into("<I", eocd_bytes, 16, block_start)
    actual = content_digest([d[:block_start], d[cd_off:eocd], bytes(eocd_bytes)])

    ok = expected == actual
    print(f"block: start=0x{block_start:x} size={block_size} cd=0x{cd_off:x} eocd=0x{eocd:x}")
    print(f"algo=0x{algo:08x} expected={expected.hex()[:24]}… actual={actual.hex()[:24]}… → {'MATCH' if ok else 'MISMATCH'}")
    # certificates sanity (must parse as X.509 DER) — `cert` extracted above.
    try:
        from cryptography import x509
        c = x509.load_der_x509_certificate(cert)
        print(f"cert: subject={c.subject.rfc4514_string()} — parses")
    except ImportError:
        # openssl fallback
        import subprocess, tempfile, os
        with tempfile.NamedTemporaryFile(suffix=".der", delete=False) as f:
            f.write(cert); tmp = f.name
        r = subprocess.run(["openssl", "x509", "-inform", "DER", "-in", tmp, "-noout", "-subject"],
                           capture_output=True, text=True)
        os.unlink(tmp)
        print("cert:", (r.stdout or r.stderr).strip() or "PARSE FAILED")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(verify(sys.argv[1]))
