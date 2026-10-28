# Owen Launcher RS

The complete Owen-Launcher rewrite in safe Rust: a universal, version-independent Fortnite mobile redirecter (`libowen.so`), a LawinServer v2-compatible backend, and the tooling that replaced the Discord bot with an embedded professional web console.

## Crates

| Crate | Purpose |
|---|---|
| `owen-core` | Safe-Rust core shared by the whole stack: ELF64 parsing, the `curl_easy_setopt` four-layer scanner, Fortnite version detection, URL redirect rules, version profiles, CloudDir manifests, pak parsing, per-season Battle Pass extraction, shared config. `#![forbid(unsafe_code)]`. |
| `owen-detect` | Universal `curl_easy_setopt` locator for stripped AArch64 ELF images. Four-layer signature strategy plus the v2 improvements. `#![forbid(unsafe_code)]`, zero dependencies. |
| `owen-hook` | AArch64 inline hook engine (trampolines, full PC-relative relocation, cache maintenance). The first audited unsafe core. |
| `owen-art` | Runtime Java method hooking (native-fication with runtime offset calibration). The second audited unsafe core. |
| `owen-redirecter` | `libowen.so`: curl hook + URL rewrite + PartyHub WebView/Kairos ART hooks + cleartext unlock + auto manifest redirect. HTTP and HTTPS backends are both first-class — the scheme is used verbatim, never rewritten. `pub mod rules` keeps the backend-shared redirect rule surface (`backend_url`, `should_redirect`, `rewrite`, …). |
| `owen-server` | The full axum backend: LawinServer v2-compatible HTTP + MCP + XMPP with real accounts (PBKDF2), OAuth grants, seasons registry (S1–S42) + season-aware timeline, native PartyHub (static serving + graphql), the auto manifest redirecter, `/login` (the liquid-glass `assets/login.html`) and `assets/partyhub-bridge.js`. `#![forbid(unsafe_code)]`. |
| `owen-pak` | UE4 pak reader and per-season Battle Pass extractor. |
| `owen-apk` | APK surgery: STORED zip builder, binary AXML manifest renamer, and the zero-smali `DT_NEEDED` loader injector. |
| `owen-lawin` | The embedded LawinServer-compatible backend ported from the original C server (`app/src/main/cpp`): zero-dependency HTTP/1.1 + MCP + XMPP over the deployed `data/lawin` layout (94 files), hosted in-process by `libowen.so` when `server.embedded = true` or run standalone (formerly the old `owen-server`). `#![forbid(unsafe_code)]`. |
| `owen-cli` | Host-side toolbelt: `scan` (curl detection), `pak` (index + Battle Pass extraction), `manifest` (cloudcontent catalog) and `serve` (runs the backend). `#![forbid(unsafe_code)]`. |
| `owen-patcher` | APK patching the Owen 2.x way — zero smali: `DT_NEEDED` injection, raw zip entry surgery and APK Signature Scheme v2 (RSA-2048/SHA-256) in pure Rust. `#![forbid(unsafe_code)]`. |
| `owen-patcher-jni` | JNI surface of `owen-patcher` (`libowenpatch.so`): the Android launcher calls `patchApk()` instead of the legacy Kotlin smali pipeline. |

## Data layout (`data/`)

```
data/owen/    battlepass (Season2–42), profiles, responses, manifests.json — the axum backend data
data/lawin/   the original LawinServer deployment payload (94 files: profiles, responses, Config, CloudStorage, public) served by owen-lawin
```

## The backend (`owen-server`)

```bash
owen-server --port 3010 --data-dir data/owen \
            --partyhub-dir data/owen/partyhub --xmpp-port 5222
```

- **Real accounts** — registration with email + username + display name + password (PBKDF2-SHA256, constant-time compare), persisted atomically (`accounts.json`, `sessions.json` — sessions survive restarts).
- **OAuth** — password / device_auth / refresh_token / exchange_code grants, `/account/api/oauth/{token,exchange,verify}`, Epic-style errors.
- **MCP** — `QueryProfile` / `ClientQuestLogin` answer with `fullProfileUpdate` (LawinServer v2 behavior); athena profiles are enriched per-season with the flattened Battle Pass items, `level`, `book_level` and `battlepass_purchased`.
- **Seasons** — the static registry (S1–S42, real dates, chapter mapping), `GET /api/launcher/seasons[/:n]` and a season-aware `/fortnite/api/calendar/v1/timeline` (`X-Season` / `?season=` / the account's selected season).
- **Manifests** — `GET /api/launcher/manifests` serves the verified CloudDir database (38 builds, 11.31 → 13.30).
- **The native PartyHub** — serves the patched Kairos shippedweb from `--partyhub-dir` (static + SPA fallback) and answers every Kairos graphql operation natively from the accounts store; `assets/partyhub-bridge.js` (v4) provides the liquid-glass login screen, session restore and the full Kairos bridge protocol.
- **`/login`** — the liquid-glass login/registration page (`assets/login.html`) that hands the game an exchange code via `com.epicgames.fortnite://authorize/?code=…`.

## The no-smali stack

1. **Layer 1 — curl**: `curl_easy_setopt` is located on every version (dlsym when exported, the v2 signature scan otherwise — verified exact on libUE4.so 10.10/12.50 and by disassembly on libUnreal.so/libEOSSDK.so 23.00) and hooked; `CURLOPT_URL` is rewritten to the backend.
2. **Layer 2 — ART**: `WebView.loadUrl`, `DebouncedLimiter.loginUrl`, `DebouncedLimiter.getGraphqlUrl` are hooked at runtime — the replacement for every smali `const-string` patch, including PartyHub on 10.31-15.50.
3. **Layer 3 — cleartext**: `NetworkSecurityPolicy.isCleartextTrafficPermitted` returns true so plain-HTTP login pages load.
4. **Layer 4 — manifest**: launcher/asset/CloudDir traffic is rewritten automatically (`manifest.mode = auto`).

## Configuration (`owen.json`)

Generated by the web console (Redirecter tab) and read from `/sdcard/Android/data/<pkg>/files/owen.json`, `/data/local/tmp/owen.json` or `$OWEN_CONFIG`.

## Verified detection results

| Library | Version | v2 result | Verification |
|---|---|---|---|
| libUE4.so | 10.10 | `0x08EC981C` | exact vs exported symbol |
| libUE4.so | 12.50 | `0x0AE64C6C` | exact vs exported symbol |
| libUnreal.so | 23.00 | `0x0657E4A4` | verified by disassembly |
| libEOSSDK.so | 23.00 | `0x0178AD00` | verified by disassembly |

The original Python tool returns results 0x68 bytes early on the libUE4.so family (forward-scan flaw); `--legacy` reproduces that behaviour exactly, the default v2 fixes it.

## Build

```bash
cargo test --workspace                                            # 46 tests
cargo build --release -p owen-redirecter --target aarch64-linux-android   # libowen.so
cargo build --release -p owen-server                                      # the axum backend
cargo build --release -p owen-lawin                                       # the embedded LawinServer backend
cargo run --release -p owen-detect -- path/to/libUnreal.so                # scanner
cargo run --release -p owen-pak -- pak.pak --battlepass                    # battle pass report
```

CI: `.github/workflows/owen-rs-ci.yml` — workspace tests, the safe-Rust policy check, the live S1–S42 season matrix, PartyHub graphql checks and the aarch64-android `libowenpatch.so` build.

## Safety model

All logic crates are `#![forbid(unsafe_code)]`. Raw memory interop is isolated in two audited cores (`owen-hook`, `owen-art`) with documented invariants — the same discipline the Rust standard library itself uses around OS calls.
