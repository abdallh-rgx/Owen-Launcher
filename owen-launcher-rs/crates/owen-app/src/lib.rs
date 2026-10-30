//! owen-app — the Owen Launcher application.
//!
//! One codebase, three hosts:
//! * **Android** (`libowenapp.so` in the APK): eframe on a NativeActivity
//!   (no Java app code — the only dex class is the install-result receiver
//!   that calls back into this library), full pipeline: permissions,
//!   Uptodown download → owen-patcher → PackageInstaller → deep-link launch.
//! * **Desktop** (`owen-app` bin): same app, in-process Owen Server.
//! * **wasm** (the preview build): the same app driving the hosted Owen
//!   Server through the supervisor gateway (`?XTransformPort=3010`).
//!
//! UI: egui. Theme: charcoal + gold (the original app's Aurum palette).
//! All user-facing strings come through the 15-language i18n.

#![cfg_attr(not(target_os = "android"), forbid(unsafe_code))]

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    let mut options = base_options();
    options.android_app = Some(app);
    let _ = eframe::run_native(
        "Owen Launcher",
        options,
        Box::new(|cc| Ok(Box::new(OwenApp::new(cc)))),
    );
}

mod platform;
#[cfg(target_os = "android")]
mod jni_android;
mod ui;

use owen_launcher::backend::{self, AccountSession, HostingStatus};
use owen_launcher::i18n::{I18n, LANGS};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/* ------------------------------- app state ------------------------------- */

#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    Boot,
    Language,
    Setup,
    Auth,
    Shell,
    Details(u32 /* season */),
}

#[derive(Debug, Clone, Default)]
pub struct Prefs {
    pub lang: Option<String>,
    pub dark: bool,
}

pub struct AppData {
    pub prefs: Prefs,
    pub account: Option<AccountSession>,
    pub hosting: ui::HostingPanel,
    pub redirecter: ui::RedirecterState,
    pub seasons: Option<serde_json::Value>,
    pub me: Option<serde_json::Value>,
    pub stats: Option<serde_json::Value>,
}

pub struct OwenApp {
    pub i18n: I18n,
    pub phase: Phase,
    pub view: ui::View,
    pub data: AppData,
    pub host: platform::Host,
    pub data_root: PathBuf,
    pub bundled: PathBuf,
    pub player_name: String,
    pub tex: HashMap<u32, egui::TextureHandle>,
    pub status: String,
    pub auth_tab: usize,
    pub auth_user: String,
    pub auth_pass: String,
    pub auth_email: String,
    pub auth_user_name: String,
    pub auth_display: String,
    pub auth_confirm: String,
    pub auth_error: Option<String>,
    pub auth_busy: bool,
    pub install_state: std::sync::Arc<Mutex<owen_launcher::uptodown::installer::InstallState>>,
    pub install_busy: bool,
    pub details_season: u32,
    pub boot_done: AtomicBool,
}

impl OwenApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // charcoal + gold theme
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgb(0x14, 0x10, 0x09);
        visuals.window_fill = egui::Color32::from_rgb(0x1e, 0x1a, 0x12);
        visuals.extreme_bg_color = egui::Color32::from_rgb(0x0b, 0x0b, 0x0f);
        widgets_gold(&mut visuals);
        cc.egui_ctx.set_visuals(visuals);

        let host = platform::host();
        let data_root = platform::data_root();
        let bundled = platform::bundled_dir();
        owen_launcher::core::diag::init(&data_root.join("logs"));
        owen_launcher::core::diag::section(&format!("Owen Launcher {APP_VERSION} — {host:?}"));
        owen_launcher::core::diag::i("app", &format!("data root: {}", data_root.display()));
        owen_launcher::core::diag::i("app", &format!("bundled:   {}", bundled.display()));
        #[cfg(not(target_arch = "wasm32"))]
        backend::set_bundled_hint(bundled.clone());

        let prefs = load_prefs(&data_root);
        let account = backend::load_account(&data_root);
        let lang = prefs.lang.clone().unwrap_or_else(|| "ar".to_string());
        let phase = if prefs.lang.is_none() {
            Phase::Language
        } else if host.needs_setup(&data_root) {
            Phase::Setup
        } else if account.is_none() {
            Phase::Auth
        } else {
            Phase::Shell
        };
        let player_name = owen_launcher::core::player_store::get(&data_root);
        Self {
            i18n: I18n::new(&lang),
            phase,
            view: ui::View::default(),
            data: AppData {
                prefs,
                account,
                hosting: ui::HostingPanel::default(),
                redirecter: ui::RedirecterState::default(),
                seasons: None,
                me: None,
                stats: None,
            },
            host,
            data_root,
            bundled,
            player_name,
            tex: HashMap::new(),
            status: String::new(),
            auth_tab: 0,
            auth_user: String::new(),
            auth_pass: String::new(),
            auth_email: String::new(),
            auth_user_name: String::new(),
            auth_display: String::new(),
            auth_confirm: String::new(),
            auth_error: None,
            auth_busy: false,
            install_state: std::sync::Arc::new(Mutex::new(
                owen_launcher::uptodown::installer::InstallState::Idle,
            )),
            install_busy: false,
            details_season: 5,
            boot_done: AtomicBool::new(false),
        }
    }

    /// Things that must happen once per launch on native hosts
    /// (the original `MainActivity.onCreate` background block).
    pub fn boot_native(&self) {
        if self.host.is_wasm() {
            return;
        }
        owen_launcher::core::lawin::deployer::ensure_deployed_blocking(&self.data_root, &self.bundled);
        owen_launcher::core::lawin::login_web_server::start(&self.player_name);
        let name = self.player_name.clone();
        owen_launcher::core::lawin::login_web_server::set_playername(&name);
        let data = self.data_root.clone();
        let bundled = self.bundled.clone();
        std::thread::Builder::new()
            .name("owen-backend-boot".into())
            .spawn(move || {
                let st = owen_launcher::core::lawin::backend_manager::ensure_running(&data, &bundled, None);
                owen_launcher::core::diag::i("app", &format!("backend boot: {st:?}"));
            })
            .ok();
    }

    pub fn save_prefs(&self) {
        let v = serde_json::json!({
            "lang": self.data.prefs.lang,
            "dark": self.data.prefs.dark,
        });
        let _ = std::fs::write(prefs_path(&self.data_root), v.to_string());
    }

    pub fn apply_language(&mut self, lang: &str) {
        self.i18n = I18n::new(lang);
        self.data.prefs.lang = Some(lang.to_string());
        self.save_prefs();
    }

    pub fn t<'a>(&'a self, key: &'a str) -> &'a str {
        self.i18n.t(key)
    }

    /// Drain the shared async slots (set by worker threads).
    pub fn auth_take(&mut self) -> Option<(Result<AccountSession, String>, ())> {
        ui::OWEN_AUTH.lock().ok().and_then(|mut g| g.take())
    }
    pub fn me_take(&mut self) -> Option<(Result<serde_json::Value, String>, ())> {
        ui::OWEN_ME.lock().ok().and_then(|mut g| g.take())
    }
    pub fn hosting_take(&mut self) -> Option<Result<HostingStatus, String>> {
        ui::OWEN_HOST.lock().ok().and_then(|mut g| g.take())
    }
    pub fn test_take(&mut self) -> Option<Result<Vec<(String, bool, String)>, String>> {
        ui::OWEN_TEST.lock().ok().and_then(|mut g| g.take())
    }

    /// Poll everything async (called each frame).
    pub fn poll(&mut self, ctx: &egui::Context) {
        if let Ok(st) = self.install_state.lock() {
            self.install_busy = !matches!(
                *st,
                owen_launcher::uptodown::installer::InstallState::Idle
                    | owen_launcher::uptodown::installer::InstallState::Completed { .. }
                    | owen_launcher::uptodown::installer::InstallState::Error { .. }
            );
        }
        let busy_ui = self.install_busy
            || self.auth_busy
            || self.data.hosting.pending
            || self.data.redirecter.testing
            || self.data.seasons.is_none();
        if busy_ui || matches!(self.phase, Phase::Details(_)) {
            ctx.request_repaint_after(std::time::Duration::from_millis(400));
        }
    }
}

fn prefs_path(data_root: &std::path::Path) -> PathBuf {
    data_root.join("owen_prefs.json")
}

fn load_prefs(data_root: &std::path::Path) -> Prefs {
    let raw = std::fs::read_to_string(prefs_path(data_root))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());
    match raw {
        Some(v) => Prefs {
            lang: v.get("lang").and_then(|x| x.as_str()).map(|s| s.to_string()),
            dark: v.get("dark").and_then(|x| x.as_bool()).unwrap_or(true),
        },
        None => Prefs { lang: None, dark: true },
    }
}

/// (can_install_packages, battery_exempt) — always granted off-Android.
pub fn jni_status() -> (bool, bool) {
    #[cfg(target_os = "android")]
    {
        crate::jni_android::perm_status()
    }
    #[cfg(not(target_os = "android"))]
    {
        (true, true)
    }
}

fn widgets_gold(v: &mut egui::Visuals) {
    use egui::Color32 as C;
    let gold = C::from_rgb(0xff, 0xb3, 0x00);
    let gold_deep = C::from_rgb(0x8a, 0x6a, 0x1e);
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, C::from_rgb(0x9a, 0x9a, 0xa3));
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, gold_deep);
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.5, gold);
    v.widgets.active.fg_stroke = egui::Stroke::new(2.0, gold);
    v.widgets.open.fg_stroke = egui::Stroke::new(1.5, gold);
    v.widgets.noninteractive.bg_fill = C::from_black_alpha(6);
    v.widgets.inactive.bg_fill = C::from_rgb(0x23, 0x1e, 0x14);
    v.widgets.hovered.bg_fill = C::from_rgb(0x2e, 0x27, 0x18);
    v.widgets.active.bg_fill = C::from_rgb(0x3a, 0x30, 0x1c);
    v.selection.bg_fill = gold_deep;
    v.hyperlink_color = gold;
}

/* --------------------------------- eframe --------------------------------- */

impl eframe::App for OwenApp {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if !self.boot_done.load(Ordering::SeqCst) && !self.host.is_wasm() {
            self.boot_done.store(true, Ordering::SeqCst);
            self.boot_native();
        }
        self.poll(root.ctx());
        ui::draw(self, root);
    }
}

/* ---------------------------------- run ----------------------------------- */

fn base_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([430.0, 820.0])
            .with_min_inner_size([340.0, 560.0]),
        ..Default::default()
    }
}

pub fn run() -> eframe::Result<()> {
    eframe::run_native(
        "Owen Launcher",
        base_options(),
        Box::new(|cc| Ok(Box::new(OwenApp::new(cc)))),
    )
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn start(canvas_id: String) -> Result<(), eframe::wasm_bindgen::JsValue> {
    eframe::WebRunner::new()
        .start(canvas_id, eframe::WebOptions::default(), Box::new(|cc| Ok(Box::new(OwenApp::new(cc)))))
}
