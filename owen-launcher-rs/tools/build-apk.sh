#!/usr/bin/env bash
#
# tools/build-apk.sh — build the Owen Launcher APK
# (dist/OwenLauncher-v1.9.0-release.apk) end to end, in the foreground:
#
#   1. cargo build the redirecter (libowen.so) for aarch64-linux-android
#   2. cargo build the app (libowen_app.so) — RETRIES while the concurrent
#      T5-JNI agent is still writing crates/owen-app/src/jni_android.rs
#      (errors mentioning jni_android → sleep 90, retry, 15 min total;
#      errors in any OTHER owen-app file abort immediately)
#   3. REAL Android build-tooling phase (aapt2 + javac + d8) from the SDK:
#        aapt2 compile --dir app/res                → app/build/res.zip
#        aapt2 link --manifest app/AndroidManifest.xml
#                    -I android.jar -R res.zip      → app/build/base.apk
#        (extract binary AndroidManifest.xml + resources.arsc from base.apk)
#        javac -source/-target 1.8 -bootclasspath android.jar app/src/**
#                                                 → app/build/classes/
#        d8 --release --lib android.jar --min-api 24 → app/build/classes.dex
#      OWEN_ALLOW_HANDCRAFTED=1 skips this phase and falls back to the legacy
#      handcrafted manifest/arsc (axmlwriter) + assets/dex/classes.dex — that
#      path is NOT device-verified and exists only for SDK-less smoke builds.
#   4. extract assets/tools/ogfn-debug.keystore from git (once)
#   5. cargo build the packer; owen-patcher --bin pack assembles the zip and
#      signs it v1+v2 with the ogfn pair, injecting the prebuilt
#      manifest/arsc/dex (same entry names, same alignment rules)
#   6. VERIFY (any failure → exit 1):
#        - python3 zipfile: CRCs, required entries, alignment walk
#        - apksigner verify --verbose --print-certs: v1 AND v2 must be true
#        - aapt dump badging: package + NativeActivity + versionCode 19
#        - zipalign -c: 4-byte alignment
#        - a human-readable size + entry-count table
#
# Environment:
#   ANDROID_HOME / ANDROID_SDK_ROOT  SDK location (fallbacks:
#                                    /usr/local/lib/android/sdk — the GitHub
#                                    runner path — then ~/android-sdk,
#                                    ~/Android/Sdk)
#   OWEN_ALLOW_HANDCRAFTED=1         skip the aapt2/javac/d8 phase, use the
#                                    legacy handcrafted inputs (smoke only)
#   OWEN_ALLOW_MISSING_DEX=1         (handcrafted path only) pack without
#                                    classes.dex
#   ANDROID_NDK_HOME                 CI-provided NDK (local .cargo/config.toml
#                                    already points at /home/z/ndk)

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# shellcheck disable=SC1090,SC1091
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"

TARGET=aarch64-linux-android
DEADLINE=$((SECONDS + 900)) # 15 minutes for the concurrent-agent blockers
APK="dist/OwenLauncher-v1.9.0-release.apk"

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
    export CXX_aarch64_linux_android=aarch64-linux-android24-clang++
    export AR_aarch64_linux_android=llvm-ar
    # The NDK sysroot ships no C library headers; the shim in tools/ndk-include
    # provides the handful ring needs (assert.h/string.h/stdlib.h).
    export CFLAGS_aarch64_linux_android="-I$ROOT/tools/ndk-include"
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

# --- 3. the REAL Android build-tooling phase (aapt2 + javac + d8) -----------
# Produces app/build/{AndroidManifest.xml, resources.arsc, classes.dex} and
# feeds them to the packer as prebuilts (phase 5).
PACK_ARGS=(--root "$ROOT")
BUILD_TOOLS=""   # set by the SDK phase; empty on the handcrafted fallback
if [ "${OWEN_ALLOW_HANDCRAFTED:-0}" = "1" ]; then
    log "WARNING: OWEN_ALLOW_HANDCRAFTED=1 — skipping aapt2/javac/d8 and using the"
    log "         handcrafted axmlwriter manifest + empty arsc + assets/dex/classes.dex."
    log "         That path is NOT device-verified; do not ship it."
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
    if [ -f "$DEX" ]; then
        log "dex ok: $DEX ($(stat -c%s "$DEX") bytes)"
    elif [ "${OWEN_ALLOW_MISSING_DEX:-0}" = "1" ]; then
        PACK_ARGS+=(--allow-missing-dex)
    fi
else
    # --- 3a. locate the SDK (GitHub runners: /usr/local/lib/android/sdk) ----
    SDK=""
    for candidate in "${ANDROID_HOME:-}" "${ANDROID_SDK_ROOT:-}" \
                     /usr/local/lib/android/sdk "$HOME/android-sdk" "$HOME/Android/Sdk"; do
        if [ -n "$candidate" ] && [ -d "$candidate/build-tools" ]; then
            SDK="$candidate"
            break
        fi
    done
    [ -n "$SDK" ] || die "no Android SDK found (tried \$ANDROID_HOME, \$ANDROID_SDK_ROOT, \
/usr/local/lib/android/sdk, ~/android-sdk, ~/Android/Sdk). Install build-tools + platforms, \
or set OWEN_ALLOW_HANDCRAFTED=1 for a smoke build."

    # newest build-tools dir (e.g. 34.0.0)
    BT_VERSION="$(ls -1 "$SDK/build-tools" 2>/dev/null | sort -V | tail -n1)"
    [ -n "$BT_VERSION" ] || die "no build-tools in $SDK/build-tools"
    BUILD_TOOLS="$SDK/build-tools/$BT_VERSION"
    for tool in aapt2 aapt d8 apksigner zipalign; do
        [ -x "$BUILD_TOOLS/$tool" ] || die "missing $tool in $BUILD_TOOLS"
    done
    command -v javac >/dev/null 2>&1 || die "javac not on PATH (a JDK is required for the real-tooling build)"
    command -v java  >/dev/null 2>&1 || die "java not on PATH (d8/apksigner need it)"

    # android.jar: prefer platforms/android-34 (the manifest's targetSdk),
    # else the newest platform available.
    if [ -f "$SDK/platforms/android-34/android.jar" ]; then
        ANDROID_JAR="$SDK/platforms/android-34/android.jar"
    else
        ANDROID_JAR="$(ls -d "$SDK"/platforms/android-* 2>/dev/null | sort -V | tail -n1)/android.jar"
    fi
    [ -f "$ANDROID_JAR" ] || die "no android.jar platform in $SDK/platforms (need android-34 or newer)"
    log "SDK: $SDK (build-tools $BT_VERSION, platform $(basename "$(dirname "$ANDROID_JAR")"))"

    rm -rf app/build
    mkdir -p app/build

    # --- 3b. aapt2: compile the resources, link the binary manifest --------
    log "aapt2 compile app/res…"
    "$BUILD_TOOLS/aapt2" compile --dir app/res -o app/build/res.zip
    # NOTE: the compiled resources zip goes in POSITIONALLY (the canonical
    # AGP-style invocation — aapt2 link accepts .flat/.flata/.apk inputs as
    # positional args). -R is for extra symbol TABLES (R.txt/.ap_), not flatas.
    log "aapt2 link app/AndroidManifest.xml → app/build/base.apk…"
    "$BUILD_TOOLS/aapt2" link \
        -o app/build/base.apk \
        --manifest app/AndroidManifest.xml \
        -I "$ANDROID_JAR" \
        --auto-add-overlay \
        app/build/res.zip
    [ -f app/build/base.apk ] || die "aapt2 link produced no base.apk"

    # extract the binary AndroidManifest.xml + resources.arsc bytes (python3
    # — no unzip dependency; handles STORED and DEFLATED entries alike)
    python3 - app/build/base.apk app/build <<'PYEOF'
import sys, zipfile

apk, out = sys.argv[1], sys.argv[2]
z = zipfile.ZipFile(apk)
for name in ("AndroidManifest.xml", "resources.arsc"):
    data = z.read(name)
    with open(f"{out}/{name}", "wb") as f:
        f.write(data)
    print(f"  extracted {name}: {len(data)} bytes")
PYEOF
    [ -s app/build/AndroidManifest.xml ] || die "AndroidManifest.xml missing from the aapt2 link output"
    [ -s app/build/resources.arsc ]     || die "resources.arsc missing from the aapt2 link output"

    # --- 3c. javac: the two real dex classes -------------------------------
    log "javac app/src → app/build/classes…"
    rm -rf app/build/classes
    mkdir -p app/build/classes
    JAVA_SOURCES=()
    while IFS= read -r f; do
        JAVA_SOURCES+=("$f")
    done < <(find app/src -name '*.java' | sort)
    [ "${#JAVA_SOURCES[@]}" -gt 0 ] || die "no .java sources found under app/src"
    # -source/-target 1.8 + -bootclasspath android.jar is the classic Android
    # javac invocation (portable across JDK 8..21; --release 8 would forbid
    # the android.jar bootclasspath).
    javac -source 1.8 -target 1.8 \
        -bootclasspath "$ANDROID_JAR" -classpath "$ANDROID_JAR" \
        -d app/build/classes \
        "${JAVA_SOURCES[@]}"
    log "javac ok: ${#JAVA_SOURCES[@]} source files"

    # --- 3d. d8: dex them (DEX 035 for --min-api 24) -----------------------
    log "d8 → app/build/classes.dex…"
    D8_INPUTS=()
    while IFS= read -r f; do
        D8_INPUTS+=("$f")
    done < <(find app/build/classes -name '*.class' | sort)
    [ "${#D8_INPUTS[@]}" -gt 0 ] || die "javac produced no .class files"
    "$BUILD_TOOLS/d8" --release --lib "$ANDROID_JAR" --min-api 24 \
        --output app/build "${D8_INPUTS[@]}"
    [ -f app/build/classes.dex ] || die "d8 produced no classes.dex"
    log "prebuilts ready: manifest=$(stat -c%s app/build/AndroidManifest.xml)B \
arsc=$(stat -c%s app/build/resources.arsc)B dex=$(stat -c%s app/build/classes.dex)B"

    PACK_ARGS+=(--prebuilt-manifest app/build/AndroidManifest.xml)
    PACK_ARGS+=(--prebuilt-arsc app/build/resources.arsc)
    PACK_ARGS+=(--prebuilt-dex app/build/classes.dex)
fi

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
cargo build --release -p owen-patcher --bin pack
./target/release/pack "${PACK_ARGS[@]}"
[ -f "$APK" ] || die "packer reported success but $APK is missing"

# --- 6a. independent verification (python zipfile + struct walk) ------------
log "python3 verification of $APK…"
APK="$APK" python3 - <<'EOF'
import os, struct, zipfile, sys

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
print(f"  alignment       : OK (.so % 16384, manifest/arsc % 4, all STORED)")

# --- the human-readable size + entry-count table ---------------------------
rows = sorted(z.infolist(), key=lambda i: -i.compress_size)
total_c = sum(i.compress_size for i in rows)
total_u = sum(i.file_size for i in rows)
print(f"\n  {'entry':<58} {'method':<7} {'comp':>12} {'raw':>12}")
print(f"  {'-'*58} {'-'*7} {'-'*12} {'-'*12}")
for i in rows:
    method = "STORED" if i.compress_type == 0 else "DEFLATE"
    print(f"  {i.filename[:58]:<58} {method:<7} {i.compress_size:>12} {i.file_size:>12}")
print(f"  {'-'*58} {'-'*7} {'-'*12} {'-'*12}")
print(f"  {len(rows)} entries — {total_c} compressed / {total_u} raw bytes")
EOF

# --- 6b. real-toolchain verification (apksigner + aapt + zipalign) ----------
if [ -n "$BUILD_TOOLS" ]; then
    log "apksigner verify (v1 + v2 must both be true)…"
    if ! "$BUILD_TOOLS/apksigner" verify --verbose --print-certs "$APK" | tee app/build/apksigner.txt; then
        die "apksigner verify failed"
    fi
    grep -q "Verified using v1 scheme (JAR signing): true" app/build/apksigner.txt \
        || die "apksigner: v1 scheme NOT verified"
    grep -q "Verified using v2 scheme (APK Signature Scheme v2): true" app/build/apksigner.txt \
        || die "apksigner: v2 scheme NOT verified"

    log "aapt dump badging…"
    if ! "$BUILD_TOOLS/aapt" dump badging "$APK" > app/build/badging.txt; then
        die "aapt dump badging failed"
    fi
    head -n 6 app/build/badging.txt
    grep -q "package: name='com.abdallhrgx.owenlauncher'" app/build/badging.txt \
        || die "aapt badging: wrong package name"
    grep -q "versionCode='19'" app/build/badging.txt \
        || die "aapt badging: versionCode is not 19"
    grep -q "android.app.NativeActivity" app/build/badging.txt \
        || die "aapt badging: android.app.NativeActivity not in the launchable set"

    log "zipalign check…"
    "$BUILD_TOOLS/zipalign" -c -f 4 "$APK" \
        || die "zipalign -c 4 failed (uncompressed entries not 4-aligned)"
else
    log "SKIPPING apksigner/aapt/zipalign checks (handcrafted fallback build)"
fi

log "APK READY: $APK"
ls -la dist/
