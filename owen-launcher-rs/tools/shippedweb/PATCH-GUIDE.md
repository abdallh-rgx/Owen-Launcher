# Fortnite "shippedweb" Offline Entry - Custom Backend Guide

This document explains, end to end, how the offline Kairos login page that ships
inside Fortnite APKs (`assets/shippedweb/offline/entry/`) talks to its backend,
how to point it at your own backend, and how to fix the page freezing on the
Epic Games logo. Everything below was verified against the real files
(`index.html` + `main.kairos.*.js`, build `8af1b828dc6188736f50`).

---

## 1. How to change the GraphQL URL (the documented way)

The entry page does **not** contain any GraphQL URL in its HTML or JS. The URL
is injected at runtime by the host through a global:

```js
window.clientEnvConfig = {
  EPIC_SERVICE_GRAPHQL_URL: "http://192.168.1.10:8090/graphql",
  ...
};
```

`main.kairos.*.js` reads `window.clientEnvConfig` at configure time and calls
`checkConfig("GraphQlService", config, "EPIC_SERVICE_GRAPHQL_URL")`, which
**throws** if the key is missing - that is why a blank config also freezes the
page. The injection must happen **before** `main.kairos.*.js` loads, i.e. as an
inline `<script>` in `<head>` or as an external script placed before the
`<script src="./main.kairos...js">` tag in `index.html`.

Required and useful keys:

| Key | Required | Used for |
|---|---|---|
| `EPIC_SERVICE_GRAPHQL_URL` | **yes** | Identity / EULA / friends GraphQL endpoint |
| `EPIC_SERVICE_FRIENDS_URL` | no | Friends service (defaults fall back to the graphql URL) |
| `EPIC_SERVICE_PRESENCE_URL` | no | Presence service |
| `EPIC_SERVICE_PARTY_URL` | no | Party service |
| `EPIC_SERVICE_GRAPHQL_WITHCREDENTIALS` | no | Set `false` for plain http backends |
| `EPIC_ENV` | no | `"local-gamedev"` also enables the hidden debug menu |

**Both `http://` and `https://` work.** The URL is passed verbatim to the
HTTP client - nothing rewrites the scheme - so a local `http://` backend works
as long as the page itself is served over `http://` (mixed-content rule).

The concrete query the bundle sends during its identity check:

```
POST <EPIC_SERVICE_GRAPHQL_URL>
Content-Type: application/json

{"query":"query {Account { myAccount {id}}}"}
```

Expected success shape (what the bundle looks for at
`data.data.Account.myAccount.id`):

```json
{ "data": { "Account": { "myAccount": { "id": "<32 hex chars>" } } } }
```

---

## 2. Why the page freezes on the Epic Games logo (root cause)

The bundle communicates with the Fortnite app through a bridge. When the page
is opened in a normal browser (or served without the app), no native bridge
exists, so the bundle falls back to its **built-in mock bridge**. That mock
answers the startup-status call with:

```json
{ "status": "authenticated" }
```

But `authenticated` is **not a handled case** in the LoginContainer state
switch. The handled statuses are `init`, `authenticating`, `prelogin`,
`postlogin`, `autherror`, `error`, `networkoffline`. An unhandled status falls
through to the default branch, which renders the static `LoadingContainer` -
the Epic Games logo - forever. This is the entire "stuck on the Epic screen"
bug. It is not a network problem and not a missing GraphQL URL.

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

Inside the real app the equivalent native calls are
`window.JSBridge.postMessage(id, subsystem, command, argsJson)` (Android) or
`window.ue.embedded.postmessage(id, subsystem, command, argsJson)`.

**Responses (host to JS):**

```js
window.postMessage({
  id: "web-<uuid>",
  type: "asyncResponse",
  payload: { ... },           // resolves the pending call
  errors: undefined           // or a non-empty array to reject
}, "*");
```

Responses are single-shot: the first `asyncResponse` for an id wins, later
ones are ignored. The built-in mock answers after a 100 ms delay, so a custom
listener registered before `main.kairos.*.js` loads can answer **first** and
take control of the flow.

**Notifications (host to JS, unsolicited):**

```js
window.postMessage({ type: "notification", command: "startupstatuschanged",
                     args: [{ status: "prelogin" }] }, "*");
window.postMessage({ type: "notification", command: "oss.identity.loginchanged",
                     args: [{ Status: "LoggedIn", UserId: "..." }] }, "*");
```

Notification commands observed in the bundle: `startupstatuschanged`,
`oss.identity.loginchanged`, `oss.presence.presencechanged`,
`oss.socialban.banchanged`, `oss.party.partymemberconnectionschanged`,
`lifecyclestatus`, `pushNotification`.

**The login state machine** (driven by the host):

```
prelogin  -> "Played Fortnite before?" screen
            user taps "YES, FIND MY ACCOUNT"  ->  JS calls native/login
            host performs the real Epic login, then pushes:
              oss.identity.loginchanged { Status: "LoggedIn", UserId }
              startupstatuschanged { status: "authenticated" }
              startupstatuschanged { status: "postlogin" }
postlogin -> "Logging you in" screen (final state of the entry page;
             the app then opens the hub or launches the game)
```

---

## 4. Ready-made fix: `owen-bridge.js` + `patch_entry.py`

`owen-bridge.js` (same folder) implements everything above:

- injects `window.clientEnvConfig` before the bundle loads,
- wins the response race for `getstartupstatus` and answers `prelogin`
  (fixes the frozen logo),
- implements `login` / `register` / `RetryLogin` / `prelogin` / `logout`
  and pushes the full notification sequence,
- triggers the bundle's own identity check
  (`oss.identity/getloggedinuserid`), which performs the
  `query {Account { myAccount {id}}}` POST against your backend.

Patch a shippedweb copy:

```bash
python3 tools/shippedweb/patch_entry.py <shippedweb-folder> \
        --graphql http://192.168.1.10:8090/graphql \
        --out shippedweb-owen
```

The script writes a **new** folder (originals untouched) with
`offline/entry/owen-bridge.js` injected before the bundle.

---

## 5. Serving requirements

Any static file server works, with three rules:

1. **Same origin for `/graphql`** - either serve the entry and the GraphQL
   endpoint from one origin (use a relative `--graphql /graphql` URL), or pass
   an absolute `http://<backend-host>:<port>/graphql` URL (kept verbatim).
2. **SPA fallback** - the app routes look like `/offline/en-US/...`
   (`appContext = "offline"` + `:locale/` prefix). Unknown paths under the
   served root must return `offline/entry/index.html`.
3. **Relative asset resolution** - the bundle uses `./main.kairos...js`, so a
   page opened from a nested path (`/offline/en-US/`) must still resolve
   assets from `offline/entry/`.

Opening either `http://<host>:<port>/` or
`http://<host>:<port>/offline/en-US/` works with the patched files.

A minimal reference server (static + mock GraphQL + SPA fallback) ships in
this repository as `mini-services/kairos-mock` (Bun), and the plain-Python
`server.py` that ships inside the archive also works for a quick test -
serve the `offline/entry` folder with it and open `http://<ip>:8081/`.

---

## 6. Verified behaviour (end-to-end)

Tested with the real `shippedweb.7z` contents served on `http://localhost:8090`:

1. Open `http://localhost:8090/offline/en-US/` ->
   the "Played Fortnite before?" login screen renders immediately
   (no frozen logo).
2. The bundle performs `POST /graphql` with
   `{"query":"query {Account { myAccount {id}}}"}` - the backend receives the
   request.
3. Tapping "YES, FIND MY ACCOUNT" completes the whole sequence
   (`login` -> `loginchanged` -> `authenticated` -> `postlogin`) and the page
   settles on its final "Logging you in" screen - the same state the real app
   reaches before it hands over to the hub.

## 7. Version note

The `shippedweb/offline/entry` Kairos build is the web login introduced with
the newer Fortnite mobile login flow (the PartyHub-era app). The old claim
that versions 10.31-12.60 contain an `offline/index.html` with a party GraphQL
URL was re-checked during this work: those old APKs do not ship this folder at
all. If you have an APK that does contain `assets/shippedweb/`, the steps in
this document apply to it directly - no smali edits, no manifest changes, only
the `index.html` injection described above.
