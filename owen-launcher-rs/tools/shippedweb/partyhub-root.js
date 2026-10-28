/*
 * partyhub-root.js — replacement body for webpack module 1041 of the shippedweb
 * offline-entry bundle (main.kairos.8af1b828dc6188736f50.js).
 *
 * WHY
 * ----
 * The stock offline entry mounts ONLY the LoginContainer as the app root.
 * Its postlogin branch renders the static "Logging you in / Checking your login
 * info..." screen forever, because handing off to the native app is what is
 * supposed to happen next. The full Kairos PartyHub UI (routes: home, party,
 * friends, settings, ...) ships inside the very same bundle but is never
 * mounted by this build's bootstrap.
 *
 * WHAT
 * ----
 * This replacement is a hybrid root:
 *   - while the startup status is init/authenticating/prelogin/autherror/error
 *     it renders the ORIGINAL login flow (unchanged behaviour), and
 *   - as soon as the status is "authenticated" (the Owen bridge pushes this
 *     right after a successful native-style login) it mounts the FULL app:
 *       useAppInitialization()   -> friends init + GetMyAccount (graphql ->
 *                                   the real backend) + initializationComplete
 *       AppEffects               -> lightswitch / content control / parties /
 *                                   presence / voice / friends effects (the
 *                                   graphql services hit the backend)
 *       useInitializers(AppStateInitializers) -> persisted settings load
 *       getRoutes(appRoutes)     -> the real PartyHub router (home, party,
 *                                   friends, settings, ... incl. modal routes)
 *
 * Module ids referenced (verified against this exact bundle):
 *   0 React, 7 hooks barrel (useAppInitialization/useInitializationComplete),
 *   9 useSharedValueEffects barrel, 54 StartupStatus, 74 react-router-dom
 *   (withRouter), 414 notice, 699 prelogin screen, 702 postlogin screen,
 *   418 autherror screen, 1060 AppEffects/OfflineBundleEffects, 120 AppState
 *   barrel (AppStateInitializers), 259 useStartupStatus, 513 useInitializers,
 *   1269 loading screen, 1275 css side-effect, 1277 App CSS module,
 *   1290 {appRoutes,getRoutes}, 1291 route definitions.
 */
function (e, t, n) {
  "use strict";
  Object.defineProperty(t, "__esModule", { value: !0 }), (t.default = void 0);
  var r = a(n(0)), i = n(9), o = n(1060), s = n(259), u = n(54),
      l = a(n(414)), c = a(n(1269)), d = a(n(699)), f = a(n(702)), p = a(n(418));
  n(1275);
  var h = a(n(1277)), m = n(7), v = n(1290), y = a(n(1291)), b = n(74),
      w = n(513), E = n(120);
  function a(e) { return e && e.__esModule ? e : { default: e }; }

  /* Full Kairos app: real router + real effects + real initialization. */
  var FullAppRoutes = function (props) {
    (0, m.useAppInitialization)();
    (0, i.useSharedValueEffects)(o.AppEffects);
    (0, w.useInitializers)(E.AppStateInitializers);
    var loc = props.location;
    var lastPlain = r.default.useRef(loc);
    var plain = !(loc && loc.state && loc.state.isModal);
    if (plain) { lastPlain.current = loc; }
    return r.default.createElement(
      "div", { className: h.default.routesContainer || "" },
      r.default.createElement(
        "div", { className: plain ? "" : (h.default.hideRoutes || "") },
        v.getRoutes(y.default(), lastPlain.current)
      ),
      plain ? null : r.default.createElement(
        "div", { className: h.default.modalOverlayFixed || h.default.modalOverlay || "" },
        v.getRoutes(y.modalRoutes, loc)
      )
    );
  };
  var FullApp = (0, b.withRouter)(FullAppRoutes);

  /* Original login flow (verbatim behaviour of the stock LoginContainer). */
  var LoginFlow = function () {
    (0, i.useSharedValueEffects)(o.OfflineBundleEffects);
    var e = (0, s.useStartupStatus)();
    var t = (function (e) {
      switch (e) {
        case u.StartupStatus.NetworkOffline:
          return { component: l.default, props: { noticeType: l.NoticeType.Connection, suppressAction: !0 } };
        case u.StartupStatus.Authenticating:
        case u.StartupStatus.Initializing:
          return { component: c.default };
        case u.StartupStatus.Prelogin:
          return { component: d.default };
        case u.StartupStatus.Postlogin:
          return { component: f.default };
        case u.StartupStatus.AuthError:
          return { component: p.default };
        case u.StartupStatus.Error:
          return { component: l.default, props: { noticeType: l.NoticeType.Error, reloadOnError: !0 } };
        default:
          return { component: c.default };
      }
    })(e);
    var comp = t.component;
    return r.default.createElement(
      "div", { className: h.default.container || "" },
      r.default.createElement(comp, t.props)
    );
  };

  /* Hybrid root: login flow until authenticated, then the full PartyHub app. */
  var OwenRoot = function () {
    (0, m.useInitializationComplete)();
    var e = (0, s.useStartupStatus)();
    console.log("LoginContainer status %s", e);
    if (e === u.StartupStatus.Authenticated || e === u.StartupStatus.Postlogin) {
      return r.default.createElement(FullApp);
    }
    return r.default.createElement(LoginFlow);
  };
  t.default = OwenRoot;
}
