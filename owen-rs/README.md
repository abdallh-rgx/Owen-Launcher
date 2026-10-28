# Owen-RS — the unified safe-Rust Owen Launcher stack

The complete Owen Launcher conversion to **safe Rust** (the successor of the
`owen-launcher-rs` workspace): a LawinServer v2-compatible backend with
Reload-Backend features, real accounts, the **native PartyHub**, the universal
redirecter (`libowen.so`) and the APK/ELF surgery tooling — behind a liquid-glass
launcher interface (iOS 26 style).

## Safety model — "Rust safe 1:1"

Every logic crate is `#![forbid(unsafe_code)]` and verified by CI to contain
zero `unsafe` blocks:

| Crate | Role | Unsafe |
|---|---|---|
| `owen-core` | profiles, manifests (CloudDir), battlepass, ELF parsing, season registry | **0** |
| `owen-server` | LawinServer v2-compatible HTTP+MCP+XMPP backend, accounts, native PartyHub | **0** |
| `owen-patcher` | `DT_NEEDED` injection, APK surgery (zero smali) | **0** |
| `owen-cli` | the host-side toolbelt | **0** |
| `owen-redirecter` | `libowen.so` — curl hook + ART hooks + auto manifest | audited cores only |
| `owen-detect` / `owen-hook` / `owen-art` | the AArch64 scanner + the two audited unsafe cores (trampolines, JNI) | documented invariants |

## The backend (`owen-server`)

```bash
owen-server --port 3010 --data-dir data/owen \
            --partyhub-dir data/owen/partyhub --xmpp-port 5222
```

- **Real accounts** — registration with email + username + display name +
  password (PBKDF2-SHA256, 60k iterations, constant-time compare). No longer
  name-only. Persisted atomically (`accounts.json`, `sessions.json` — sessions
  survive restarts).
- **OAuth** — password / device_auth / refresh_token / exchange_code grants,
  `/account/api/oauth/{token,exchange,verify}`, Epic-style errors.
- **MCP** — `QueryProfile` / `ClientQuestLogin` always answer with
  `fullProfileUpdate` (LawinServer v2 behavior); athena profiles are enriched
  per-season with the flattened Battle Pass items (Reload-Backend tier
  slicing), `level`, `book_level` and `battlepass_purchased`.
- **Seasons** — the static registry (S1–S42, real dates, chapter mapping) +
  `GET /api/launcher/seasons[/:n]` + a season-aware
  `/fortnite/api/calendar/v1/timeline` (`X-Season` / `?season=` / the
  account's selected season).
- **Manifests** — `GET /api/launcher/manifests` serves the verified CloudDir
  database (38 builds, 11.31 → 13.30, including **12.41 = `aEzFZyiSFDrqiS_7jkqD6K8KN4zQEg`**).
- **The native PartyHub** — serves the patched Kairos shippedweb from
  `--partyhub-dir` (static + SPA fallback + nested-asset resolution) and
  answers every Kairos graphql operation natively from the accounts store —
  no relay, no Node: `GetMyAccount`, `GetStatusPerService` (kairos UP),
  `EulaAccepted`, `GetMySetting`/`UpdateUserSetting` (persisted),
  friends, `myProfile` character, batch arrays.
- **`/login`** — the liquid-glass login/registration page
  (`assets/login.html`) that hands the game an exchange code via
  `com.epicgames.fortnite://authorize/?code=…`.

## The bridge (`assets/partyhub-bridge.js`, v4)

Injected into the PartyHub webview by the redirecter (or the patched
`index.html`):

- the **liquid-glass login/registration screen** (email + username + display
  name + password — the Owen accounts, same design tokens as the launcher UI)
- session restore (`localStorage` + `/account/api/oauth/verify`, shares the
  launcher session when served from the same origin)
- the full Kairos bridge protocol: stateful `getstartupstatus`,
  `loginchanged`, mockRequest/asyncResponse racing, `DataCloneError` defusing
- native presence/party: `getpresence`/`setpresence` (real account, Online,
  `party.joininfodata.286331153_j`), `oss.party.partyjoined` +
  `oss.presence.presencechanged` **native_notification** pushes (the app never
  polls `getparty`), voice `join`/`getchannelinfo` with `{ChannelName,
  ChannelType}` so the lobby connects
- XHR interception → every raw Kairos graphql query forwarded same-origin to
  `/partyhub/graphql` with the Bearer token; local fallbacks keep the UI alive
  offline

## Build

```bash
cargo test --workspace                                   # 44+ tests
cargo build --release -p owen-server                     # the backend
cargo build --release -p owen-redirecter \
  --target aarch64-linux-android                          # libowen.so (NDK)
```

CI: `.github/workflows/owen-rs-ci.yml` — workspace tests, the safe-Rust
policy check, the **live S1–S42 season matrix** (register → login → timeline +
detail + MCP per season), PartyHub graphql checks and the aarch64-android
redirecter build. The Android emulator E2E (launcher + redirecter + game
install + hooks) lives in `test-launcher-e2e.yml`.

## Data layout (`data/owen`)

```
battlepass/   Season2.json … Season42.json   (41 season files)
profiles/     athena, common_core, campaign, …
responses/    catalog, keychain, CloudDir, …
manifests.json                                  38 verified CloudDir builds
partyhub/     the patched Kairos shippedweb (NOT redistributed in this repo)
accounts.json / sessions.json                   created at runtime
```
