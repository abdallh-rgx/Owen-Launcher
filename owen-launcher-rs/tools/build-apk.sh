#!/usr/bin/env bash
#
# tools/build-apk.sh — build the pure-Rust Owen Launcher APK
# (dist/OwenLauncher-v1.8.0-release.apk) end to end, in the foreground:
#
#   1. cargo build the redirecter (libowen.so) for aarch64-linux-android
#   2. cargo build the app (libowen_app.so) — RETRIES while the concurrent
#      T5-JNI agent is still writing crates/owen-app/src/jni_android.rs
#      (errors mentioning jni_android → sleep 90, retry, 15 min total;
#      errors in any OTHER owen-app file abort immediately)
#   3. wait for assets/dex/classes.dex (same agent, same retry budget)
#   4. extract assets/tools/ogfn-debug.keystore from git (once)
#   5. cargo run the packer (assemble + v1/v2 sign with the ogfn pair)
#   6. self-verify: python zipfile testzip + entry count + alignment walk
#
# Environment:
#   OWEN_ALLOW_MISSING_DEX=1  pack without classes.dex (smoke build only)
#   ANDROID_NDK_HOME          CI-provided NDK (local .cargo/config.toml
#                             already points at /home/z/ndk)

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# shellcheck disable=SC1090,SC1091
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"

TARGET=aarch64-linux-android
DEADLINE=$((SECONDS + 900)) # 15 minutes for the concurrent-agent blockers

log() { printf '\033[1;36m[build-apk]\033[0m %s\n' "$*"; }
die() { printf '\033[1;31m[build-apk] FATAL:\033[0m %s\n' "$*" >&2; exit 1; }

# --- NDK / linker setup (local .cargo/config.toml already has the absolute
# linker; in CI (setup-ndk) write the relative one and put the toolchain on
# PATH). -------------------------------------------------------------------
if [ -n "${ANDROID_NDK_HOME:-}" ] && [ -d "${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/linux-x86_64/bin" ]; then
    if ! grep -q "aarch64-linux-android" .cargo/config.toml 2>/dev/null; then
        mkdir -p .cargo
        printf '[target.aarch64-linux-android]\nlinker = "aarch64-linux-android24-clang"\n' > .cargo/config.toml
    fi
    export PATH="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH"
    export CC_aarch64_linux_android=aarch64-linux-android24-clang
    export AR_aarch64_linux_android=llvm-ar
fi

# --- 1. the redirecter (nothing blocks this) -------------------------------
log "building owen-redirecter ($TARGET, release)…"
cargo build --release -p owen-redirecter --target "$TARGET"
REDIRECTER_SO="target/$TARGET/release/libowen.so"
[ -f "$REDIRECTER_SO" ] || die "redirecter output $REDIRECTER_SO missing"
log "redirecter ok: $REDIRECTER_SO ($(stat -c%s "$REDIRECTER_SO") bytes)"

# --- 2. the app (libowen_app.so) — tolerate the concurrent jni_android.rs --
log "building owen-app ($TARGET, release)… [retries while jni_android.rs is in flight]"
while true; do
    if cargo build --release -p owen-app --target "$TARGET" 2> >(tee /tmp/owen-app-build.err >&2); then
        break
    fi
    if grep -q "jni_android" /tmp/owen-app-build.err; then
        if [ "$SECONDS" -ge "$DEADLINE" ]; then
            die "owen-app still failing inside jni_android.rs after 15 min (T5-JNI agent?)"
        fi
        log "jni_android.rs errors (concurrent agent) — sleeping 90s, retrying…"
        sleep 90
    else
        die "owen-app failed with errors OUTSIDE jni_android.rs — not retrying:
$(tail -40 /tmp/owen-app-build.err)"
    fi
done
APP_SO="$(ls -1 target/$TARGET/release/libowen_app.so target/$TARGET/release/libowenapp.so 2>/dev/null | head -1 || true)"
[ -n "$APP_SO" ] || die "owen-app produced no libowen_app.so in target/$TARGET/release/"
log "app ok: $APP_SO ($(stat -c%s "$APP_SO") bytes)"

# --- 3. classes.dex (same T5-JNI agent) -------------------------------------
DEX="assets/dex/classes.dex"
if [ ! -f "$DEX" ]; then
    log "waiting for $DEX (T5-JNI agent)…"
    while [ ! -f "$DEX" ]; do
        if [ "$SECONDS" -ge "$DEADLINE" ]; then
            if [ "${OWEN_ALLOW_MISSING_DEX:-0}" = "1" ]; then
                log "WARNING: $DEX never appeared — packing WITHOUT it (smoke build)"
                break
            fi
            die "$DEX never appeared after 15 min (T5-JNI agent?)"
        fi
        sleep 15
    done
fi
[ -f "$DEX" ] && log "dex ok: $DEX ($(stat -c%s "$DEX") bytes)"

# --- 4. the ogfn keystore (runtime asset; extracted from git once) ----------
KEYSTORE="assets/tools/ogfn-debug.keystore"
if [ ! -f "$KEYSTORE" ]; then
    log "extracting $KEYSTORE from git…"
    mkdir -p assets/tools
    REPO="$(git rev-parse --show-toplevel 2>/dev/null || printf '%s' "$ROOT/..")"
    if git -C "$REPO" show owen/main:app/src/main/assets/tools/ogfn-debug.keystore > "$KEYSTORE" 2>/dev/null \
        || git -C "$REPO" show origin/main:app/src/main/assets/tools/ogfn-debug.keystore > "$KEYSTORE" 2>/dev/null \
        || git -C "$REPO" show main:app/src/main/assets/tools/ogfn-debug.keystore > "$KEYSTORE" 2>/dev/null; then
        log "keystore extracted: $KEYSTORE ($(stat -c%s "$KEYSTORE") bytes)"
    else
        rm -f "$KEYSTORE"
        die "cannot extract ogfn-debug.keystore (need owen/main or origin/main in git)"
    fi
fi

# --- 5. assemble + sign ------------------------------------------------------
log "building the packer and assembling the APK…"
PACK_ARGS=(--root "$ROOT")
[ "${OWEN_ALLOW_MISSING_DEX:-0}" = "1" ] && [ ! -f "$DEX" ] && PACK_ARGS+=(--allow-missing-dex)
cargo build --release -p owen-patcher --bin pack
./target/release/pack "${PACK_ARGS[@]}"
APK="dist/OwenLauncher-v1.8.0-release.apk"
[ -f "$APK" ] || die "packer reported success but $APK is missing"

# --- 6. independent verification (python zipfile + struct walk) -------------
log "python3 verification of $APK…"
APK="$APK" python3 - <<'EOF'
import os, struct, zipfile, zlib, sys

apk = os.environ["APK"]
z = zipfile.ZipFile(apk)
bad = z.testzip()
assert bad is None, f"CRC failure: {bad}"
names = z.namelist()
print(f"  zipfile.testzip : OK (all CRCs valid)")
print(f"  entries         : {len(names)}")
for required in [
    "AndroidManifest.xml",
    "resources.arsc",
    "classes.dex",
    "lib/arm64-v8a/libowen.so",
    "lib/arm64-v8a/libowen_app.so",
    "assets/tools/ogfn-debug.keystore",
    "assets/signing/ogfn-key.pem",
    "assets/signing/ogfn-cert.der",
    "META-INF/MANIFEST.MF",
    "META-INF/CERT.SF",
    "META-INF/CERT.RSA",
]:
    assert required in names, f"missing entry: {required}"
raw = open(apk, "rb").read()
problems = []
for i in z.infolist():
    nlen, elen = struct.unpack_from("<HH", raw, i.header_offset + 26)
    data_off = i.header_offset + 30 + nlen + elen
    if i.filename.startswith("lib/") and i.filename.endswith(".so"):
        if i.compress_type != 0:
            problems.append(f"{i.filename}: not STORED")
        if data_off % 16384:
            problems.append(f"{i.filename}: data offset {data_off} not 16384-aligned")
    if i.filename in ("AndroidManifest.xml", "resources.arsc"):
        if i.compress_type != 0:
            problems.append(f"{i.filename}: not STORED")
        if data_off % 4:
            problems.append(f"{i.filename}: data offset {data_off} not 4-aligned")
assert not problems, "alignment problems: " + "; ".join(problems)
n_seasons = sum(1 for n in names if n.startswith("assets/images/season_") and n.endswith("_folder.png"))
print(f"  season images   : {n_seasons}")
print(f"  alignment       : OK (.so %% 16384, manifest/arsc %% 4, all STORED)")
EOF

log "APK READY: $APK"
ls -la dist/
