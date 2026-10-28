/*
 * Owen Bridge - offline entry enabler for the Fortnite "shippedweb" Kairos login page.
 *
 * WHAT THIS FIXES
 * ---------------
 * The offline entry bundle (main.kairos.*.js) renders a full-screen Epic Games logo
 * forever when it is opened outside the real Fortnite app. The cause is not a missing
 * GraphQL URL: the built-in mock bridge answers the "native/getstartupstatus" call
 * with {status:"authenticated"}, and "authenticated" is intentionally NOT a case in
 * the LoginContainer switch (init/authenticating/prelogin/postlogin/autherror/
 * error/networkoffline are). It falls through to the default branch, which is the
 * static LoadingContainer - the Epic logo - with no further transitions.
 *
 * HOW IT WORKS
 * ------------
 * The bundle talks to its host through a bridge:
 *   OUT (JS -> host): window.postMessage({id, type:"mockRequest", command, args}, "*")
 *                     when no window.JSBridge / window.ue / window.webkit exists,
 *   IN  (host -> JS): window.postMessage({id, type:"asyncResponse", payload, errors}, "*")
 *                     window.postMessage({type:"notification", command, args:[...]}, "*")
 * The bundle ships its own mock responder (configureMockBridge) that answers every
 * mockRequest after a 100 ms delay. Responses are single-shot: the pending callback
 * is deleted on first answer, so the FIRST response wins.
 *
 * This script registers a listener BEFORE the bundle loads and answers the critical
 * calls immediately (0 ms), beating the built-in mock:
 *   - native/getstartupstatus -> {status:"prelogin"}  (unlocks the real login UI)
 *   - native/login|register   -> completes the login flow and pushes the
 *                                oss.identity.loginchanged + startupstatuschanged
 *                                notifications exactly like the native app does,
 *   - the identity check is performed through the bundle's own exposed bridge
 *     function, which makes the bundle POST
 *       {"query":"query {Account { myAccount {id}}}"}
 *     to window.clientEnvConfig.EPIC_SERVICE_GRAPHQL_URL - i.e. YOUR backend
 *     receives the request.
 *
 * CONFIGURATION
 * -------------
 * Edit the window.clientEnvConfig block below (kept verbatim: http:// and https://
 * are both supported and are NEVER rewritten). Set the URLs to your backend.
 */

(function () {
  "use strict";

  /* ------------------------------------------------------------------ *
   * 1. Client environment - MUST be set before main.kairos.*.js loads.  *
   *    EPIC_SERVICE_GRAPHQL_URL is the single required key (the bundle  *
   *    throws "Failed to configure GraphQlService" without it).         *
   * ------------------------------------------------------------------ */
  window.clientEnvConfig = Object.assign(
    {
      /* Required - the identity/EULA GraphQL endpoint (Account.myAccount). */
      EPIC_SERVICE_GRAPHQL_URL: "/graphql",
      /* Optional service endpoints - same server in a Lawin-style setup. */
      EPIC_SERVICE_FRIENDS_URL: "/graphql",
      EPIC_SERVICE_PRESENCE_URL: "/graphql",
      EPIC_SERVICE_PARTY_URL: "/graphql",
      /* Keep false for local http backends (avoids credential-cookies issues). */
      EPIC_SERVICE_GRAPHQL_WITHCREDENTIALS: false,
      /* Environment flags. "local-gamedev" additionally enables the hidden
       * debug menu (all screens reachable from the UI). */
      EPIC_ENV: "production",
      EPIC_ENABLE_DEBUG_MENU: false,
    },
    window.clientEnvConfig || {}
  );

  /* Startup statuses understood by the bundle:
   *   init | authenticating | authenticated | prelogin | postlogin |
   *   autherror | error | networkoffline
   * prelogin  -> shows the "Played Fortnite before?" login screen.
   * postlogin -> shows the "Logging you in" screen (final entry state). */
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

  /* Local account used to satisfy the identity flow. Replace with the
   * account id of your backend user if you want deeper flows to match. */
  var OWEN_ACCOUNT = {
    UserId: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    DisplayName: "OwenTester",
  };

  /* ------------------------------------------------------------------ *
   * 2. Bridge helper functions.                                        *
   * ------------------------------------------------------------------ */
  function respond(id, payload) {
    window.postMessage(
      { id: id, type: "asyncResponse", payload: payload === undefined ? null : payload },
      "*"
    );
  }

  /* The bundle's built-in mock answers async commands (e.g. getloggedinuserid)
   * with a Promise as the postMessage payload, which throws DataCloneError
   * ("<Promise> could not be cloned"). Those late answers are ignored anyway
   * (responses are single-shot and ours arrive first), so defuse the payload
   * instead of letting the error surface. */
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
    notify("startupstatuschanged", [{ status: status }]);
  }

  function loginChanged(status, userId) {
    notify("oss.identity.loginchanged", [{ Status: status, UserId: userId || "" }]);
  }

  /* ------------------------------------------------------------------ *
   * 3. Command table. Answers returned here win the race against the   *
   *    bundle's built-in mock (which answers after 100 ms).            *
   * ------------------------------------------------------------------ */
  var COMMANDS = {
    /* THE FIX: the built-in mock answers "authenticated" which is a dead
     * end for the UI. "prelogin" renders the actual login screen. */
    getstartupstatus: function () {
      return { status: STARTUP_STATUS.PRELOGIN };
    },

    /* Login buttons on the entry screen. Emulate a successful native
     * Epic login: resolve the call, then drive the state machine the way
     * the Fortnite app does (loginchanged -> authenticated -> postlogin). */
    login: function () {
      setTimeout(function () {
        loginChanged("LoggedIn", OWEN_ACCOUNT.UserId);
        setStartupStatus(STARTUP_STATUS.AUTHENTICATED);
        setStartupStatus(STARTUP_STATUS.POSTLOGIN);
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

    /* Identity. Answering here gives the app a proper {UserId} instantly;
     * the built-in mock still runs its own implementation for the same call,
     * which performs the GraphQL POST against EPIC_SERVICE_GRAPHQL_URL -
     * i.e. the backend receives the request either way. */
    getloggedinuserid: function () {
      return { UserId: OWEN_ACCOUNT.UserId };
    },
    getuserauthtoken: function () {
      return { Token: "owen-offline-token" };
    },
    refreshauth: function () {
      return { Token: "owen-offline-token" };
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
   * 4. Listener registered before main.kairos.*.js - sees every        *
   *    mockRequest first and answers instantly.                        *
   * ------------------------------------------------------------------ */
  window.addEventListener("message", function (event) {
    var data = event.data;
    if (!data || typeof data !== "object" || data.type !== "mockRequest") return;
    var handler = COMMANDS[data.command];
    if (typeof handler === "function") {
      try {
        var parsed = typeof data.args === "string" ? JSON.parse(data.args || "null") : data.args;
        respond(data.id, handler(parsed) || {});
      } catch (e) {
        respond(data.id, null);
      }
    }
    /* Commands without a handler fall through to the bundle's own mock,
     * which covers voice, party, settings and analytics defaults. */
  });

  /* ------------------------------------------------------------------ *
   * 5. Identity check against the backend.                             *
   *    The bundle exposes window.__executeBridgeCallAsMsg - the same   *
   *    entry point the native layer uses. Calling "oss.identity /     *
   *    getloggedinuserid" executes the bundle's own implementation,    *
   *    which performs:                                                 *
   *      POST {clientEnvConfig.EPIC_SERVICE_GRAPHQL_URL}               *
   *      body: {"query":"query {Account { myAccount {id}}}"}           *
   *    i.e. the request lands on YOUR backend.                         *
   * ------------------------------------------------------------------ */
  function runIdentityCheck() {
    if (typeof window.__executeBridgeCallAsMsg !== "function") {
      /* Bundle not ready yet - retry shortly. */
      setTimeout(runIdentityCheck, 500);
      return;
    }
    try {
      window
        .__executeBridgeCallAsMsg("oss.identity", "getloggedinuserid")
        .then(function (userId) {
          /* eslint-disable no-console */
          console.log("[owen-bridge] identity check complete:", userId);
        })
        .catch(function (err) {
          console.log("[owen-bridge] identity check failed (backend offline?):", err);
        });
    } catch (e) {
      console.log("[owen-bridge] identity check error:", e);
    }
  }

  /* Run once the page has settled so the bridge listeners are ready. */
  if (document.readyState === "complete") {
    setTimeout(runIdentityCheck, 800);
  } else {
    window.addEventListener("load", function () {
      setTimeout(runIdentityCheck, 800);
    });
  }

  /* Mark presence for debugging. */
  window.__owenBridge = { version: "1.0.0" };
})();
