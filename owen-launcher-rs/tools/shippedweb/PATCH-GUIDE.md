# Fortnite "shippedweb" Offline Entry - Party Hub on a Custom Backend

This document explains, end to end, how the offline Kairos page that ships
inside Fortnite APKs (`assets/shippedweb/offline/entry/`) talks to its backend,
how to point it at your own backend (Voltronite used here as the reference),
and how to take it all the way to the **Party Hub interface** - not just past
the frozen Epic logo, but past the "Checking your login info..." dead end and
into the actual hub UI (friends list, party, settings, modals).

Everything below was verified against the real files (`index.html` +
`main.kairos.*.js`, build `8af1b828dc6188736f50`) in a real browser, with every
forwarded request logged on the backend.

---

## 1. How to change the GraphQL URL (the documented way)

The entry page does **not** contain any GraphQL URL in its HTML or JS. The URL
is injected at runtime by the host through a global:

```js
window.clientEnvConfig = {
  EPIC_SERVICE_GRAPHQL_URL: "./graphql",   // or an absolute http(s):// URL
  EPIC_SERVICE_GRAPHQL_WITHCREDENTIALS: false,
};
```

`main.kairos.*.js` reads `window.clientEnvConfig` and calls
`checkConfig("GraphQlService", config, "EPIC_SERVICE_GRAPHQL_URL")`, which
**throws** if the key is missing - that is why a blank config also freezes the
page. The injection must happen **before** `main.kairos.*.js` loads.

**Both `http://` and `https://` work.** The URL is passed verbatim to the HTTP
client - nothing rewrites the scheme.

> **v2 note - the configure call itself is missing in the offline build.** The
> online app's bootstrap calls `window.__GraphQlService.configure(config)`;
> the offline bundle never does. Without patching, the singleton keeps its
> constructor default `endpoint: ""` and **every** graphql query POSTs to an
> empty URL (which the browser resolves to the page itself - you get HTML back
> and the account/friends init silently fails). `patch_partyhub.py` therefore
> also patches webpack module 170 to self-configure from
> `window.clientEnvConfig`.

---

## 2. The three reasons the page hangs (root causes)

1. **Frozen Epic logo** - the built-in mock bridge answers the
   startup-status call with `{ "status": "authenticated" }`, which is **not a
   handled case** in the LoginContainer switch (handled: `init`,
   `authenticating`, `prelogin`, `postlogin`, `autherror`, `error`,
   `networkoffline`). Unhandled falls through to the static loading screen -
   the Epic logo - forever.
2. **"Checking your login info..." dead end** - this build mounts *only* the
   LoginContainer as its app root (webpack module 1041, rendered directly by
   the bootstrap in module 982). The **entire Party Hub app ships inside the
   same bundle** (route table in module 1291: home, party, friends, settings,
   about 40 screens + modal routes) but is never mounted. The `postlogin`
   screen ("Logging you in / Checking your login info...", localization key
   `diesel.kairos.login.authenticating`) is a dead end by design - the native
   app is supposed to take over at that point.
3. **Unconfigured graphql singleton** - see the note in section 1: without the
   module-170 patch, all service queries (account, friends, lightswitch,
   settings) fail silently because they POST to an empty URL.

---

## 3. The bridge protocol (for custom integrations)

All traffic is `window.postMessage` based (same window, `"*"` target).

**Calls (JS to host):**

```js
window.postMessage({
  id: "web-<uuid>",          // correlation id
  type: "mockRequest",       // sent when no window.JSBridge / window.ue exists
  command: "getstartupstatus",
  args: JSON.stringify(...)  // optional
}, "*");
```

**Responses (host to JS):**

```js
window.postMessage({
  id: "web-<uuid>",
  type: "asyncResponse",
  payload: { ... },           // resolves the pending call
}, "*");
```

Responses are single-shot: the first `asyncResponse` for an id wins. The
built-in mock answers after a 100 ms delay, so a listener registered before
`main.kairos.*.js` loads can answer **first** and take control. v2 additionally
calls `event.stopImmediatePropagation()` for handled commands - otherwise the
built-in mock still *executes* them, and its `login`/`register`/`logout`
handlers navigate the page away (`location.href = appContext + "/" + command`)
whenever `window.appContext` is not literally `"offline"`.

**Notifications (host to JS, unsolicited):**

```js
window.postMessage({ type: "notification", command: "startupstatuschanged",
                     args: [{ status: "prelogin" }] }, "*");
window.postMessage({ type: "notification", command: "oss.identity.loginchanged",
                     args: [{ Status: "LoggedIn", UserId: "..." }] }, "*");
```

**The v2 login state machine** (driven by the host):

```
prelogin  -> "Played Fortnite before?" screen
            user taps "YES, FIND MY ACCOUNT"  ->  JS calls native/login
            host performs the login, then pushes:
              oss.identity.loginchanged { Status: "LoggedIn", UserId }
              startupstatuschanged { status: "authenticated" }
            ...and STOPS. No "postlogin"!
```

Two statefulness rules matter once the full hub is mounted:

- `getstartupstatus` must always report the **current** status. When the
  patched root mounts the full app it re-registers the startup-state effect,
  which re-queries the bridge; a bridge that keeps answering `prelogin` would
  kick the user back to the login screen seconds after login.
- Do **not** push `postlogin`. The app's initialization chain
  (`useAppInitialization`) fires "once on status == authenticated"; if the
  status races past `authenticated` to `postlogin` before the hub mounts, that
  trigger never fires and the account/friends never initialize.

Other commands the hub needs answered (all implemented in `owen-bridge.js`):
`oss.identity/getconfiguredbackend` (queried before the lightswitch graphql
call), `oss.identity/getloggedinuserid` (must return the same account id the
backend reports), `native/getsetting` (persisted settings - the bridge seeds
`<accountId>:onboardingComplete` and `<accountId>:introVideoWatched` so the
hub renders in its final state; every other key falls through to the built-in
mock, which stores settings in localStorage).

---

## 4. Ready-made fix: `patch_partyhub.py` (v2)

`patch_partyhub.py` (same folder) produces a complete Party Hub web from any
pristine shippedweb copy. It:

1. replaces webpack **module 1041** (the LoginContainer root) with a hybrid
   root (`partyhub-root.js`): the original login flow until the status is
   `authenticated`, then the **real app** - `useAppInitialization()`,
   `AppEffects` (lightswitch, content control, parties, presence, voice,
   friends), `useInitializers(AppStateInitializers)`, and the real router
   (`getRoutes(appRoutes)` + modal routes with the isModal location pattern);
2. patches **module 170** so the GraphQlService singleton configures itself
   from `window.clientEnvConfig` (see section 1);
3. patches `index.html`: sets `window.appContext` (the route base - see
   section 5), injects `clientEnvConfig`, and injects the stateful
   **`owen-bridge.js`** before the polyfill bundle.

```bash
python3 tools/shippedweb/patch_partyhub.py \
        --src <pristine-shippedweb> \
        --out <patched-copy> \
        --base "/partyhub/offline" \   # or "offline" when served from the root
        --graphql "./graphql"
```

`window.appContext` is both the URL prefix the router matches
(`getPath()` + `":locale/" + route`) and the URL prefix under which the page
is served:

| --base            | page URL                        | served from                 |
|-------------------|---------------------------------|-----------------------------|
| `offline`         | `/offline/en-US/home`           | the shippedweb folder root  |
| `/partyhub/offline` | `/partyhub/offline/en-US/home` | behind the console proxy    |

---

## 5. Serving requirements

Any static file server works, with three rules:

1. **Same origin for the graphql adapter** - use a relative `./graphql` URL
   and make the serving host answer the graphql endpoint on **every** SPA
   route (the page's URL changes as the router navigates; a relative URL
   stays correct). Alternatively pass an absolute
   `http://<backend-host>:<port>/graphql` URL.
2. **SPA fallback** - the app routes look like `/<base>/:locale/...`
   Unknown paths must return `offline/entry/index.html`.
3. **Relative asset resolution** - the bundle uses `./main.kairos...js`, so a
   page opened from a nested path must still resolve assets from
   `offline/entry/`.

---

## 6. The Voltronite dialect adapter (`mini-services/partyhub-web`)

The Kairos web POSTs **raw graphql documents**:

```json
{ "query": "query { Account { myAccount { ... } }, Fortnite { myProfile { id } } }",
  "variables": {} }
```

- no `operationName`
- no `Authorization` header (in the real app the native layer owns auth; the
  web's `getAuthToken` helper is never called by this build)

Voltronite's `/partyhub/graphql` switches on `body.operationName` and requires
`Authorization: Bearer <token>`. The adapter in `mini-services/partyhub-web`
(Bun) bridges the two:

1. receives the raw document same-origin on any route ending in `/graphql`,
2. fingerprints the query text and maps it to a Voltronite operation:

   | raw query (fingerprint)                    | Voltronite operation        |
   |--------------------------------------------|-----------------------------|
   | `Account.myAccount` + `externalAuths`      | `GetMyAccount`              |
   | `LightSwitch.fortniteStatus`+`kairosStatus`| `GetStatusPerService`       |
   | `ContentControl ... canUseVoiceChat`       | `GetStatusPerService` (extract ContentControl) |
   | `Eula.hasAccountAccepted`                  | `EulaAccepted`              |
   | `UserSettings.mySetting`                   | `GetMySetting`              |
   | `UserSettings.setting(key, accountIds)`    | `GetAccountSettings`        |
   | `mutation UserSettings.updateSetting`      | `UpdateUserSetting`         |
   | `notificationSettings` (Friends/Party)     | `GetNotificationSettings`   |
   | `query {Account { myAccount {id}}}` probe  | `GetMyAccount`              |

3. obtains a real access token from Voltronite's
   `POST /account/api/oauth/token` (`grant_type=password&username=OwenTester`,
   cached, refreshed on 401) - the account id is `hex(username)`, and the
   bridge's `getloggedinuserid` answer **must match it** so the
   persisted-settings keys line up;
4. forwards `{operationName, query, variables}` + Bearer to
   `/partyhub/graphql` - **the backend receives and answers the request**;
5. applies two compatibility transforms: `kairosStatus.status` is forced to
   `"UP"` (Voltronite reports `DOWN` for Kairos; the old client parks the hub
   on a "service offline" warning without it), and `SocialBan.socialBan` is
   added as `null` (the client reads `.starts_at`, absent -> not banned).

Operations Voltronite does not implement (friends summary/blockList/getFriend,
friend mutations, eula body, character profile, account lookups) are answered
locally with neutral defaults so the hub renders cleanly. Run it with:

```bash
bun run mini-services/partyhub-web    # serves the patched copy + adapter :8091
```

Environment: `OWEN_WEB_ROOT` (patched copy), `VOLTRONITE_URL`
(default `http://127.0.0.1:3551`), `OWEN_USER` (default `OwenTester`),
`PORT` (default 8091). `GET /__status` returns the recent request log with
which entries were forwarded to the backend.

---

## 7. Verified behaviour (end-to-end, real browser)

With Voltronite on :3551 and partyhub-web on :8091 behind the console's
`/partyhub` proxy:

1. Open `/partyhub/offline/en-US` -> the "Played Fortnite before?" login
   screen renders immediately (no frozen logo).
2. Tap "YES, FIND MY ACCOUNT" -> `login` -> `loginchanged(LoggedIn)` ->
   `startupstatuschanged(authenticated)`; the patched root mounts the full
   app and the router lands on `/partyhub/offline/en-US/home`.
3. The Party Hub home screen renders: onboarding cards, "ENABLE YOUR
   MICROPHONE" / "ENABLE NOTIFICATIONS" permissions, the Llama Bell section,
   the friend-request badge, "ADD FRIENDS" and "SHARE THE APP" actions. No
   JavaScript errors (verified with the in-page error box).
4. Modal routes work - "ADD FRIENDS" navigates to
   `home/universal-search/PartyHubSearch` and renders the search modal over
   the hub.
5. The backend log shows every forwarded operation: `GetMyAccount`,
   `GetStatusPerService`, `GetMySetting`, `GetAccountSettings`,
   `UpdateUserSetting`, `GetNotificationSettings` - all answered by
   Voltronite with real account data (`OwenTester` /
   `4f77656e546573746572`).

The whole flow is embedded live in the Owen web console (PartyHub tab).

---

## 8. Version note

The `shippedweb/offline/entry` Kairos build is the web login introduced with
the newer Fortnite mobile login flow (the PartyHub-era app). The old claim
that versions 10.31-12.60 contain an `offline/index.html` with a party GraphQL
URL was re-checked during this work: those old APKs do not ship this folder at
all. If you have an APK that contains `assets/shippedweb/`, the steps in this
document apply to it directly - no smali edits, no manifest changes.

---

## 10. v3 - The phone package (golden login, no debug strip, Voltronite links)

`patch_partyhub.py --src <pristine> --out <copy> --base /offline --server`
produces the runnable phone package. What v3 adds on top of v2:

### 10.1 The top strip is gone

The original shippedweb.7z carried an injected Arabic debug-overlay script in
`index.html` (a `position:fixed; top:0` red-on-black `__debugBox` that covers
the top of every screen and prints JS errors). The patcher now strips ANY
`<script>` block whose body contains `__debugBox`, `captureLog`,
`window.__errors` or `unhandledrejection` (tempered regex - the marker can
never leak past a closing `</script>`), so the strip never renders.

### 10.2 The golden launcher login page

`owen-bridge.js` renders the Owen Launcher "Aurum" login as a full-screen page
instead of the Kairos "Played Fortnite before?" screen (which mounts underneath
and stays hidden):

- charcoal gradient background (`#1A140C -> #141009`), `OWEN` in white +
  `LAUNCHER` in the gold gradient (`#FFE9B8 -> #FFC24B -> #FF9F0A`)
- a player-name card (`#211A11`, gold border, 16px radius) with the
  `Required`/`OK` pill, an outlined gold-focus input, and the gold gradient
  LOGIN button (52px, `#FFB300 -> #FF9F0A -> #FF8A00`)
- the typed name becomes the Voltronite account; it is persisted in
  localStorage and prefilled on the next visit; logout re-shows the page

### 10.3 Voltronite links (all in `owen-bridge.js`)

```js
window.OWEN_CONFIG = {
  VOLTRONITE_URL: "http://127.0.0.1:3551",  // <- the ONE link to edit
  USERNAME: "OwenTester",                    // default (login page wins)
  RELAY_URL: "auto",                         // <appContext>/owen-graphql
};
```

The bundle's HTTP stack is **axios over XMLHttpRequest** (NOT fetch), so the
bridge hooks `XMLHttpRequest.prototype.open/send`: every POST whose URL
matches graphql is consumed, its raw query fingerprinted and mapped to a
Voltronite `operationName`, and forwarded (via the ORIGINAL window.fetch) to
the same-origin relay `<appContext>/owen-graphql` with `X-Owen-Backend` /
`X-Owen-User` headers. The relay (`server.py` on the phone, `partyhub-web`
behind the web console) attaches the real Bearer token (password grant,
cached per user, refreshed once on 401) and forwards to
`<backend>/partyhub/graphql`. A same-origin relay is REQUIRED: Voltronite
sends no CORS headers, so a browser page can never call it directly.

### 10.4 The 32-char account id (short-name crash fix)

Voltronite derives `accountId = hex(username)` - 6 hex chars for "Rex". The
Kairos home screen hashes the id with `parseInt(id.slice(8, 9), 16)`: with a
short id that slice is empty, the hash is `NaN`, and HomeContainer crashes on
`backgroundColor: b[1]` (red error boundary, "An unknown error occurred").
The bridge therefore uses a **client-side id = hex(name) zero-padded to 32
chars** everywhere (login notifications, getloggedinuserid, settings keys)
and rewrites the ids Voltronite returns (myAccount/myProfile) to the padded
form in the GetMyAccount extract. Verified with 1-char ("K"), 3-char ("Rex")
and 10-char ("WebTester") names.

### 10.5 Running it

```
python server.py                    # http://127.0.0.1:8081 -> /offline/en-US/
PORT=9000 python server.py          # custom port
```

`server.py` is pure Python 3 stdlib (ThreadingHTTPServer): static files from
its own folder, SPA fallback (extension-less misses -> offline/entry/index.html),
relative-asset resolution (`/offline/en-US/main.*.js` -> `offline/entry/`),
`POST */owen-graphql` -> the Voltronite relay, per-request backend override
from `X-Owen-Backend` (fallback: `OWEN_BACKEND` env or `http://127.0.0.1:3551`).

If the backend is unreachable the relay answers 502 and the bridge falls back
to local neutral answers, so the hub still renders (the relay log prints
`[RELAY] <op> user=<name> -> FAILED`).
