/*
 * Owen Bridge v3 — Voltronite edition, with the Owen-Launcher golden login.
 *
 * WHAT'S IN v3
 * ------------
 * 1. GOLDEN LOGIN PAGE — the login screen of the Owen Launcher ("Aurum"
 *    theme: charcoal background, gold gradient button, player-name card)
 *    rendered as a full-screen page INSTEAD of the Kairos "Played Fortnite
 *    before?" screen. The name typed here becomes the Voltronite account
 *    (accountId = hex(name), exactly how Voltronite derives it).
 *
 * 2. VOLTRONITE LINKS — all backend endpoints live in OWEN_CONFIG below.
 *    The Kairos bundle speaks a RAW graphql dialect (no operationName, no
 *    Authorization) while Voltronite's /partyhub/graphql switches on
 *    body.operationName and requires a Bearer token. This bridge hooks
 *    XMLHttpRequest (the bundle's HTTP stack is axios-over-XHR) and:
 *      - fingerprints every raw query  -> maps it to a Voltronite operation
 *      - forwards {operationName, query, variables} + the Voltronite URL
 *        to the tiny same-origin relay (server.py or partyhub-web) which
 *        adds the real Bearer token (browsers cannot call Voltronite
 *        directly — it sends no CORS headers — so the relay is required)
 *      - applies the compatibility transforms the old client needs
 *        (kairos lightswitch UP, socialBan null) and answers the queries
 *        Voltronite does not implement (friends, eula body, character)
 *        locally with neutral defaults
 *      - falls back to local answers if the backend is unreachable, so
 *        the PartyHub UI always renders.
 *
 * 3. NO DEBUG BAR — v3 never injects any overlay except the login page.
 *
 * BRIDGE PROTOCOL (identical in the real app)
 *   OUT (JS -> host): window.postMessage({id, type:"mockRequest", command, args}, "*")
 *   IN  (host -> JS): window.postMessage({id, type:"asyncResponse", payload}, "*")
 *                     window.postMessage({type:"notification", command, args:[...]}, "*")
 */

(function () {
  "use strict";

  /* ================================================================== *
   * 1. CONFIG — الروابط — this is the only block you need to edit.     *
   * ================================================================== */
  window.OWEN_CONFIG = {
    /* Voltronite backend address. Every graphql request is forwarded here
     * through the local relay. If Voltronite runs on your PC and this page
     * runs on your phone, use the PC's LAN address, e.g.
     *     "http://192.168.1.20:3551"                                   */
    VOLTRONITE_URL: "http://127.0.0.1:3551",

    /* Default player name (used only if the login page is bypassed). The
     * name you type on the golden login page always wins. Voltronite
     * derives the account id as hex(username).                           */
    USERNAME: "OwenTester",

    /* Same-origin relay endpoint. "auto" derives it from window.appContext
     * ("/offline" -> "/offline/owen-graphql", "/partyhub/offline" ->
     * "/partyhub/offline/owen-graphql") so the request always reaches the
     * relay no matter which URL prefix the build is served under. Set an
     * explicit path here only if your relay lives somewhere unusual. */
    RELAY_URL: "auto",
  };

  /* ------------------------------------------------------------------ *
   * 2. Client environment — MUST be set before main.kairos.*.js loads.  *
   *    The URL below is what the bundle's GraphQlService is configured  *
   *    with; every request to it is intercepted in section 5.           *
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

  /* Resolve the relay URL once: "auto" -> <appContext>/owen-graphql so it
   * works both when served from the root (server.py, appContext "/offline")
   * and behind a path prefix (web console, appContext "/partyhub/offline"). */
  var RELAY_URL = (function () {
    var cfg = window.OWEN_CONFIG;
    if (cfg.RELAY_URL && cfg.RELAY_URL !== "auto") return cfg.RELAY_URL;
    var base = String(window.appContext || "/offline");
    if (base.charAt(0) !== "/") base = "/" + base;
    if (base.charAt(base.length - 1) === "/") base = base.slice(0, -1);
    return base + "/owen-graphql";
  })();

  /* Startup statuses understood by the bundle. */
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

  /* Local account — the CLIENT-SIDE id is the hex of the name LEFT-PADDED
   * with zeros to 32 chars (Epic account-id format). Padding is REQUIRED:
   * the Kairos home screen hashes the id with parseInt(id.slice(8,9),16) —
   * with a short id (hex of a 1-4 char name) that slice is empty, the hash
   * becomes NaN and the home screen crashes (backgroundColor undefined).
   * The padded id is what the app sees everywhere (login notifications,
   * getloggedinuserid, settings keys); Voltronite keeps deriving its own
   * short hex id from the oauth token, and the ids it returns are rewritten
   * to the padded form in the GetMyAccount extract below. */
  function owenHexId(name) {
    try {
      var bytes = new TextEncoder().encode(String(name));
      var out = "";
      for (var i = 0; i < bytes.length; i++)
        out += (bytes[i] < 16 ? "0" : "") + bytes[i].toString(16);
      return out;
    } catch (err) {
      return String(name);
    }
  }

  function owenClientId(name) {
    var h = owenHexId(name);
    while (h.length < 32) h = "0" + h;
    return h;
  }

  var account = {
    UserId: owenClientId(window.OWEN_CONFIG.USERNAME),
    DisplayName: window.OWEN_CONFIG.USERNAME,
  };

  /* Current startup status — the bridge is STATEFUL exactly like the
   * native layer (see partyhub-root.js). */
  var currentStatus = STARTUP_STATUS.PRELOGIN;

  /* ------------------------------------------------------------------ *
   * 3. Bridge helpers (postMessage protocol).                           *
   * ------------------------------------------------------------------ */
  function respond(id, payload) {
    window.postMessage(
      { id: id, type: "asyncResponse", payload: payload === undefined ? null : payload },
      "*"
    );
  }

  /* The bundle's built-in mock answers async commands with a Promise as
   * the postMessage payload, which throws DataCloneError. Defuse those
   * late answers (responses are single-shot, they are ignored anyway). */
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
    window.postMessage({ type: "notification", command: command, args: args }, "*");
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

    /* Login now happens on the GOLDEN page (section 6); these handlers
     * drive the state machine for it (and as a fallback path). */
    login: function () {
      setTimeout(function () {
        loginChanged("LoggedIn", account.UserId);
        setStartupStatus(STARTUP_STATUS.AUTHENTICATED);
      }, 150);
      return {};
    },
    register: function () {
      return COMMANDS.login();
    },
    RetryLogin: function () {
      return COMMANDS.login();
    },
    prelogin: function () {
      setTimeout(function () {
        setStartupStatus(STARTUP_STATUS.PRELOGIN);
      }, 100);
      return {};
    },
    logout: function () {
      setTimeout(function () {
        loginChanged("LoggedOut", "");
        setStartupStatus(STARTUP_STATUS.PRELOGIN);
        showGoldenLogin(true); /* back to the golden login page */
      }, 100);
      return {};
    },

    getloggedinuserid: function () {
      return { UserId: account.UserId };
    },
    getuserauthtoken: function () {
      return { Token: "owen-offline-token" };
    },
    refreshauth: function () {
      return { Token: "owen-offline-token" };
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
      } catch (err) {
        /* fall through */
      }
      return undefined; /* not handled -> built-in mock (localStorage) */
    },

    getplatforminfo: function () {
      return { Platform: "ANDROID", DeviceModel: "Owen", OSVersion: "13" };
    },
    getsafezone: function () {
      return { top: 0, right: 0, bottom: 0, left: 0, initialized: true };
    },
    getepicappversion: function () {
      return 1;
    },
    getgamesupportstatus: function () {
      return { supported: true };
    },
  };

  window.addEventListener("message", function (event) {
    var data = event.data;
    if (!data || typeof data !== "object" || data.type !== "mockRequest") return;
    var handler = COMMANDS[data.command];
    if (typeof handler !== "function") return;
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
    if (handled) {
      event.stopImmediatePropagation();
    }
  });

  /* ------------------------------------------------------------------ *
   * 5. GraphQL: XMLHttpRequest interception -> Voltronite.              *
   *    The bundle's HTTP stack is axios over XHR (NOT fetch), so this   *
   *    hook is the one that matters.                                    *
   * ------------------------------------------------------------------ */
  var nativeFetch = typeof window.fetch === "function" ? window.fetch.bind(window) : null;
  var XHR = window.XMLHttpRequest;
  var xhrOpen = XHR && XHR.prototype.open;
  var xhrSend = XHR && XHR.prototype.send;

  function hexUser() {
    return owenClientId(account.DisplayName || window.OWEN_CONFIG.USERNAME);
  }

  /* Rewrite Voltronite's short hex ids (myAccount/myProfile) into the padded
   * client-safe form the app expects. */
  function padIds(data) {
    try {
      var id = hexUser();
      if (data && data.Account && data.Account.myAccount)
        data.Account.myAccount.id = id;
      if (data && data.Fortnite && data.Fortnite.myProfile)
        data.Fortnite.myProfile.id = id;
    } catch (err) { /* keep raw */ }
    return data;
  }

  /* --- query fingerprinting -> Voltronite operation mapping ---------- */
  var norm = function (q) {
    return String(q || "").replace(/\s+/g, " ").trim();
  };

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

  function mapQuery(query) {
    var q = norm(query);
    var isMutation = /^mutation\b/.test(q);
    var user = account.DisplayName || window.OWEN_CONFIG.USERNAME;

    if (!isMutation) {
      /* LightSwitch: fortnite + kairos service status (+ social ban).
       * Voltronite answers kairosStatus DOWN — the old client needs UP. */
      if (q.indexOf("fortniteStatus") >= 0 && q.indexOf("kairosStatus") >= 0)
        return {
              label: "GetStatusPerService",
              op: "GetStatusPerService",
              extract: function (d) {
                try {
                  if (d && d.LightSwitch && d.LightSwitch.kairosStatus)
                    d.LightSwitch.kairosStatus.status = "UP";
                  if (d && d.SocialBan && !("socialBan" in d.SocialBan))
                    d.SocialBan.socialBan = null;
                } catch (err) { /* keep raw */ }
                return d;
              },
              fallback: localStatusAnswer(),
            };

      /* ContentControl voice permission. */
      if (q.indexOf("canUseVoiceChat") >= 0)
        return {
              label: "canVoiceChat",
              op: "GetStatusPerService",
              extract: function (d) {
                return { ContentControl: d && d.ContentControl };
              },
              fallback: { data: { ContentControl: { namespace: { result: { canUseVoiceChat: true } } } } },
            };

      /* Full account query (app initialization). */
      if (q.indexOf("myAccount") >= 0 && q.indexOf("externalAuths") >= 0)
        return {
              label: "GetMyAccount",
              op: "GetMyAccount",
              extract: padIds,
              fallback: {
                data: {
                  Account: { myAccount: { id: hexUser(), displayName: user, email: user + "@voltronite.local", country: "MA", externalAuths: [] } },
                  Fortnite: { myProfile: { id: hexUser() } },
                },
              },
            };

      /* The built-in mock's identity probe. */
      if (q.replace(/\s/g, "") === "query{Account{myAccount{id}}}")
        return { label: "identity-probe", op: "GetMyAccount", extract: padIds };

      if (q.indexOf("hasAccountAccepted") >= 0)
        return {
              label: "EulaAccepted",
              op: "EulaAccepted",
              fallback: { data: { Eula: { hasAccountAccepted: { accepted: true } } } },
            };

      if (q.indexOf("getLatestAndConvert") >= 0)
        return {
              label: "GetEula(local)",
              local: { data: { Eula: { getLatestAndConvert: { accepted: true, body: "", key: "fortnite", locale: "en-US", title: "", version: 1 } } } },
            };

      if (q.indexOf("myAvailableSetting") >= 0)
        return { label: "myAvailableSetting(local)", local: { data: { UserSettings: { myAvailableSetting: null } } } };

      if (q.indexOf("mySetting") >= 0)
        return {
              label: "GetMySetting",
              op: "GetMySetting",
              fallback: { data: { UserSettings: { mySetting: null } } },
            };

      if (q.indexOf("setting(key") >= 0 && q.indexOf("accountIds") >= 0)
        return {
              label: "GetAccountSettings",
              op: "GetAccountSettings",
              fallback: { data: { UserSettings: { setting: [] } } },
            };

      if (q.indexOf("notificationSettings") >= 0)
        return {
              label: "GetNotificationSettings",
              op: "GetNotificationSettings",
              fallback: {
                data: {
                  Friends: { notificationSettings: { offline: null, success: true, message: null } },
                  PartySettings: { notificationSettings: { offline: null, success: true, message: null } },
                },
              },
            };

      /* Single-service lightswitch probe. */
      if (q.indexOf("allowedActions") >= 0)
        return {
              label: "serviceStatus(local)",
              local: { data: { LightSwitch: { serviceStatus: { allowedActions: ["PLAY"], status: "UP", banned: false } } } },
            };

      if (q.indexOf("getMySocialBan") >= 0 && q.indexOf("fortniteStatus") < 0)
        return {
              label: "getMySocialBanSummary",
              op: "getMySocialBanSummary",
              fallback: { data: { SocialBan: { summary: { warnings: [], bans: [] } } } },
            };

      /* Friends. */
      if (q.indexOf("summary(displayNames") >= 0)
        return {
              label: "friends-summary(local)",
              local: { data: { Friends: { summary: { friends: [], incoming: [], outgoing: [], blocklist: [], suggestions: [] } } } },
            };
      if (q.indexOf("blockList") >= 0)
        return { label: "blockList(local)", local: { data: { Friends: { blockList: [] } } } };
      if (q.indexOf("friend(friendId") >= 0)
        return { label: "getFriend(local)", local: { data: { Friends: { friend: null } } } };

      /* Account lookups. */
      if (q.indexOf("accounts(accountIds") >= 0)
        return { label: "accounts(local)", local: { data: { Account: { accounts: [] } } } };
      if (q.indexOf("friendshipStatus") >= 0)
        return { label: "account(local)", local: { data: { Account: { account: null } } } };
      if (q.indexOf("unlinkedDieselPlatforms") >= 0)
        return { label: "unlinkedPlatforms(local)", local: { data: { Account: { myAccount: { unlinkedDieselPlatforms: [] } } } } };

      /* Fortnite profile: powers the PartyHub avatar. */
      if (q.indexOf("myProfile") >= 0 && q.indexOf("character") >= 0)
        return {
              label: "myProfile-character(local)",
              local: {
                data: {
                  Fortnite: {
                    myProfile: {
                      id: hexUser(),
                      character: {
                        templateId: "CID_029_Athena_Commando_F_Halloween",
                        attributes: { favorite: false, item_seen: true, level: 1, variants: [] },
                      },
                      backpack: { templateId: "" },
                    },
                  },
                },
              },
            };
    } else {
      if (q.indexOf("updateSetting") >= 0)
        return {
              label: "UpdateUserSetting",
              op: "UpdateUserSetting",
              fallback: { data: { UserSettings: { updateSetting: { success: true } } } },
            };
      if (q.indexOf("acceptEula") >= 0)
        return { label: "AcceptEula(local)", local: { data: { Eula: { acceptEula: { accepted: true } } } } };
      if (q.indexOf("setNotificationSettings") >= 0)
        return {
              label: "setNotificationSettings(local)",
              local: {
                data: q.indexOf("PartySettings") >= 0
                  ? { PartySettings: { setNotificationSettings: { offline: null, success: true } } }
                  : { Friends: { setNotificationSettings: { offline: null, success: true } } },
              },
            };
      if (q.indexOf("reportPlayer") >= 0)
        return {
              label: "reportPlayer(local)",
              local: { data: { Fortnite: { reportPlayer: { success: true, status: "OK", errorCode: "", errorMessage: "" } } } },
            };
      return { label: "generic-mutation(local)", local: { data: { success: true } } };
    }

    return { label: "unknown(local)", local: { data: {} } };
  }

  /* --- simulate an XHR response on an opened-but-never-sent XHR ------ */
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
        Object.defineProperty(xhr, k, {
          value: props[k], configurable: true, writable: true, enumerable: true,
        });
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

  /* --- forward one raw graphql body to Voltronite via the relay ------ */
  function owenForwardOne(bodyJson) {
    var query = String((bodyJson && bodyJson.query) || "");
    var mapping = mapQuery(query);
    if (mapping.local) return Promise.resolve(mapping.local);

    if (!nativeFetch) return Promise.resolve(mapping.fallback || { data: {} });

    return nativeFetch(RELAY_URL, {
      method: "POST",
      headers: {
        "content-type": "application/json",
        "x-owen-backend": String(window.OWEN_CONFIG.VOLTRONITE_URL || ""),
        "x-owen-user": String(account.DisplayName || window.OWEN_CONFIG.USERNAME || ""),
      },
      body: JSON.stringify({
        operationName: mapping.op,
        query: query,
        variables: (bodyJson && bodyJson.variables) || {},
      }),
    })
      .then(function (res) {
        if (!res.ok) throw new Error("relay HTTP " + res.status);
        return res.json();
      })
      .then(function (json) {
        var data = mapping.extract ? mapping.extract(json && json.data) : json && json.data;
        return { data: data };
      })
      .catch(function (err) {
        try {
          console.warn("[owen-bridge] " + mapping.label + " relay failed (" + (err && err.message) + ") — local fallback");
        } catch (e) {}
        return mapping.fallback || { data: {} };
      });
  }

  function owenHandleGraphql(xhr, parsed) {
    Promise.resolve()
      .then(function () {
        if (Array.isArray(parsed)) {
          return Promise.all(
            parsed.map(function (item) { return owenForwardOne(item); })
          );
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
        req.url !== RELAY_URL
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
   * 6. The golden login page — Owen Launcher "Aurum" design.            *
   *    (Charcoal background, gold gradient button, player-name card.)   *
   * ------------------------------------------------------------------ */
  var GOLDEN_CSS = [
    "#owen-login{position:fixed;inset:0;z-index:2147483000;display:flex;align-items:center;justify-content:center;",
    "background:linear-gradient(180deg,#1A140C 0%,#141009 35%,#141009 100%);",
    "font-family:'Segoe UI',Roboto,system-ui,-apple-system,sans-serif;",
    "-webkit-tap-highlight-color:transparent;transition:opacity .35s ease;}",
    "#owen-login.owen-hide{opacity:0;pointer-events:none;}",
    "#owen-login *{box-sizing:border-box;margin:0;padding:0;}",
    ".owen-inner{width:100%;max-width:400px;padding:28px 22px;}",
    ".owen-brand-owen{color:#FFFFFF;font-weight:900;font-size:46px;line-height:1;letter-spacing:1px;}",
    ".owen-brand-launcher{font-size:30px;font-weight:900;letter-spacing:5px;line-height:1.15;",
    "background:linear-gradient(90deg,#FFE9B8,#FFC24B,#FF9F0A);-webkit-background-clip:text;",
    "background-clip:text;-webkit-text-fill-color:transparent;color:#FFC24B;}",
    ".owen-tagline{color:#D6C5AC;font-size:15px;margin-top:8px;}",
    ".owen-card{background:#211A11;border:1px solid rgba(255,194,75,.2);border-radius:16px;padding:16px;margin-top:20px;",
    "box-shadow:0 10px 34px rgba(0,0,0,.45);}",
    ".owen-card-head{display:flex;align-items:center;justify-content:space-between;}",
    ".owen-card-title{color:#FFF6E9;font-weight:700;font-size:16px;}",
    ".owen-pill{background:rgba(255,194,75,.15);color:#FFC24B;border-radius:8px;padding:3px 10px;",
    "font-size:11px;font-weight:700;line-height:1.5;}",
    ".owen-pill.owen-ok{background:rgba(127,232,164,.15);color:#7FE8A4;}",
    ".owen-field{width:100%;margin-top:12px;background:transparent;border:1px solid rgba(255,246,233,.2);",
    "border-radius:8px;padding:14px;font-size:16px;color:#FFF6E9;caret-color:#FFC24B;outline:none;",
    "-webkit-user-select:text;user-select:text;font-family:inherit;}",
    ".owen-field:focus{border-color:#FFC24B;}",
    ".owen-field::placeholder{color:#9C8C74;}",
    ".owen-note{color:#9C8C74;font-size:12px;margin-top:8px;line-height:1.5;}",
    ".owen-btn{display:block;width:100%;height:52px;margin-top:14px;border:none;border-radius:12px;cursor:pointer;",
    "background:linear-gradient(90deg,#FFB300,#FF9F0A,#FF8A00);color:#FFF6E0;font-weight:700;font-size:16px;",
    "font-family:inherit;transition:opacity .2s ease;}",
    ".owen-btn:disabled{opacity:.45;cursor:default;}",
    ".owen-backend{color:#9C8C74;font-size:11px;text-align:center;margin-top:16px;line-height:1.6;",
    "word-break:break-all;}",
    ".owen-backend b{color:#D6C5AC;font-weight:600;}",
  ].join("");

  var goldenLogin = null;

  function buildGoldenLogin() {
    if (document.getElementById("owen-login")) return;

    var style = document.createElement("style");
    style.id = "owen-login-style";
    style.textContent = GOLDEN_CSS;
    (document.head || document.documentElement).appendChild(style);

    var root = document.createElement("div");
    root.id = "owen-login";
    root.setAttribute("role", "dialog");
    root.setAttribute("aria-label", "Owen Launcher login");
    root.innerHTML = [
      '<div class="owen-inner">',
      '  <div class="owen-brand">',
      '    <div class="owen-brand-owen">OWEN</div>',
      '    <div class="owen-brand-launcher">LAUNCHER</div>',
      '  </div>',
      '  <div class="owen-tagline">Party Hub &mdash; relive the old days</div>',
      '  <div class="owen-card">',
      '    <div class="owen-card-head">',
      '      <div class="owen-card-title">Player name</div>',
      '      <div class="owen-pill" id="owen-pill">Required</div>',
      '    </div>',
      '    <input class="owen-field" id="owen-name" type="text" maxlength="32"',
      '      placeholder="Type your name" autocomplete="username" autocapitalize="none"',
      '      spellcheck="false" enterkeyhint="go" aria-label="Player name" />',
      '    <div class="owen-note">The name is sent to your Voltronite backend as the account name.</div>',
      '    <button class="owen-btn" id="owen-btn" type="button" disabled>LOGIN</button>',
      '  </div>',
      '  <div class="owen-backend">Voltronite backend: <b id="owen-url"></b></div>',
      '</div>',
    ].join("");

    var input = root.querySelector("#owen-name");
    var btn = root.querySelector("#owen-btn");
    var pill = root.querySelector("#owen-pill");

    root.querySelector("#owen-url").textContent = String(window.OWEN_CONFIG.VOLTRONITE_URL);

    var saved = null;
    try { saved = window.localStorage.getItem("owen:playername"); } catch (err) { saved = null; }
    if (saved) input.value = saved;
    refreshPill();

    function refreshPill() {
      var ok = input.value.trim().length > 0;
      pill.textContent = ok ? "OK" : "Required";
      pill.className = "owen-pill" + (ok ? " owen-ok" : "");
      btn.disabled = !ok;
    }

    input.addEventListener("input", refreshPill);
    input.addEventListener("keydown", function (ev) {
      if (ev.key === "Enter") { ev.preventDefault(); if (!btn.disabled) doLogin(); }
    });
    btn.addEventListener("click", function () { if (!btn.disabled) doLogin(); });

    function doLogin() {
      var name = input.value.trim().slice(0, 32);
      if (!name) return;
      try { window.localStorage.setItem("owen:playername", name); } catch (err) {}
      account.DisplayName = name;
      account.UserId = owenClientId(name);
      btn.disabled = true;
      btn.textContent = "LOGGING IN\u2026";
      /* drive the Kairos state machine: the patched root mounts the full
       * PartyHub app as soon as the status reads "authenticated". */
      setTimeout(function () {
        loginChanged("LoggedIn", account.UserId);
        setStartupStatus(STARTUP_STATUS.AUTHENTICATED);
        setTimeout(function () {
          root.className = "owen-hide";
          setTimeout(function () {
            if (root.parentNode) root.parentNode.removeChild(root);
          }, 400);
        }, 450);
      }, 150);
    }

    (document.body || document.documentElement).appendChild(root);
    goldenLogin = { root: root, input: input, btn: btn, pill: pill };
    try { input.focus(); } catch (err) {}
  }

  /* Re-show the golden login (used on logout). */
  function showGoldenLogin(reset) {
    if (goldenLogin && goldenLogin.root.parentNode) {
      goldenLogin.root.className = "";
      if (reset) {
        goldenLogin.btn.disabled = false;
        goldenLogin.btn.textContent = "LOGIN";
      }
      return;
    }
    buildGoldenLogin();
    if (reset && goldenLogin) {
      goldenLogin.btn.disabled = false;
      goldenLogin.btn.textContent = "LOGIN";
    }
  }

  if (document.body) {
    buildGoldenLogin();
  } else {
    document.addEventListener("DOMContentLoaded", buildGoldenLogin);
  }

  /* Mark presence for debugging. */
  window.__owenBridge = {
    version: "3.0.0",
    account: account,
    config: window.OWEN_CONFIG,
    showLogin: function () { showGoldenLogin(true); },
  };
})();
