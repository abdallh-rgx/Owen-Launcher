/*
 * Owen Bridge v4 — Owen-native edition (Liquid Glass).
 *
 * WHAT'S IN v4
 * ------------
 * 1. LIQUID GLASS LOGIN PAGE — a full iOS-26-style glass login/registration
 *    screen (email + username + display name + password, PBKDF2-verified by
 *    the Owen Rust backend) rendered INSTEAD of the Kairos "Played Fortnite
 *    before?" screen. Registration is no longer name-only.
 *
 * 2. OWEN-NATIVE GRAPHQL — every Kairos graphql query is answered by the
 *    Owen Rust server itself (same origin, POST /partyhub/graphql) with the
 *    session's Bearer token. No relay, no Voltronite: the accounts,
 *    settings and friends live in the Owen backend (accounts.json).
 *
 * 3. SESSION RESTORE — the token survives reloads (localStorage +
 *    /account/api/oauth/verify), so the hub opens straight to the app.
 *
 * 4. BRIDGE PROTOCOL — identical to the real native layer (stateful
 *    startupstatus, loginchanged notifications, mockRequest/asyncResponse
 *    with stopImmediatePropagation, DataCloneError defusing).
 *
 * 5. OFFLINE RESILIENCE — if the backend is unreachable, every query falls
 *    back to a local answer so the PartyHub UI always renders.
 *
 * SERVING
 * -------
 * The Owen server serves this file at /partyhub/owen-bridge.js (absolute
 * path — patch the Kairos index.html to reference it before the bundle).
 */

(function () {
  "use strict";

  /* When the hub is served behind the web-console proxy (appContext
   * "/partyhub/..."), the Owen REST endpoints live under the /owen-api
   * rewrite; on-device (served from the backend itself) they are plain
   * same-origin paths. Detected once, here. */
  var OWEN_API_BASE = (function () {
    try {
      var ctx = String(window.appContext || "");
      if (ctx.indexOf("/partyhub") === 0) return "/owen-api";
    } catch (err) { /* ignore */ }
    return "";
  })();

  window.OWEN_CONFIG = {
    /* Same-origin endpoints of the Owen Rust backend. */
    GRAPHQL_URL: "/partyhub/graphql",
    TOKEN_URL: OWEN_API_BASE + "/account/api/oauth/token",
    REGISTER_URL: OWEN_API_BASE + "/api/launcher/register",
    VERIFY_URL: OWEN_API_BASE + "/account/api/oauth/verify",
    TOKEN_KEY: "owen:token",
    ACCOUNT_KEY: "owen:account",
    LANG_KEY: "owen:lang",
  };

  /* ------------------------------------------------------------------ *
   * 1. Client environment — MUST be set before main.kairos.*.js loads.  *
   * ------------------------------------------------------------------ */
  window.clientEnvConfig = Object.assign(
    {
      EPIC_SERVICE_GRAPHQL_URL: "./graphql",
      EPIC_SERVICE_GRAPHQL_WITHCREDENTIALS: false,
      EPIC_ENV: "production",
      EPIC_ENABLE_DEBUG_MENU: false,
    },
    window.clientEnvConfig || {}
  );

  var STARTUP_STATUS = {
    INIT: "init",
    AUTHENTICATING: "authenticating",
    AUTHENTICATED: "authenticated",
    PRELOGIN: "prelogin",
    POSTLOGIN: "postlogin",
    AUTH_ERROR: "autherror",
    ERROR: "error",
    NETWORK_OFFLINE: "networkoffline",
  };

  /* ------------------------------------------------------------------ *
   * 2. Session (restored from localStorage, verified lazily).          *
   * ------------------------------------------------------------------ */
  var session = { token: "", accountId: "", displayName: "", username: "" };
  try {
    session.token = localStorage.getItem(window.OWEN_CONFIG.TOKEN_KEY) || "";
    /* The hub shares its origin with the launcher web UI (both served by
     * the Owen backend) — reuse the launcher session when present. */
    if (!session.token) {
      session.token = localStorage.getItem("owen:web-token") || "";
    }
    var savedAcc = JSON.parse(localStorage.getItem(window.OWEN_CONFIG.ACCOUNT_KEY) || "null");
    if (savedAcc) {
      session.accountId = String(savedAcc.accountId || savedAcc.account_id || "");
      session.displayName = String(savedAcc.displayName || savedAcc.display_name || "");
      session.username = String(savedAcc.username || "");
    }
  } catch (err) { /* private mode */ }

  var currentStatus = STARTUP_STATUS.PRELOGIN;
  /* Set when the session was restored from a stored token (boot verify) —
   * the app is then pushed straight past its login screen. */
  var sessionAutoLogin = false;

  function persistSession(account, token) {
    session.token = token;
    session.accountId = String((account && (account.accountId || account.account_id)) || "");
    session.displayName = String((account && (account.displayName || account.display_name)) || "");
    session.username = String((account && account.username) || "");
    try {
      localStorage.setItem(window.OWEN_CONFIG.TOKEN_KEY, token);
      localStorage.setItem(window.OWEN_CONFIG.ACCOUNT_KEY, JSON.stringify({
        accountId: session.accountId,
        displayName: session.displayName,
        username: session.username,
      }));
    } catch (err) { /* ignore */ }
  }

  function clearSession() {
    session = { token: "", accountId: "", displayName: "", username: "" };
    try {
      localStorage.removeItem(window.OWEN_CONFIG.TOKEN_KEY);
      localStorage.removeItem(window.OWEN_CONFIG.ACCOUNT_KEY);
    } catch (err) { /* ignore */ }
  }

  /* ------------------------------------------------------------------ *
   * 3. Bridge helpers (postMessage protocol).                           *
   * ------------------------------------------------------------------ */
  function respond(id, payload) {
    window.postMessage(
      { id: id, type: "asyncResponse", payload: payload === undefined ? null : payload },
      "*"
    );
  }

  /* Defuse the built-in mock's Promise payloads (DataCloneError). */
  var nativePostMessage = window.postMessage.bind(window);
  window.postMessage = function (message, targetOrigin, transfer) {
    if (
      message &&
      typeof message === "object" &&
      message.type === "asyncResponse" &&
      message.payload &&
      typeof message.payload.then === "function"
    ) {
      message.payload = null;
    }
    return nativePostMessage(message, targetOrigin, transfer);
  };

  function notify(command, args) {
    /* The Kairos bundle's processBridgeResponse only dispatches messages
     * whose type is exactly "native_notification" (BridgeType enum) —
     * any other type is silently ignored. */
    window.postMessage(
      { type: "native_notification", command: command, args: args },
      "*"
    );
  }

  function setStartupStatus(status) {
    currentStatus = status;
    notify("startupstatuschanged", [{ status: status }]);
  }

  function loginChanged(status, userId) {
    notify("oss.identity.loginchanged", [{ Status: status, UserId: userId || "" }]);
  }

  /* ------------------------------------------------------------------ *
   * 4. Command table — answers win the race against the built-in mock.  *
   * ------------------------------------------------------------------ */
  var COMMANDS = {
    getstartupstatus: function () {
      return { status: currentStatus };
    },
    login: function () { return driveAuthenticated(); },
    register: function () { return COMMANDS.login(); },
    RetryLogin: function () { return COMMANDS.login(); },
    prelogin: function () {
      /* Auto-login: with a restored session the app is pushed straight to
       * the authenticated state instead of its login screen (the hybrid
       * root mounts the full PartyHub on the authenticated transition). */
      if (session.token && sessionAutoLogin) {
        return driveAuthenticated();
      }
      setTimeout(function () { setStartupStatus(STARTUP_STATUS.PRELOGIN); }, 100);
      return {};
    },
    logout: function () {
      clearSession();
      setTimeout(function () {
        loginChanged("LoggedOut", "");
        setStartupStatus(STARTUP_STATUS.PRELOGIN);
        showGlassLogin(true);
      }, 100);
      return {};
    },
    getloggedinuserid: function () {
      return { UserId: session.accountId || owenFallbackId() };
    },
    getuserauthtoken: function () {
      return { Token: session.token || "owen-offline-token" };
    },
    refreshauth: function () {
      return { Token: session.token || "owen-offline-token" };
    },
    getconfiguredbackend: function () {
      return { appName: "fortnite", environment: "production", gameBackend: "fortnite" };
    },
    getsetting: function (args) {
      try {
        var key = typeof args === "string" ? (JSON.parse(args) || {}).key : args && args.key;
        if (typeof key === "string") {
          if (/(^|:)onboardingComplete$/.test(key)) return { key: key, value: "true" };
          if (/(^|:)introVideoWatched$/.test(key)) return { key: key, value: "true" };
        }
      } catch (err) { /* fall through */ }
      return undefined; /* not handled -> built-in mock (localStorage) */
    },
    getplatforminfo: function () {
      return { Platform: "ANDROID", DeviceModel: "Owen", OSVersion: "13" };
    },
    getsafezone: function () {
      return { top: 0, right: 0, bottom: 0, left: 0, initialized: true };
    },
    getepicappversion: function () { return 1; },
    getgamesupportstatus: function () { return { supported: true }; },

    /* ---- presence (the party card's data source) ---- */
    getpresence: function (args) {
      var me = session.accountId || owenFallbackId();
      var ids = (args && args.UserIds) || [];
      if (!Array.isArray(ids) || ids.length === 0) ids = [me];
      return ids
        .filter(function (u) { return !!u; })
        .map(function (u) {
          var isMe = String(u) === me;
          return isMe ? ownPresence() : {
            UserId: String(u),
            State: "Offline",
            Status: "",
            Properties: "{}",
          };
        });
    },
    setpresence: function (args) {
      /* echo back the presence (the app adapts the response). */
      var me = session.accountId || owenFallbackId();
      return {
        UserId: me,
        State: (args && args.state) || "Online",
        Status: (args && args.status) || "",
        Properties: (args && args.properties) || "{}",
      };
    },

    /* ---- party (single-player party with the real account) ---- */
    getparty: function () { return owenParty(); },
    createparty: function () { return owenParty(); },
    join: function (args) {
      /* `join` is shared by the party and the voice subsystems — the voice
       * join carries {channelId, channelType} and needs {ChannelName,
       * ChannelType} back to flip the lobby to Connected. */
      var a = args || {};
      if (typeof a.channelId === "string" || typeof a.channelType === "string") {
        return {
          ChannelName: a.channelId || "owen-voice",
          ChannelType: a.channelType || "party",
        };
      }
      return owenParty();
    },
    getchannelinfo: function (args) {
      var a = args || {};
      return {
        ChannelName: a.channelId || "owen-voice",
        ChannelType: a.channelType || "party",
      };
    },
    leave: function () { return {}; },
    webheartbeat: function () { return {}; },
    initializevoice: function () { return {}; },
    iscontentuptodate: function () { return true; },
    getactivedevicestate: function () { return { state: "Active" }; },
    getactivedeviceplatform: function () { return { platform: "ANDROID" }; },
    getrecordmuted: function () { return { muted: false }; },
    getplaybackmuted: function () { return { muted: false }; },
    getplaybackvolume: function () { return { volume: 1 }; },
    getparticipantvolume: function () { return { volume: 1 }; },
    isparticipantmuted: function () { return { muted: false }; },

    /* ---- settings echo (download toggles, onboarding flags) ---- */
    setsetting: function (args) {
      try {
        if (args && typeof args.key === "string") {
          localStorage.setItem("owen:setting:" + args.key, String(args.value));
        }
      } catch (err) { /* ignore */ }
      return args || {};
    },
    getusersetting: function (args) {
      var key = Array.isArray(args) ? args[0] : args && args.key;
      var val = null;
      try {
        val = localStorage.getItem("owen:setting:" + key);
      } catch (err) { /* ignore */ }
      return { key: String(key || ""), value: val === null ? "true" : val };
    },
    requestbroadcast: function () { return {}; },
    setfeaturestatuscallback: function () { return {}; },
    getanalyticssessiondetails: function () { return {}; },
  };

  function owenParty() {
    var me = session.accountId || owenFallbackId();
    var display = session.displayName || "OwenPlayer";
    return {
      PartyId: "owen-party-" + me.slice(0, 8),
      LeaderId: me,
      PresencePermissions: "Anyone",
      State: "Active",
      Members: [
        {
          Id: me,
          DisplayName: display,
          Platform: "ANDROID",
          bConnected: true,
          Type: "embedded",
          Properties: JSON.stringify({
            VoiceChatStatus_s: "Enabled",
            voicechatmuted_b: false,
            "party.joininfodata.286331153_j": {},
          }),
        },
      ],
    };
  }

  /* Own presence — carries the Kairos profile and the party join info
   * (the lobby reads `party.joininfodata.286331153_j` to know a party
   * exists and how many members it has). */
  function ownPresence() {
    var me = session.accountId || owenFallbackId();
    var party = owenParty();
    return {
      UserId: me,
      State: "Online",
      Status: "",
      Properties: JSON.stringify({
        KairosProfile_j: { appInstalled: "init" },
        "party.joininfodata.286331153_j": {
          partyId: party.PartyId,
          pc: party.Members.length,
          bIsPrivate: false,
          sourceId: me,
        },
      }),
    };
  }

  /* Push the party + presence to the app once its listeners are up (the
   * notifications are what actually populate the party store — the app
   * never polls getparty on its own). */
  function notifyPartyJoined(delayMs) {
    setTimeout(function () {
      try {
        notify("oss.party.partyjoined", [owenParty()]);
        notify("oss.presence.presencechanged", [ownPresence()]);
      } catch (err) { /* ignore */ }
    }, delayMs || 1500);
  }

  function driveAuthenticated() {
    setTimeout(function () {
      loginChanged("LoggedIn", session.accountId || owenFallbackId());
      setStartupStatus(STARTUP_STATUS.AUTHENTICATED);
      notifyPartyJoined(1800);
      notifyPartyJoined(4200);
    }, 150);
    return {};
  }

  function owenFallbackId() {
    var h = "owenplayer";
    var out = "";
    for (var i = 0; i < 32; i++) out += ((h.charCodeAt(i % h.length) + i) % 16).toString(16);
    return out;
  }

  window.addEventListener("message", function (event) {
    var data = event.data;
    if (!data || typeof data !== "object" || data.type !== "mockRequest") return;
    try {
      (window.__owenCalls = window.__owenCalls || []).push(String(data.command));
    } catch (err) { /* ignore */ }
    var handler = COMMANDS[data.command];
    if (typeof handler !== "function") {
      try {
        console.debug("[owen-bridge] unhandled command:", data.command, data.args);
      } catch (err) { /* ignore */ }
      return;
    }
    var handled = false;
    try {
      var parsed = typeof data.args === "string" ? JSON.parse(data.args || "null") : data.args;
      var result = handler(parsed);
      if (result !== undefined) {
        respond(data.id, result);
        handled = true;
      }
    } catch (e) {
      respond(data.id, null);
      handled = true;
    }
    if (handled) event.stopImmediatePropagation();
  });

  /* ------------------------------------------------------------------ *
   * 5. GraphQL: XHR interception -> the Owen backend (same origin).     *
   * ------------------------------------------------------------------ */
  var nativeFetch = typeof window.fetch === "function" ? window.fetch.bind(window) : null;
  var XHR = window.XMLHttpRequest;
  var xhrOpen = XHR && XHR.prototype.open;
  var xhrSend = XHR && XHR.prototype.send;

  function localStatusAnswer() {
    return {
      data: {
        ContentControl: { namespace: { result: { canUseVoiceChat: true } } },
        LightSwitch: {
          fortniteStatus: { allowedActions: ["PLAY"], status: "UP", banned: false },
          kairosStatus: { status: "UP" },
        },
        SocialBan: { socialBan: null },
      },
    };
  }

  /* Local fallbacks (used when the backend is unreachable). */
  function localAnswer(query) {
    var q = String(query || "").replace(/\s+/g, " ").trim();
    var user = session.displayName || "OwenPlayer";
    var id = session.accountId || owenFallbackId();
    if (q.indexOf("fortniteStatus") >= 0 && q.indexOf("kairosStatus") >= 0) return localStatusAnswer();
    if (q.indexOf("canUseVoiceChat") >= 0)
      return { data: { ContentControl: { namespace: { result: { canUseVoiceChat: true } } } } };
    if (q.indexOf("myAccount") >= 0 && q.indexOf("externalAuths") >= 0)
      return { data: { Account: { myAccount: { id: id, displayName: user, email: user.toLowerCase() + "@owen.local", country: "US", externalAuths: [] } }, Fortnite: { myProfile: { id: id } } } };
    if (q.replace(/\s/g, "") === "query{Account{myAccount{id}}}")
      return { data: { Account: { myAccount: { id: id } } } };
    if (q.indexOf("hasAccountAccepted") >= 0)
      return { data: { Eula: { hasAccountAccepted: { accepted: true } } } };
    if (q.indexOf("getLatestAndConvert") >= 0)
      return { data: { Eula: { getLatestAndConvert: { accepted: true, body: "", key: "fortnite", locale: "en-US", title: "", version: 1 } } } };
    if (q.indexOf("myAvailableSetting") >= 0)
      return { data: { UserSettings: { myAvailableSetting: null } } };
    if (q.indexOf("mySetting") >= 0)
      return { data: { UserSettings: { mySetting: null } } };
    if (q.indexOf("setting(key") >= 0 && q.indexOf("accountIds") >= 0)
      return { data: { UserSettings: { setting: [] } } };
    if (q.indexOf("notificationSettings") >= 0)
      return { data: { Friends: { notificationSettings: { offline: null, success: true, message: null } }, PartySettings: { notificationSettings: { offline: null, success: true, message: null } } } };
    if (q.indexOf("allowedActions") >= 0)
      return { data: { LightSwitch: { serviceStatus: { allowedActions: ["PLAY"], status: "UP", banned: false } } } };
    if (q.indexOf("getMySocialBan") >= 0)
      return { data: { SocialBan: { summary: { warnings: [], bans: [] } } } };
    if (q.indexOf("summary(displayNames") >= 0)
      return { data: { Friends: { summary: { friends: [], incoming: [], outgoing: [], blocklist: [], suggestions: [] } } } };
    if (q.indexOf("blockList") >= 0) return { data: { Friends: { blockList: [] } } };
    if (q.indexOf("friend(friendId") >= 0) return { data: { Friends: { friend: null } } };
    if (q.indexOf("accounts(accountIds") >= 0) return { data: { Account: { accounts: [] } } };
    if (q.indexOf("friendshipStatus") >= 0) return { data: { Account: { account: null } } };
    if (q.indexOf("unlinkedDieselPlatforms") >= 0)
      return { data: { Account: { myAccount: { unlinkedDieselPlatforms: [] } } } };
    if (q.indexOf("myProfile") >= 0 && q.indexOf("character") >= 0)
      return { data: { Fortnite: { myProfile: { id: id, character: { templateId: "CID_029_Athena_Commando_F_Halloween", attributes: { favorite: false, item_seen: true, level: 1, variants: [] } }, backpack: { templateId: "" } } } } };
    if (q.indexOf("acceptEula") >= 0) return { data: { Eula: { acceptEula: { accepted: true } } } };
    if (q.indexOf("setNotificationSettings") >= 0)
      return { data: q.indexOf("PartySettings") >= 0 ? { PartySettings: { setNotificationSettings: { offline: null, success: true } } } : { Friends: { setNotificationSettings: { offline: null, success: true } } } };
    if (q.indexOf("reportPlayer") >= 0)
      return { data: { Fortnite: { reportPlayer: { success: true, status: "OK", errorCode: "", errorMessage: "" } } } };
    return { data: {} };
  }

  function owenRespondXHR(xhr, status, text) {
    var props = {
      readyState: 4,
      status: status,
      statusText: status === 200 ? "OK" : "Error",
      responseText: text,
      response: text,
      responseURL: (xhr.__owenReq && xhr.__owenReq.url) || "",
    };
    for (var k in props) {
      try {
        Object.defineProperty(xhr, k, { value: props[k], configurable: true, writable: true, enumerable: true });
      } catch (err) { /* ignore */ }
    }
    try {
      Object.defineProperty(xhr, "getAllResponseHeaders", {
        value: function () { return "content-type: application/json\r\n"; },
        configurable: true, writable: true,
      });
    } catch (err) { /* ignore */ }
    try { if (typeof xhr.onreadystatechange === "function") xhr.onreadystatechange(); } catch (err) {}
    try { xhr.dispatchEvent(new Event("readystatechange")); } catch (err) {}
    try { xhr.dispatchEvent(new ProgressEvent("load")); } catch (err) {}
    try { xhr.dispatchEvent(new ProgressEvent("loadend")); } catch (err) {}
    try { if (typeof xhr.onload === "function") xhr.onload(); } catch (err) {}
  }

  /* Forward one raw graphql body to the Owen backend. The server does the
   * query fingerprinting natively — we pass the raw query verbatim. */
  function owenForwardOne(bodyJson) {
    if (!nativeFetch) return Promise.resolve(localAnswer(bodyJson && bodyJson.query));
    var headers = { "content-type": "application/json" };
    if (session.token) headers["Authorization"] = "Bearer " + session.token;
    if (session.username) headers["X-Owen-User"] = session.username;
    else if (session.displayName) headers["X-Owen-User"] = session.displayName;
    return nativeFetch(window.OWEN_CONFIG.GRAPHQL_URL, {
      method: "POST",
      headers: headers,
      body: JSON.stringify(bodyJson && bodyJson.query !== undefined ? bodyJson : { query: String(bodyJson || "") }),
    })
      .then(function (res) {
        if (!res.ok) throw new Error("backend HTTP " + res.status);
        return res.json();
      })
      .catch(function (err) {
        try { console.warn("[owen-bridge] backend unreachable (" + (err && err.message) + ") — local fallback"); } catch (e) {}
        return localAnswer(bodyJson && bodyJson.query);
      });
  }

  function owenHandleGraphql(xhr, parsed) {
    Promise.resolve()
      .then(function () {
        if (Array.isArray(parsed)) {
          return Promise.all(parsed.map(function (item) { return owenForwardOne(item); }));
        }
        return owenForwardOne(parsed);
      })
      .then(function (result) {
        owenRespondXHR(xhr, 200, JSON.stringify(result));
      })
      .catch(function (err) {
        try { console.warn("[owen-bridge] graphql error:", err && err.message); } catch (e) {}
        owenRespondXHR(xhr, 200, JSON.stringify({ data: {} }));
      });
  }

  if (XHR && xhrOpen && xhrSend) {
    XHR.prototype.open = function (method, url) {
      this.__owenReq = { method: String(method || "GET"), url: String(url || "") };
      return xhrOpen.apply(this, arguments);
    };
    XHR.prototype.send = function (body) {
      var req = this.__owenReq;
      if (
        req &&
        typeof body === "string" &&
        body.length > 0 &&
        req.method.toUpperCase() === "POST" &&
        /graphql/i.test(req.url) &&
        req.url.indexOf(window.OWEN_CONFIG.GRAPHQL_URL) === -1
      ) {
        var parsed = null;
        try { parsed = JSON.parse(body); } catch (err) { parsed = null; }
        if (parsed && (parsed.query || (Array.isArray(parsed) && parsed.length))) {
          owenHandleGraphql(this, parsed);
          return; /* consumed — never hits the network directly */
        }
      }
      return xhrSend.apply(this, arguments);
    };
  }

  /* ------------------------------------------------------------------ *
   * 6. The Liquid Glass login page (design shared with /login).         *
   * ------------------------------------------------------------------ */
  var GLASS_CSS = [
    "#owen-login{position:fixed;inset:0;z-index:2147483000;display:flex;align-items:center;justify-content:center;",
    "overflow-y:auto;padding:24px 18px;font-family:'Rubik',system-ui,-apple-system,'Segoe UI',Roboto,sans-serif;",
    "background:#05060b;color:#f4f6fb;-webkit-tap-highlight-color:transparent;transition:opacity .35s ease;}",
    "#owen-login.owen-hide{opacity:0;pointer-events:none;}",
    "#owen-login *{box-sizing:border-box;margin:0;padding:0}",
    "#owen-login .aurora{position:fixed;inset:-20%;z-index:0;filter:blur(90px);opacity:.85;pointer-events:none}",
    "#owen-login .blob{position:absolute;border-radius:50%;mix-blend-mode:screen}",
    "#owen-login .b1{width:44vmax;height:44vmax;left:-8%;top:-12%;background:radial-gradient(circle at 35% 35%,rgba(139,92,246,.55),transparent 60%);animation:odrift1 26s ease-in-out infinite alternate}",
    "#owen-login .b2{width:38vmax;height:38vmax;right:-10%;top:6%;background:radial-gradient(circle at 60% 40%,rgba(236,72,153,.42),transparent 60%);animation:odrift2 31s ease-in-out infinite alternate}",
    "#owen-login .b3{width:34vmax;height:34vmax;left:16%;bottom:-16%;background:radial-gradient(circle at 50% 50%,rgba(45,212,191,.34),transparent 60%);animation:odrift3 37s ease-in-out infinite alternate}",
    "@keyframes odrift1{from{transform:translate(0,0) scale(1)}to{transform:translate(9vw,7vh) scale(1.14)}}",
    "@keyframes odrift2{from{transform:translate(0,0) scale(1.06)}to{transform:translate(-8vw,9vh) scale(.94)}}",
    "@keyframes odrift3{from{transform:translate(0,0) scale(.96)}to{transform:translate(7vw,-8vh) scale(1.1)}}",
    "#owen-login .ocard{position:relative;z-index:10;width:100%;max-width:420px;padding:32px 28px 26px;",
    "background:linear-gradient(160deg,rgba(255,255,255,.10),rgba(255,255,255,.03));border:1px solid rgba(255,255,255,.14);",
    "border-radius:26px;backdrop-filter:blur(28px) saturate(175%);-webkit-backdrop-filter:blur(28px) saturate(175%);",
    "box-shadow:0 30px 80px rgba(0,0,0,.55),inset 0 1px 0 rgba(255,255,255,.22);animation:ocardIn .7s cubic-bezier(.34,1.56,.64,1) both}",
    "@keyframes ocardIn{from{opacity:0;transform:translateY(26px) scale(.96);filter:blur(6px)}to{opacity:1;transform:none;filter:none}}",
    "#owen-login .obrand{display:flex;align-items:center;gap:14px;margin-bottom:22px}",
    "#owen-login .omark{width:52px;height:52px;flex:none;border-radius:17px;display:flex;align-items:center;justify-content:center;",
    "background:linear-gradient(135deg,#a78bfa,#f472b6);box-shadow:0 12px 34px rgba(167,139,250,.45),inset 0 1px 0 rgba(255,255,255,.5);",
    "animation:opulse 3.2s ease-in-out infinite}",
    "@keyframes opulse{0%,100%{box-shadow:0 12px 34px rgba(167,139,250,.45),inset 0 1px 0 rgba(255,255,255,.5)}50%{box-shadow:0 16px 48px rgba(244,114,182,.55),inset 0 1px 0 rgba(255,255,255,.5)}}",
    "#owen-login .omark svg{width:30px;height:30px}",
    "#owen-login .obrand b{display:block;font-size:23px;font-weight:800;letter-spacing:.4px}",
    "#owen-login .obrand span{font-size:11px;font-weight:700;letter-spacing:3.5px;background:linear-gradient(90deg,#a78bfa,#f472b6);",
    "-webkit-background-clip:text;background-clip:text;-webkit-text-fill-color:transparent;color:#a78bfa}",
    "#owen-login .otabs{position:relative;display:flex;background:rgba(255,255,255,.05);border:1px solid rgba(255,255,255,.09);border-radius:999px;padding:4px;margin-bottom:20px}",
    "#owen-login .otab{flex:1;position:relative;z-index:2;border:0;background:none;color:#a5adbe;cursor:pointer;font:inherit;font-size:13.5px;font-weight:700;padding:10px 0;border-radius:999px;transition:color .3s}",
    "#owen-login .otab.on{color:#0b0616}",
    "#owen-login .opill{position:absolute;z-index:1;top:4px;bottom:4px;border-radius:999px;background:linear-gradient(135deg,#a78bfa,#f472b6);",
    "box-shadow:0 6px 22px rgba(167,139,250,.5);transition:all .45s cubic-bezier(.34,1.56,.64,1)}",
    "#owen-login .ofld{position:relative;margin-bottom:13px}",
    "#owen-login .ofld input{width:100%;padding:14px 15px;border-radius:16px;font:inherit;font-size:15px;color:#f4f6fb;caret-color:#a78bfa;outline:none;",
    "background:rgba(255,255,255,.05);border:1px solid rgba(255,255,255,.09);transition:border-color .25s,box-shadow .25s}",
    "#owen-login[dir=rtl] .ofld input{direction:ltr;text-align:right}",
    "#owen-login .ofld input:focus{border-color:rgba(167,139,250,.65);background:rgba(255,255,255,.08);box-shadow:0 0 0 4px rgba(167,139,250,.14)}",
    "#owen-login .ofld label{position:absolute;inset-inline-start:15px;top:14px;color:#6b7280;font-size:15px;pointer-events:none;transition:all .22s cubic-bezier(.34,1.56,.64,1)}",
    "#owen-login .ofld input:focus+label,#owen-login .ofld input:not(:placeholder-shown)+label{top:-9px;font-size:11px;font-weight:700;",
    "padding:0 7px;border-radius:6px;color:#a78bfa;background:#0b0d16}",
    "#owen-login .obtn{position:relative;width:100%;height:52px;border:0;border-radius:999px;cursor:pointer;overflow:hidden;font:inherit;",
    "font-size:16px;font-weight:800;color:#fff;background:linear-gradient(135deg,#a78bfa,#f472b6);",
    "box-shadow:0 14px 42px rgba(167,139,250,.45),inset 0 1px 0 rgba(255,255,255,.4);transition:transform .25s cubic-bezier(.34,1.56,.64,1),box-shadow .3s}",
    "#owen-login .obtn:hover{transform:translateY(-2px)}",
    "#owen-login .obtn:active{transform:scale(.965)}",
    "#owen-login .obtn:disabled{opacity:.55;cursor:default;transform:none}",
    "#owen-login .oshine{position:absolute;top:0;bottom:0;width:44%;inset-inline-start:-60%;transform:skewX(-18deg);",
    "background:linear-gradient(105deg,transparent,rgba(255,255,255,.4),transparent);animation:oshine 2.8s ease-in-out infinite}",
    "@keyframes oshine{0%,55%{inset-inline-start:-60%}100%{inset-inline-start:130%}}",
    "#owen-login .obanner{display:none;margin-bottom:13px;padding:11px 13px;border-radius:14px;font-size:13px;line-height:1.6;animation:oshake .5s cubic-bezier(.34,1.56,.64,1)}",
    "#owen-login .obanner.show{display:block}",
    "#owen-login .obanner.err{background:rgba(251,113,133,.1);border:1px solid rgba(251,113,133,.4);color:#fda4af}",
    "@keyframes oshake{0%{opacity:0;transform:translateX(0)}25%{transform:translateX(-7px)}55%{transform:translateX(6px)}100%{opacity:1;transform:none}}",
    "#owen-login .oquick{text-align:center;margin-top:16px;font-size:13px;color:#6b7280}",
    "#owen-login .oquick a{color:#a78bfa;font-weight:700;text-decoration:none;cursor:pointer}",
    "#owen-login .ofoot{margin-top:18px;text-align:center;font-size:11px;color:#6b7280;line-height:1.7}",
    "#owen-login .olang{position:absolute;top:14px;inset-inline-end:14px;display:flex;align-items:center;padding:6px 12px;border-radius:999px;cursor:pointer;",
    "background:rgba(255,255,255,.07);border:1px solid rgba(255,255,255,.14);color:#a5adbe;font-size:12.5px;font-weight:600}",
    "@media (prefers-reduced-motion:reduce){#owen-login *{animation-duration:.001s!important}}",
  ].join("");

  var STR = {
    ar: { brand: "أوين", login: "دخول", reg: "حساب جديد", user: "اسم المستخدم أو البريد", pass: "كلمة المرور",
      email: "البريد الإلكتروني", uname: "اسم المستخدم", dname: "الاسم داخل اللعبة", pass2: "تأكيد كلمة المرور",
      go: "دخول", create: "إنشاء الحساب", quick: "دخول سريع؟", quickL: "بالاسم فقط",
      foot: "خادم أوين — PartyHub الأصلي", busy: "…",
      eName: "أدخل اسم المستخدم أو البريد أولاً.", ePass: "أدخل كلمة المرور.", eMatch: "كلمتا المرور غير متطابقتين.",
      eEmail: "صيغة البريد الإلكتروني غير صحيحة.", eUser: "اسم المستخدم: 3-16 حرفاً (أحرف وأرقام _ .)",
      eNameLen: "الاسم داخل اللعبة: 3-32 حرفاً.", eWeak: "كلمة المرور 8 أحرف على الأقل." },
    en: { brand: "OWEN", login: "Sign in", reg: "Create account", user: "Username or email", pass: "Password",
      email: "Email address", uname: "Username", dname: "In-game display name", pass2: "Confirm password",
      go: "Sign in", create: "Create account", quick: "Quick access?", quickL: "name only",
      foot: "Owen backend — the native PartyHub", busy: "…",
      eName: "Enter your username or email first.", ePass: "Enter your password.", eMatch: "Passwords do not match.",
      eEmail: "Invalid email address format.", eUser: "Username: 3-16 chars (letters, digits, _ .)",
      eNameLen: "Display name: 3-32 characters.", eWeak: "Password must be at least 8 characters." }
  };
  var uiLang = "ar";
  try { uiLang = localStorage.getItem(window.OWEN_CONFIG.LANG_KEY) || "ar"; } catch (err) {}
  function T() { return STR[uiLang] || STR.ar; }

  var glassLogin = null;
  var loginMode = "login";

  function buildGlassLogin() {
    if (document.getElementById("owen-login")) return;
    var style = document.createElement("style");
    style.id = "owen-login-style";
    style.textContent = GLASS_CSS;
    (document.head || document.documentElement).appendChild(style);

    var root = document.createElement("div");
    root.id = "owen-login";
    root.setAttribute("role", "dialog");
    root.setAttribute("aria-label", "Owen Launcher login");
    root.innerHTML = [
      '<div class="aurora"><div class="blob b1"></div><div class="blob b2"></div><div class="blob b3"></div></div>',
      '<div class="ocard">',
      '  <button class="olang" id="owen-lang" type="button">EN</button>',
      '  <div class="obrand">',
      '    <div class="omark" aria-hidden="true"><svg viewBox="0 0 24 24" fill="none"><path d="M12 2L21 8.2 17.4 21H6.6L3 8.2 12 2Z" stroke="#fff" stroke-width="1.7" stroke-linejoin="round"/><path d="M12 2v19M3.3 8.6 12 12l8.7-3.4M6.6 21 12 12l5.4 9" stroke="#fff" stroke-width="1" opacity=".75"/></svg></div>',
      '    <div><b id="owen-brand">OWEN</b><span>LAUNCHER</span></div>',
      '  </div>',
      '  <div class="otabs"><div class="opill" id="owen-pill"></div>',
      '    <button class="otab on" id="owen-tab-login" type="button"></button>',
      '    <button class="otab" id="owen-tab-reg" type="button"></button>',
      '  </div>',
      '  <div class="obanner err" id="owen-err" role="alert"></div>',
      '  <form id="owen-f-login" novalidate>',
      '    <div class="ofld"><input id="owen-l-user" type="text" placeholder=" " autocomplete="username" autocapitalize="none" spellcheck="false"><label id="owen-l-user-l"></label></div>',
      '    <div class="ofld"><input id="owen-l-pass" type="password" placeholder=" " autocomplete="current-password"><label id="owen-l-pass-l"></label></div>',
      '    <button class="obtn" id="owen-l-go" type="submit"><span class="oshine"></span><span id="owen-l-go-t"></span></button>',
      '    <p class="oquick"><span id="owen-quick"></span> <a id="owen-quick-a" type="button"></a></p>',
      '  </form>',
      '  <form id="owen-f-reg" style="display:none" novalidate>',
      '    <div class="ofld"><input id="owen-r-email" type="email" placeholder=" " autocomplete="email" dir="ltr"><label id="owen-r-email-l"></label></div>',
      '    <div class="ofld"><input id="owen-r-user" type="text" placeholder=" " autocomplete="username" autocapitalize="none" spellcheck="false" dir="ltr"><label id="owen-r-user-l"></label></div>',
      '    <div class="ofld"><input id="owen-r-name" type="text" placeholder=" " autocapitalize="none"><label id="owen-r-name-l"></label></div>',
      '    <div class="ofld"><input id="owen-r-pass" type="password" placeholder=" " autocomplete="new-password" dir="ltr"><label id="owen-r-pass-l"></label></div>',
      '    <div class="ofld"><input id="owen-r-pass2" type="password" placeholder=" " autocomplete="new-password" dir="ltr"><label id="owen-r-pass2-l"></label></div>',
      '    <button class="obtn" id="owen-r-go" type="submit"><span class="oshine"></span><span id="owen-r-go-t"></span></button>',
      '  </form>',
      '  <div class="ofoot" id="owen-foot"></div>',
      '</div>',
    ].join("");

    (document.body || document.documentElement).appendChild(root);
    var q = function (id) { return root.querySelector("#" + id); };
    glassLogin = { root: root };

    function applyLang() {
      var t = T();
      root.setAttribute("dir", uiLang === "ar" ? "rtl" : "ltr");
      root.setAttribute("lang", uiLang);
      q("owen-lang").textContent = uiLang === "ar" ? "EN" : "ع";
      q("owen-brand").textContent = t.brand;
      q("owen-tab-login").textContent = t.login;
      q("owen-tab-reg").textContent = t.reg;
      q("owen-l-user-l").textContent = t.user;
      q("owen-l-pass-l").textContent = t.pass;
      q("owen-r-email-l").textContent = t.email;
      q("owen-r-user-l").textContent = t.uname;
      q("owen-r-name-l").textContent = t.dname;
      q("owen-r-pass-l").textContent = t.pass;
      q("owen-r-pass2-l").textContent = t.pass2;
      q("owen-l-go-t").textContent = t.go;
      q("owen-r-go-t").textContent = t.create;
      q("owen-quick").textContent = t.quick;
      q("owen-quick-a").textContent = t.quickL;
      q("owen-foot").textContent = t.foot;
      requestAnimationFrame(movePill);
    }
    function movePill() {
      var pill = q("owen-pill");
      var on = loginMode === "login" ? q("owen-tab-login") : q("owen-tab-reg");
      if (!pill || !on) return;
      pill.style.insetInlineStart = on.offsetLeft + "px";
      pill.style.width = on.offsetWidth + "px";
    }
    glassLogin.applyLang = applyLang;
    glassLogin.movePill = movePill;

    q("owen-lang").addEventListener("click", function () {
      uiLang = uiLang === "ar" ? "en" : "ar";
      try { localStorage.setItem(window.OWEN_CONFIG.LANG_KEY, uiLang); } catch (err) {}
      applyLang();
    });

    function setMode(m) {
      if (loginMode === m) return;
      loginMode = m;
      q("owen-f-login").style.display = m === "login" ? "" : "none";
      q("owen-f-reg").style.display = m === "reg" ? "" : "none";
      q("owen-tab-login").className = m === "login" ? "otab on" : "otab";
      q("owen-tab-reg").className = m === "reg" ? "otab on" : "otab";
      hideBanner();
      movePill();
    }
    q("owen-tab-login").addEventListener("click", function () { setMode("login"); });
    q("owen-tab-reg").addEventListener("click", function () { setMode("reg"); });

    function banner(msg) {
      var el = q("owen-err");
      el.textContent = msg;
      el.classList.remove("show");
      void el.offsetWidth;
      el.classList.add("show");
    }
    function hideBanner() { q("owen-err").classList.remove("show"); }
    function busy(btn, on) {
      btn.disabled = on;
      var label = btn.querySelector("span:last-child");
      if (on) { btn.dataset.t = label.textContent; label.textContent = T().busy; }
      else if (btn.dataset.t) { label.textContent = btn.dataset.t; }
    }

    function afterAuth(account, token) {
      persistSession(account, token);
      var btns = [q("owen-l-go"), q("owen-r-go")];
      for (var i = 0; i < btns.length; i++) busy(btns[i], false);
      /* drive the Kairos state machine: the patched root mounts the full
       * PartyHub app as soon as the status reads "authenticated". */
      setTimeout(function () {
        loginChanged("LoggedIn", session.accountId || owenFallbackId());
        setStartupStatus(STARTUP_STATUS.AUTHENTICATED);
        notifyPartyJoined(1800);
        notifyPartyJoined(4200);
        setTimeout(function () {
          root.className = "owen-hide";
          setTimeout(function () { if (root.parentNode) root.parentNode.removeChild(root); }, 420);
        }, 460);
      }, 160);
    }

    function api(path, opts) {
      return nativeFetch(path, opts).then(function (res) {
        return res.json().catch(function () { return {}; }).then(function (j) {
          if (!res.ok) throw new Error(j.message || j.error_description || j.error || ("HTTP " + res.status));
          return j;
        });
      });
    }

    q("owen-f-login").addEventListener("submit", function (ev) {
      ev.preventDefault(); hideBanner();
      var t = T(), u = q("owen-l-user").value.trim(), p = q("owen-l-pass").value;
      if (!u) return banner(t.eName);
      if (!p) return banner(t.ePass);
      var btn = q("owen-l-go"); busy(btn, true);
      var body = new URLSearchParams({ grant_type: "password", username: u, password: p, token_type: "eg1" });
      api(window.OWEN_CONFIG.TOKEN_URL, {
        method: "POST",
        headers: { "Content-Type": "application/x-www-form-urlencoded" },
        body: body.toString(),
      }).then(function (j) {
        afterAuth({ accountId: j.account_id, displayName: j.displayName, username: u }, j.access_token);
      }).catch(function (ex) { busy(btn, false); banner(ex.message || "login failed"); });
    });

    q("owen-f-reg").addEventListener("submit", function (ev) {
      ev.preventDefault(); hideBanner();
      var t = T();
      var email = q("owen-r-email").value.trim().toLowerCase();
      var user = q("owen-r-user").value.trim();
      var dname = q("owen-r-name").value.trim() || user;
      var p1 = q("owen-r-pass").value, p2 = q("owen-r-pass2").value;
      if (!/^[^@\s]+@[^@\s]+\.[^@\s]{2,}$/.test(email)) return banner(t.eEmail);
      if (!/^[a-zA-Z0-9_.]{3,16}$/.test(user)) return banner(t.eUser);
      if (dname.length < 3 || dname.length > 32) return banner(t.eNameLen);
      if (p1.length < 8) return banner(t.eWeak);
      if (p1 !== p2) return banner(t.eMatch);
      var btn = q("owen-r-go"); busy(btn, true);
      api(window.OWEN_CONFIG.REGISTER_URL, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email: email, username: user, displayName: dname, password: p1 }),
      }).then(function (j) {
        if (j.access_token) afterAuth(j, j.access_token);
        else {
          var body = new URLSearchParams({ grant_type: "password", username: user, password: p1, token_type: "eg1" });
          return api(window.OWEN_CONFIG.TOKEN_URL, {
            method: "POST",
            headers: { "Content-Type": "application/x-www-form-urlencoded" },
            body: body.toString(),
          }).then(function (j2) {
            afterAuth({ accountId: j2.account_id, displayName: j2.displayName, username: user }, j2.access_token);
          });
        }
      }).catch(function (ex) { busy(btn, false); banner(ex.message || "registration failed"); });
    });

    q("owen-quick-a").addEventListener("click", function (ev) {
      ev.preventDefault(); hideBanner();
      var t = T(), name = window.prompt(t.user + ":");
      if (!name) return;
      var body = new URLSearchParams({ grant_type: "password", username: name.trim(), password: "owen", token_type: "eg1" });
      api(window.OWEN_CONFIG.TOKEN_URL, {
        method: "POST",
        headers: { "Content-Type": "application/x-www-form-urlencoded" },
        body: body.toString(),
      }).then(function (j) {
        afterAuth({ accountId: j.account_id, displayName: j.displayName, username: name.trim() }, j.access_token);
      }).catch(function (ex) { banner(ex.message || "login failed"); });
    });

    /* Prefill the last display name. */
    if (session.displayName) q("owen-l-user").value = session.username || session.displayName;
    applyLang();
    try { q("owen-l-user").focus(); } catch (err) {}
  }

  function showGlassLogin(reset) {
    if (glassLogin && glassLogin.root && glassLogin.root.parentNode) {
      glassLogin.root.className = "";
      if (reset && glassLogin.applyLang) glassLogin.applyLang();
      return;
    }
    buildGlassLogin();
  }

  /* ------------------------------------------------------------------ *
   * 7. Boot: restore the session or show the glass login.               *
   * ------------------------------------------------------------------ */
  function boot() {
    if (session.token && nativeFetch) {
      /* silent verify — on success skip the login page entirely */
      nativeFetch(window.OWEN_CONFIG.VERIFY_URL, { headers: { Authorization: "Bearer " + session.token } })
        .then(function (res) {
          if (res.ok) {
            sessionAutoLogin = true;
            if (!session.accountId) {
              return res.json().catch(function () { return {}; }).then(function (j) {
                if (j.accountId) {
                  session.accountId = j.accountId;
                  if (j.displayName) session.displayName = j.displayName;
                  try {
                    localStorage.setItem(window.OWEN_CONFIG.ACCOUNT_KEY, JSON.stringify({
                      accountId: session.accountId, displayName: session.displayName, username: session.username,
                    }));
                  } catch (err) {}
                }
              });
            }
            loginChanged("LoggedIn", session.accountId || owenFallbackId());
            setStartupStatus(STARTUP_STATUS.AUTHENTICATED);
            notifyPartyJoined(2500);
            notifyPartyJoined(5000);
          } else {
            clearSession();
            showGlassLogin(true);
          }
        })
        .catch(function () { showGlassLogin(true); });
    } else if (session.accountId) {
      /* offline restore: mount the app with the cached identity */
      sessionAutoLogin = true;
      loginChanged("LoggedIn", session.accountId);
      setStartupStatus(STARTUP_STATUS.AUTHENTICATED);
      notifyPartyJoined(2500);
      notifyPartyJoined(5000);
    } else {
      showGlassLogin(true);
    }
  }

  if (document.body) {
    boot();
  } else {
    document.addEventListener("DOMContentLoaded", boot);
  }

  /* Mark presence for debugging. */
  window.__owenBridge = {
    version: "4.0.0",
    session: session,
    config: window.OWEN_CONFIG,
    status: function () { return currentStatus; },
    showLogin: function () { showGlassLogin(true); },
  };
})();
