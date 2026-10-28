/*
 * Owen Bridge v2 — offline entry enabler + PartyHub unlocker for the Fortnite
 * "shippedweb" Kairos bundle.
 *
 * WHAT THIS FIXES (v1)
 * ---------------------
 * The offline entry bundle renders a full-screen Epic Games logo forever when
 * opened outside the real Fortnite app: the built-in mock bridge answers the
 * "native/getstartupstatus" call with {status:"authenticated"} and
 * "authenticated" is intentionally NOT a case in the LoginContainer switch.
 * Answering {status:"prelogin"} instead unlocks the real login UI.
 *
 * WHAT THIS ADDS (v2)
 * --------------------
 * The stock entry ends on the "Checking your login info..." screen and waits
 * for the native app to take over — the PartyHub UI is never mounted by this
 * build (patching webpack module 1041 with tools/shippedweb/partyhub-root.js
 * fixes that). This bridge drives the state machine exactly the way the
 * patched root expects:
 *
 *   login  ->  oss.identity.loginchanged(LoggedIn, <id>)
 *          ->  startupstatuschanged(authenticated)      [STOPS HERE — no
 *              "postlogin", so the app mounts the full PartyHub router while
 *              the status is still "authenticated" and the once-on-
 *              authenticated initialization chain fires]
 *
 *   getconfiguredbackend -> {appName:"fortnite"}  (the LightSwitch service
 *              queries this BEFORE its graphql call)
 *   getloggedinuserid    -> {UserId:<hex(OwenTester)>} — MUST equal the
 *              accountId the backend returns for the same username, so the
 *              persisted-settings keys line up with the real account.
 *   getsetting           -> seeded values for <accountId>:onboardingComplete
 *              and <accountId>:introVideoWatched so the home screen skips the
 *              first-run intro and renders the hub directly. Every other key
 *              falls through to the bundle's own mock (localStorage-backed).
 *
 * BRIDGE PROTOCOL (identical in the real app)
 *   OUT (JS -> host): window.postMessage({id, type:"mockRequest", command, args}, "*")
 *   IN  (host -> JS): window.postMessage({id, type:"asyncResponse", payload}, "*")
 *                     window.postMessage({type:"notification", command, args:[...]}, "*")
 * Responses are single-shot (first answer wins); the built-in mock answers
 * after 100 ms, this script answers immediately.
 *
 * CONFIGURATION
 * -------------
 * window.clientEnvConfig.EPIC_SERVICE_GRAPHQL_URL must point at your backend
 * (relative "./graphql" works when the serving host exposes the graphql
 * endpoint on every SPA route — see mini-services/partyhub-web).
 */

(function () {
  "use strict";

  /* ------------------------------------------------------------------ *
   * 1. Client environment - MUST be set before main.kairos.*.js loads.  *
   * ------------------------------------------------------------------ */
  window.clientEnvConfig = Object.assign(
    {
      /* Identity / EULA / lightswitch / settings graphql endpoint.
       * Relative: resolves against the current SPA route, so any path the
       * router lands on keeps hitting the same adapter. */
      EPIC_SERVICE_GRAPHQL_URL: "./graphql",
      /* Keep false for local http backends (avoids credential-cookies issues). */
      EPIC_SERVICE_GRAPHQL_WITHCREDENTIALS: false,
      EPIC_ENV: "production",
      EPIC_ENABLE_DEBUG_MENU: false,
    },
    window.clientEnvConfig || {}
  );

  /* Startup statuses understood by the bundle:
   *   init | authenticating | authenticated | prelogin | postlogin |
   *   autherror | error | networkoffline */
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

  /* Local account. UserId MUST match the account id the backend reports for
   * the same username (Voltronite: accountId = hex(username)). */
  var OWEN_ACCOUNT = {
    UserId: "4f77656e546573746572", /* hex("OwenTester") */
    DisplayName: "OwenTester",
  };

  /* Current startup status — the bridge is STATEFUL exactly like the native
   * layer: re-queries of getstartupstatus (the FullApp re-registers the
   * startup-state effect when it mounts) must return the CURRENT status,
   * otherwise the app would fall back to the login screen right after
   * logging in. */
  var currentStatus = STARTUP_STATUS.PRELOGIN;

  /* ------------------------------------------------------------------ *
   * 2. Bridge helpers.                                                  *
   * ------------------------------------------------------------------ */
  function respond(id, payload) {
    window.postMessage(
      { id: id, type: "asyncResponse", payload: payload === undefined ? null : payload },
      "*"
    );
  }

  /* The bundle's built-in mock answers async commands with a Promise as the
   * postMessage payload, which throws DataCloneError. Defuse those late
   * answers (they are ignored anyway — responses are single-shot). */
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
   * 3. Command table — answers win the race against the built-in mock.  *
   * ------------------------------------------------------------------ */
  var COMMANDS = {
    /* THE v1 FIX: "prelogin" renders the real login screen instead of the
     * dead-end "authenticated" case (the Epic logo). STATEFUL: returns the
     * current status so re-queries after login keep the PartyHub mounted. */
    getstartupstatus: function () {
      return { status: currentStatus };
    },

    /* Login buttons. v2: drive to "authenticated" and STOP — the patched
     * root mounts the full PartyHub app at that point, and the
     * once-on-authenticated initialization chain (friends + GetMyAccount +
     * initializationComplete) fires while the status still reads
     * "authenticated". Pushing "postlogin" too would race past that trigger. */
    login: function () {
      setTimeout(function () {
        loginChanged("LoggedIn", OWEN_ACCOUNT.UserId);
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
      }, 100);
      return {};
    },

    /* Identity: the id MUST match the backend account (see OWEN_ACCOUNT). */
    getloggedinuserid: function () {
      return { UserId: OWEN_ACCOUNT.UserId };
    },
    getuserauthtoken: function () {
      return { Token: "owen-offline-token" };
    },
    refreshauth: function () {
      return { Token: "owen-offline-token" };
    },

    /* The LightSwitch service calls this BEFORE its graphql query. */
    getconfiguredbackend: function () {
      return { appName: "fortnite", environment: "production", gameBackend: "fortnite" };
    },

    /* Persisted settings: seed first-run values so the hub renders in its
     * final state. Any other key falls through to the built-in mock. */
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

    /* Platform info (not present in the built-in mock table). */
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

  /* ------------------------------------------------------------------ *
   * 4. Listener registered before main.kairos.*.js — sees every         *
   *    mockRequest first and answers instantly. Handled commands are    *
   *    consumed with stopImmediatePropagation so the bundle's built-in  *
   *    mock never runs them: its login/register/logout handlers would   *
   *    otherwise navigate the page away (location.href = appContext +   *
   *    "/" + command) whenever appContext is not literally "offline".   *
   * ------------------------------------------------------------------ */
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
      /* keep the built-in mock from executing the same command */
      event.stopImmediatePropagation();
      if (console && console.debug) {
        try { console.debug("[owen-bridge] answered native/" + data.command); } catch (e) {}
      }
    }
    /* Commands without a handler (or returning undefined) fall through to
     * the bundle's own mock — it covers voice, party, settings and
     * analytics defaults (and persists settings in localStorage). */
  });

  /* Mark presence for debugging. */
  window.__owenBridge = { version: "2.0.0", account: OWEN_ACCOUNT };
})();
