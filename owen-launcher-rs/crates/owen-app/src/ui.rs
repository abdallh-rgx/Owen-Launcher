//! The whole Owen Launcher UI — one file, egui immediate mode.
//!
//! Screens: language picker → first-time setup → auth → shell
//! (home / seasons / downloads / partyhub / profile / settings) and the
//! per-season build browser with the full install pipeline.

use crate::platform;
use crate::OwenApp;
use egui::{Button, Color32, Layout, RichText, Sense, Vec2};
use owen_launcher::backend;
use owen_launcher::core::catalogs::{season_catalog, version_catalog};
use owen_launcher::i18n::LANGS;
use owen_launcher::uptodown::installer::{InstallState, OgVersion as InstallerVersion};
use std::path::Path;

const GOLD: Color32 = Color32::from_rgb(0xff, 0xb3, 0x00);
const CHARCOAL: Color32 = Color32::from_rgb(0x14, 0x10, 0x09);
const CARD: Color32 = Color32::from_rgb(0x1e, 0x1a, 0x12);
const MUTED: Color32 = Color32::from_rgb(0x9a, 0x9a, 0xa3);
const GREEN: Color32 = Color32::from_rgb(0x54, 0xd1, 0x8a);
const RED: Color32 = Color32::from_rgb(0xf2, 0x5c, 0x5c);

/* ------------------------- shared async result slots ------------------------- */

pub(crate) static OWEN_AUTH: std::sync::Mutex<Option<(Result<backend::AccountSession, String>, ())>> =
    std::sync::Mutex::new(None);
pub(crate) static OWEN_ME: std::sync::Mutex<Option<(Result<serde_json::Value, String>, ())>> =
    std::sync::Mutex::new(None);
pub(crate) static OWEN_HOST: std::sync::Mutex<Option<Result<backend::HostingStatus, String>>> =
    std::sync::Mutex::new(None);
pub(crate) static OWEN_TEST: std::sync::Mutex<Option<Result<Vec<(String, bool, String)>, String>>> =
    std::sync::Mutex::new(None);
pub(crate) static OWEN_REDIR: std::sync::Mutex<
    Option<Result<(String, Option<String>, Vec<String>), String>>,
> = std::sync::Mutex::new(None);

#[derive(Default)]
pub struct HostingPanel {
    pub status: Option<backend::HostingStatus>,
    pub pending: bool,
    pub error: Option<String>,
}

pub struct RedirecterState {
    pub mode: String,
    pub target: Option<String>,
    pub versions: Vec<String>,
    pub testing: bool,
    pub result: Option<Vec<(String, bool, String)>>,
    pub error: Option<String>,
}

impl Default for RedirecterState {
    fn default() -> Self {
        Self {
            mode: "auto".into(),
            target: None,
            versions: Vec::new(),
            testing: false,
            result: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Home,
    Seasons,
    Downloads,
    PartyHub,
    Profile,
    Settings,
}

impl Default for View {
    fn default() -> Self {
        View::Home
    }
}

pub fn draw(app: &mut OwenApp, root: &mut egui::Ui) {
    let ctx = root.ctx().clone();
    // drain shared slots every frame
    if let Some((result, _)) = app.auth_take() {
        app.auth_busy = false;
        match result {
            Ok(session) => {
                backend::save_account(&app.data_root, &session);
                app.data.account = Some(session);
                app.phase = crate::Phase::Shell;
                app.auth_error = None;
                app.auth_user.clear();
                app.auth_pass.clear();
            }
            Err(e) => app.auth_error = Some(e),
        }
    }
    if let Some((result, _)) = app.me_take() {
        match result {
            Ok(v) => app.data.me = Some(v),
            Err(e) => app.status = e,
        }
    }
    if let Some(result) = app.hosting_take() {
        app.data.hosting.pending = false;
        match result {
            Ok(st) => {
                app.data.hosting.status = Some(st);
                app.data.hosting.error = None;
            }
            Err(e) => app.data.hosting.error = Some(e),
        }
    }
    if let Some(result) = app.test_take() {
        app.data.redirecter.testing = false;
        match result {
            Ok(checks) => app.data.redirecter.result = Some(checks),
            Err(e) => app.data.redirecter.error = Some(e),
        }
    }
    if let Some(result) = OWEN_REDIR.lock().ok().and_then(|mut g| g.take()) {
        match result {
            Ok((mode, target, versions)) => {
                app.data.redirecter.mode = mode;
                app.data.redirecter.target = target;
                app.data.redirecter.versions = versions;
            }
            Err(e) => app.data.redirecter.error = Some(e),
        }
    }

    match app.phase.clone() {
        crate::Phase::Boot | crate::Phase::Language => language_screen(app, root, &ctx),
        crate::Phase::Setup => setup_screen(app, root, &ctx),
        crate::Phase::Auth => auth_screen(app, root, &ctx),
        crate::Phase::Shell => shell(app, root, &ctx),
        crate::Phase::Details(_) => details_screen(app, root, &ctx),
    }
}

/* ------------------------------- primitives ------------------------------- */

fn gold_button(ui: &mut egui::Ui, label: &str, enabled: bool) -> bool {
    ui.add(
        Button::new(RichText::new(label).strong().color(if enabled { CHARCOAL } else { MUTED }))
            .fill(if enabled { GOLD } else { Color32::from_gray(0x2a) })
            .corner_radius(10),
    )
    .clicked()
}

fn pill(ui: &mut egui::Ui, text: &str, ok: bool) {
    let bg = if ok { GREEN } else { GOLD };
    egui::Frame::default()
        .fill(bg.gamma_multiply(0.2))
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.0).color(bg).strong());
        });
}

fn card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::default()
        .fill(CARD)
        .corner_radius(14)
        .inner_margin(egui::Margin::same(12))
        .stroke(egui::Stroke::new(1.0, Color32::from_gray(0x2a)))
        .show(ui, add);
}

fn brand_header(app: &OwenApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.add_space(6.0);
        ui.label(RichText::new(app.t("brand")).size(24.0).strong().color(Color32::WHITE));
        ui.label(
            RichText::new(app.t("launcher"))
                .size(24.0)
                .strong()
                .color(GOLD),
        );
    });
    ui.label(RichText::new(app.t("tagline")).size(12.0).color(MUTED));
}

/* ------------------------------ language pick ------------------------------ */

fn language_screen(app: &mut OwenApp, root: &mut egui::Ui, _ctx: &egui::Context) {
    egui::CentralPanel::default().show(root, |ui| {
        ui.add_space(18.0);
        brand_header(app, ui);
        ui.add_space(4.0);
        ui.label(
            RichText::new(app.t("chooseLanguage"))
                .size(20.0)
                .strong()
                .color(Color32::WHITE),
        );
        ui.label(RichText::new(app.t("chooseLanguageSub")).size(12.0).color(MUTED));
        ui.add_space(12.0);
        egui::ScrollArea::vertical().show(ui, |ui| {
            let per_row = if ui.available_width() > 560.0 { 2 } else { 1 };
            for chunk in LANGS.chunks(per_row) {
                ui.horizontal(|ui| {
                    for meta in chunk.iter() {
                        let w = ui.available_width() / per_row as f32 - 6.0;
                        let resp = egui::Frame::default()
                            .fill(CARD)
                            .corner_radius(12)
                            .inner_margin(egui::Margin::same(10))
                            .stroke(egui::Stroke::new(1.0, Color32::from_gray(0x33)))
                            .show(ui, |ui| {
                                ui.set_min_size(Vec2::new(w, 48.0));
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(meta.native)
                                            .size(16.0)
                                            .strong()
                                            .color(Color32::WHITE),
                                    );
                                    ui.label(RichText::new(meta.english).size(11.0).color(MUTED));
                                });
                            })
                            .response
                            .interact(Sense::click());
                        if resp.clicked() {
                            app.apply_language(meta.code);
                            app.phase = if app.host.needs_setup(&app.data_root) {
                                crate::Phase::Setup
                            } else if app.data.account.is_none() {
                                crate::Phase::Auth
                            } else {
                                crate::Phase::Shell
                            };
                        }
                        ui.add_space(4.0);
                    }
                });
                ui.add_space(4.0);
            }
        });
    });
}

/* --------------------------------- setup ---------------------------------- */

fn setup_screen(app: &mut OwenApp, root: &mut egui::Ui, _ctx: &egui::Context) {
    egui::CentralPanel::default().show(root, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(14.0);
            brand_header(app, ui);
            ui.label(RichText::new(app.t("firstTimeSetup")).size(20.0).strong().color(GOLD));
            ui.label(RichText::new(app.t("setupSub")).size(12.0).color(MUTED));
            ui.add_space(10.0);

            if app.host.is_android() {
                let (perm, batt) = crate::jni_status();
                card(ui, |ui| {
                    ui.label(RichText::new(app.t("installPermission")).strong().color(Color32::WHITE));
                    ui.label(RichText::new(app.t("installPermDesc")).size(11.0).color(MUTED));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        pill(ui, if perm { app.t("statusOk") } else { app.t("statusRequired") }, perm);
                        if !perm && gold_button(ui, app.t("grant"), true) {
                            platform::request_install_permission();
                        }
                    });
                });
                ui.add_space(8.0);
                card(ui, |ui| {
                    ui.label(RichText::new(app.t("batteryOpt")).strong().color(Color32::WHITE));
                    ui.label(RichText::new(app.t("batteryDesc")).size(11.0).color(MUTED));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        pill(ui, if batt { app.t("statusOk") } else { app.t("statusRequired") }, batt);
                        if !batt && gold_button(ui, app.t("allow"), true) {
                            platform::request_battery_exemption();
                        }
                    });
                });
                ui.add_space(8.0);
            }

            if app.player_name.trim().is_empty() {
                card(ui, |ui| {
                    ui.label(RichText::new(app.t("displayName")).strong().color(Color32::WHITE));
                    ui.label(RichText::new(app.t("playerNameNote")).size(11.0).color(MUTED));
                    ui.add_space(6.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut app.player_name)
                            .char_limit(32)
                            .desired_width(ui.available_width() - 90.0),
                    );
                    ui.add_space(6.0);
                    let enabled = !app.player_name.trim().is_empty();
                    if gold_button(ui, app.t("saveName"), enabled) {
                        let name = app.player_name.trim().to_string();
                        owen_launcher::core::player_store::set(&app.data_root, &name);
                        owen_launcher::core::lawin::login_web_server::set_playername(&name);
                        app.status = app.t("savedWord").into();
                    }
                });
            } else if gold_button(ui, app.t("continueWord"), true) {
                app.phase = if app.data.account.is_none() {
                    crate::Phase::Auth
                } else {
                    crate::Phase::Shell
                };
            }
            ui.label(RichText::new(app.t("setupNote")).size(10.0).color(MUTED));
        });
    });
}

/* ---------------------------------- auth ---------------------------------- */

fn auth_screen(app: &mut OwenApp, root: &mut egui::Ui, ctx: &egui::Context) {
    egui::CentralPanel::default().show(root, |ui| {
        ui.add_space(16.0);
        brand_header(app, ui);
        ui.label(RichText::new(app.t("authWelcome")).size(20.0).strong().color(Color32::WHITE));
        ui.label(RichText::new(app.t("authSub")).size(12.0).color(MUTED));
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            if ui.selectable_label(app.auth_tab == 0, gold(app.t("signIn"))).clicked() {
                app.auth_tab = 0;
            }
            if ui.selectable_label(app.auth_tab == 1, gold(app.t("createAccount"))).clicked() {
                app.auth_tab = 1;
            }
        });
        ui.add_space(8.0);

        card(ui, |ui| {
            let w = ui.available_width() - 20.0;
            if app.auth_tab == 0 {
                ui.label(app.t("usernameOrEmail"));
                ui.add(egui::TextEdit::singleline(&mut app.auth_user).desired_width(w));
                ui.label(app.t("password"));
                ui.add(egui::TextEdit::singleline(&mut app.auth_pass).password(true).desired_width(w));
                ui.label(RichText::new(app.t("passwordHint")).size(10.0).color(MUTED));
            } else {
                ui.label(app.t("email"));
                ui.add(egui::TextEdit::singleline(&mut app.auth_email).desired_width(w));
                ui.label(app.t("username"));
                ui.add(egui::TextEdit::singleline(&mut app.auth_user_name).desired_width(w));
                ui.label(app.t("displayName"));
                ui.add(egui::TextEdit::singleline(&mut app.auth_display).desired_width(w));
                ui.label(app.t("password"));
                ui.add(egui::TextEdit::singleline(&mut app.auth_pass).password(true).desired_width(w));
                ui.label(app.t("confirmPassword"));
                ui.add(egui::TextEdit::singleline(&mut app.auth_confirm).password(true).desired_width(w));
                ui.label(RichText::new(app.t("passwordHint")).size(10.0).color(MUTED));
            }
        });

        if let Some(err) = &app.auth_error {
            ui.add_space(6.0);
            ui.label(RichText::new(err.as_str()).size(12.0).color(RED));
        }
        ui.add_space(8.0);

        let label = if app.auth_tab == 0 { app.t("signIn") } else { app.t("register") };
        if gold_button(ui, label, !app.auth_busy) {
            let mut bad: Option<&str> = None;
            if app.auth_tab == 0 {
                if app.auth_user.trim().is_empty() {
                    bad = Some("vEnterUser");
                } else if app.auth_pass.is_empty() {
                    bad = Some("vEnterPass");
                }
            } else {
                if !app.auth_email.contains('@') || !app.auth_email.contains('.') {
                    bad = Some("vEmail");
                } else if !(3..=16).contains(&app.auth_user_name.trim().len())
                    || !app
                        .auth_user_name
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
                {
                    bad = Some("vUser");
                } else if !(3..=32).contains(&app.auth_display.trim().len()) {
                    bad = Some("vName");
                } else if app.auth_pass.len() < 8 {
                    bad = Some("vWeak");
                } else if app.auth_pass != app.auth_confirm {
                    bad = Some("vMatch");
                }
            }
            if let Some(k) = bad {
                app.auth_error = Some(app.t(k).into());
            } else {
                app.auth_busy = true;
                app.auth_error = None;
                if app.auth_tab == 0 {
                    let (ue, pw) = (
                        app.auth_user.trim().to_string(),
                        app.auth_pass.clone(),
                    );
                    backend::login(ctx, &ue, &pw, move |result| {
                        if let Ok(mut g) = OWEN_AUTH.lock() {
                            *g = Some((result, ()));
                        }
                    });
                } else {
                    let (em, un, dn, pw) = (
                        app.auth_email.trim().to_string(),
                        app.auth_user_name.trim().to_string(),
                        app.auth_display.trim().to_string(),
                        app.auth_pass.clone(),
                    );
                    backend::register(ctx, &em, &un, &dn, &pw, move |result| {
                        if let Ok(mut g) = OWEN_AUTH.lock() {
                            *g = Some((result, ()));
                        }
                    });
                }
            }
        }
        ui.add_space(4.0);
        ui.label(RichText::new(app.t("realAccountNote")).size(10.0).color(MUTED));
    });
}

fn gold(text: impl Into<String>) -> RichText {
    RichText::new(text).color(GOLD)
}

/* ---------------------------------- shell ---------------------------------- */

fn shell(app: &mut OwenApp, root: &mut egui::Ui, ctx: &egui::Context) {
    // lazy polls
    if app.data.hosting.status.is_none() && !app.data.hosting.pending {
        app.data.hosting.pending = true;
        backend::hosting_status(ctx, |r| {
            if let Ok(mut g) = OWEN_HOST.lock() {
                *g = Some(r);
            }
        });
    }
    if app.data.stats.is_none() {
        backend::get_json(ctx, "/", move |r: Result<serde_json::Value, String>| {
            if let Ok(mut g) = OWEN_ME.lock() {
                *g = Some((r, ()));
            }
        });
    }
    if app.data.seasons.is_none() {
        backend::get_json(ctx, "/api/launcher/seasons", move |r: Result<serde_json::Value, String>| {
            if let Ok(mut g) = OWEN_ME.lock() {
                *g = Some((r, ()));
            }
        });
    }

    egui::Panel::top("header").show(root, |ui| {
        ui.horizontal(|ui| {
            brand_header(app, ui);
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("⚙").clicked() {
                    app.view = View::Settings;
                }
                if let Some(acc) = &app.data.account {
                    ui.label(RichText::new(acc.display_name.as_str()).size(12.0).color(GOLD).strong());
                }
            });
        });
    });

    egui::Panel::bottom("nav").show(root, |ui| {
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            let tabs = [
                (View::Home, app.t("home").to_string()),
                (View::Seasons, app.t("seasons").to_string()),
                (View::Downloads, app.t("downloads").to_string()),
                (View::PartyHub, app.t("partyhub").to_string()),
                (View::Profile, app.t("profile").to_string()),
            ];
            for (v, label) in tabs.iter() {
                let active = std::mem::discriminant(v) == std::mem::discriminant(&app.view);
                if ui
                    .add(
                        Button::new(
                            RichText::new(label.as_str())
                                .size(12.0)
                                .strong()
                                .color(if active { CHARCOAL } else { MUTED }),
                        )
                        .fill(if active { GOLD } else { Color32::TRANSPARENT })
                        .corner_radius(10),
                    )
                    .clicked()
                {
                    app.view = v.clone();
                }
            }
        });
    });

    if !app.status.is_empty() {
        egui::Panel::bottom("status").show(root, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("●").color(GOLD));
                ui.label(RichText::new(app.status.as_str()).size(11.0).color(MUTED));
            });
        });
    }

    egui::CentralPanel::default().show(root, |ui| match app.view.clone() {
        View::Home => home_view(app, ui, ctx),
        View::Seasons => seasons_view(app, ui, ctx),
        View::Downloads => downloads_view(app, ui),
        View::PartyHub => partyhub_view(app, ui),
        View::Profile => profile_view(app, ui, ctx),
        View::Settings => settings_view(app, ui, ctx),
    });
}

/* ---------------------------------- views ---------------------------------- */

fn home_view(app: &mut OwenApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.label(
            RichText::new(format!(
                "{} {}",
                app.t("welcome"),
                app.data
                    .account
                    .as_ref()
                    .map(|a| a.display_name.as_str())
                    .unwrap_or("")
            ))
            .size(18.0)
            .strong()
            .color(Color32::WHITE),
        );
        let running = app.data.hosting.status.as_ref().map(|s| s.running).unwrap_or(false);
        ui.horizontal(|ui| {
            pill(ui, if running { app.t("serverOnline") } else { app.t("serverOffline") }, running);
            if let Some(st) = &app.data.hosting.status {
                ui.label(RichText::new(st.detail.as_str()).size(10.0).color(MUTED));
            }
        });
        ui.add_space(8.0);

        let current = app
            .data
            .seasons
            .as_ref()
            .and_then(|v| v.get("current").and_then(|x| x.as_u64()).map(|n| n as u32))
            .unwrap_or(42);
        card(ui, |ui| {
            ui.label(RichText::new(app.t("currentSeason")).size(11.0).color(MUTED));
            ui.label(
                RichText::new(format!("{} {}", app.t("seasonWord"), current))
                    .size(24.0)
                    .strong()
                    .color(GOLD),
            );
            if let Some(tex) = season_texture(app, ctx, current) {
                ui.add(
                    egui::Image::from_texture(&tex)
                        .fit_to_fraction(Vec2::new(1.0, 0.42))
                        .corner_radius(10),
                );
            }
            if gold_button(ui, app.t("playNow"), true) {
                app.view = View::Seasons;
            }
        });
        ui.add_space(8.0);

        card(ui, |ui| {
            ui.label(RichText::new(app.t("quickStats")).strong().color(GOLD));
            if let Some(stats) = &app.data.stats {
                let rows: Vec<(&str, String)> = vec![
                    (
                        app.t("backend"),
                        stats
                            .get("server")
                            .and_then(|x| x.as_str())
                            .unwrap_or("owen-server")
                            .to_string(),
                    ),
                    (
                        app.t("version"),
                        stats
                            .get("version")
                            .and_then(|x| x.as_str())
                            .unwrap_or("—")
                            .to_string(),
                    ),
                    (
                        app.t("gameVersions"),
                        stats
                            .get("manifests")
                            .and_then(|x| x.as_u64())
                            .map(|n| n.to_string())
                            .unwrap_or_else(|| "—".into()),
                    ),
                ];
                for (k, v) in rows.iter() {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(k.to_string()).size(12.0).color(MUTED));
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(v.as_str()).size(12.0).strong().color(Color32::WHITE));
                        });
                    });
                }
            } else {
                ui.label(RichText::new(app.t("loading")).size(12.0).color(MUTED));
            }
        });
    });
}

fn seasons_view(app: &mut OwenApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.label(RichText::new(app.t("allSeasons")).size(16.0).strong().color(GOLD));
        ui.label(RichText::new(app.t("seasonsTagline")).size(11.0).color(MUTED));
        ui.label(RichText::new(format!("{} abdallhrgx", app.t("by"))).size(10.0).color(MUTED));
        ui.add_space(6.0);
        let seasons = version_catalog::seasons();
        let width = ui.available_width();
        let per_row = if width > 720.0 { 3 } else if width > 460.0 { 2 } else { 1 };
        let rows: Vec<u32> = seasons.iter().rev().copied().collect();
        for chunk in rows.chunks(per_row) {
            ui.horizontal(|ui| {
                for n in chunk.iter().copied() {
                    let avail = ui.available_width() / per_row as f32;
                    let resp = egui::Frame::default()
                        .fill(CARD)
                        .corner_radius(12)
                        .inner_margin(egui::Margin::same(8))
                        .show(ui, |ui| {
                            ui.set_min_size(Vec2::new(avail - 8.0, 120.0));
                            if let Some(tex) = season_texture(app, ctx, n) {
                                ui.add(
                                    egui::Image::from_texture(&tex)
                                        .fit_to_fraction(Vec2::new(1.0, 0.62))
                                        .corner_radius(8),
                                );
                            } else {
                                ui.add_space(52.0);
                            }
                            let info = season_catalog::get(n);
                            ui.label(
                                RichText::new(
                                    info.map(|i| i.title.to_string())
                                        .unwrap_or_else(|| format!("{} {}", app.t("seasonWord"), n)),
                                )
                                .size(13.0)
                                .strong()
                                .color(Color32::WHITE),
                            );
                            if let Some(i) = info {
                                ui.label(RichText::new(i.chapter).size(10.0).color(GOLD));
                            }
                        })
                        .response
                        .interact(Sense::click());
                    if resp.clicked() {
                        app.details_season = n;
                        app.phase = crate::Phase::Details(n);
                    }
                    ui.add_space(4.0);
                }
            });
            ui.add_space(6.0);
        }
    });
}

fn details_screen(app: &mut OwenApp, root: &mut egui::Ui, ctx: &egui::Context) {
    let season = app.details_season;
    egui::Panel::top("details_header").show(root, |ui| {
        ui.horizontal(|ui| {
            if ui.button("←").clicked() {
                app.phase = crate::Phase::Shell;
                app.view = View::Seasons;
            }
            ui.label(
                RichText::new(format!("{} {}", app.t("seasonWord"), season))
                    .size(18.0)
                    .strong()
                    .color(GOLD),
            );
        });
    });
    if !app.status.is_empty() {
        egui::Panel::bottom("details_status").show(root, |ui| {
            ui.label(RichText::new(app.status.as_str()).size(11.0).color(MUTED));
        });
    }
    egui::CentralPanel::default().show(root, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Some(info) = season_catalog::get(season) {
                card(ui, |ui| {
                    ui.label(RichText::new(info.title).size(16.0).strong().color(Color32::WHITE));
                    ui.label(RichText::new(format!("{} • {}", info.chapter, info.dates)).size(10.0).color(GOLD));
                    ui.label(RichText::new(info.slogan).size(11.0).color(MUTED));
                    ui.label(RichText::new(info.description).size(11.0).color(MUTED));
                });
                ui.add_space(8.0);
            }

            let st = app.install_state.lock().ok().map(|g| g.clone());
            if let Some(st) = &st {
                match st {
                    InstallState::Idle => {}
                    InstallState::Resolving { status } => busy_row(ui, status),
                    InstallState::Downloading { progress, bytes_done, bytes_total } => {
                        card(ui, |ui| {
                            ui.label(RichText::new(app.t("downloading")).strong().color(GOLD));
                            ui.add(
                                egui::ProgressBar::new(*progress).text(format!(
                                    "{:.0}% • {:.1}/{:.1} MB",
                                    progress * 100.0,
                                    *bytes_done as f64 / 1048576.0,
                                    *bytes_total as f64 / 1048576.0
                                )),
                            );
                        });
                    }
                    InstallState::Patching { status } | InstallState::Installing { status } => {
                        busy_row(ui, status)
                    }
                    InstallState::Completed { package_name } => {
                        card(ui, |ui| {
                            pill(ui, package_name, true);
                        });
                    }
                    InstallState::Error { message } => {
                        card(ui, |ui| {
                            ui.label(RichText::new(message.as_str()).size(12.0).color(RED));
                        });
                    }
                }
                ui.add_space(8.0);
            }

            ui.label(
                RichText::new(format!(
                    "{} ({})",
                    app.t("allVersions"),
                    version_catalog::by_season(season).len()
                ))
                .size(14.0)
                .strong()
                .color(GOLD),
            );
            ui.add_space(4.0);

            for v in version_catalog::by_season(season) {
                let pkg = owen_patcher::owen_manager::package_name_for(&to_patch_version(v));
                let installed = platform::installed_package(&pkg).is_some();
                card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("V{}-CL-{}", v.version, v.cl))
                                .size(14.0)
                                .strong()
                                .color(Color32::WHITE),
                        );
                        if v.is_dev {
                            pill(ui, app.t("devBuild"), false);
                        }
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            pill(
                                ui,
                                if installed { app.t("installedLaunch") } else { app.t("pressDownload") },
                                installed,
                            );
                        });
                    });
                    ui.label(
                        RichText::new(format!("{} {}", app.t("manifestHash"), v.manifest_hash.unwrap_or("—")))
                            .size(10.0)
                            .color(MUTED),
                    );
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if gold_button(
                            ui,
                            if installed { app.t("launchWord") } else { app.t("downloadInstall") },
                            !app.install_busy,
                        ) {
                            if installed {
                                launch_flow(app, ctx, v, &pkg);
                            } else {
                                install_flow(app, ctx, v);
                            }
                        }
                        if installed
                            && ui.button(RichText::new(app.t("uninstallWord")).color(RED)).clicked()
                        {
                            platform::uninstall_package(&pkg);
                        }
                    });
                });
                ui.add_space(6.0);
            }

            ui.add_space(6.0);
            if ui.button(app.t("technicalDetails")).clicked() {}
            ui.label(
                RichText::new(owen_launcher::core::diag::tail_text(12))
                    .size(9.0)
                    .color(MUTED)
                    .monospace(),
            );
        });
    });
}

fn busy_row(ui: &mut egui::Ui, status: &str) {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.add(egui::Spinner::new());
            ui.label(RichText::new(status).size(12.0).color(GOLD));
        });
    });
}

fn downloads_view(app: &mut OwenApp, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.label(RichText::new(app.t("gameVersions")).size(16.0).strong().color(GOLD));
        ui.label(RichText::new(app.t("gameVersionsSub")).size(11.0).color(MUTED));
        ui.add_space(6.0);
        egui::Grid::new("versions").striped(true).show(ui, |ui| {
            ui.strong(app.t("buildVersion"));
            ui.strong(app.t("manifestHash"));
            ui.strong(app.t("seasonWord"));
            ui.end_row();
            for v in version_catalog::ALL.iter().rev().take(80) {
                ui.label(format!(
                    "V{}-CL-{}{}",
                    v.version,
                    v.cl,
                    if v.is_dev { " (DEV)" } else { "" }
                ));
                ui.label(RichText::new(v.manifest_hash.unwrap_or("—")).size(10.0).color(MUTED));
                ui.label(v.season.to_string());
                ui.end_row();
            }
        });
    });
}

fn partyhub_view(app: &mut OwenApp, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.label(RichText::new(app.t("partyhubTitle")).size(16.0).strong().color(GOLD));
        ui.label(RichText::new(app.t("partyhubSub")).size(11.0).color(MUTED));
        ui.add_space(8.0);
        let url = if app.host.is_wasm() {
            "/partyhub/offline/en-US/".to_string()
        } else {
            format!("{}/partyhub/offline/en-US/", platform::base_url().trim_end_matches('/'))
        };
        card(ui, |ui| {
            ui.label(RichText::new(url.as_str()).size(11.0).color(MUTED));
            if gold_button(ui, app.t("openFull"), true) {
                platform::open_url(&url);
            }
        });
        ui.add_space(6.0);
        ui.label(RichText::new(app.t("trailerPlaying")).size(10.0).color(MUTED));
    });
}

fn profile_view(app: &mut OwenApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.label(RichText::new(app.t("myProfile")).size(16.0).strong().color(GOLD));
        if let Some(acc) = &app.data.account {
            card(ui, |ui| {
                ui.label(RichText::new(acc.display_name.as_str()).size(18.0).strong().color(Color32::WHITE));
                ui.label(RichText::new(acc.email.as_str()).size(11.0).color(MUTED));
            });
        }
        if app.data.me.is_none() {
            backend::get_json(
                ctx,
                "/api/launcher/me",
                move |r: Result<serde_json::Value, String>| {
                    if let Ok(mut g) = OWEN_ME.lock() {
                        *g = Some((r, ()));
                    }
                },
            );
        }
        if let Some(me) = app.data.me.clone() {
            card(ui, |ui| {
                for (k, v) in [
                    (
                        app.t("level"),
                        me.pointer("/profile/level")
                            .and_then(|x| x.as_u64())
                            .map(|n| n.to_string()),
                    ),
                    (
                        app.t("vbucks"),
                        me.pointer("/profile/vbucks")
                            .and_then(|x| x.as_u64())
                            .map(|n| n.to_string()),
                    ),
                    (
                        app.t("items"),
                        me.pointer("/profile/items")
                            .and_then(|x| x.as_u64())
                            .map(|n| n.to_string()),
                    ),
                ] {
                    if let Some(v) = v {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(k.to_string()).size(12.0).color(MUTED));
                            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(RichText::new(v).size(12.0).strong().color(GOLD));
                            });
                        });
                    }
                }
            });
        }
        ui.add_space(8.0);
        if ui.button(RichText::new(app.t("logout")).color(RED)).clicked() {
            backend::clear_account(&app.data_root);
            app.data.account = None;
            app.data.me = None;
            app.phase = crate::Phase::Auth;
        }
    });
}

fn settings_view(app: &mut OwenApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.label(RichText::new(app.t("settings")).size(16.0).strong().color(GOLD));
        ui.add_space(8.0);

        // language
        card(ui, |ui| {
            ui.label(RichText::new(app.t("language")).strong().color(Color32::WHITE));
            let current = app.data.prefs.lang.clone().unwrap_or_else(|| "ar".into());
            let mut picked: Option<&str> = None;
            ui.horizontal_wrapped(|ui| {
                for meta in LANGS.iter() {
                    let active = meta.code == current;
                    if ui
                        .add(
                            Button::new(
                                RichText::new(meta.native)
                                    .size(11.0)
                                    .color(if active { CHARCOAL } else { MUTED }),
                            )
                            .fill(if active { GOLD } else { Color32::TRANSPARENT })
                            .corner_radius(8),
                        )
                        .clicked()
                    {
                        picked = Some(meta.code);
                    }
                }
            });
            if let Some(code) = picked {
                app.apply_language(code);
            }
        });
        ui.add_space(6.0);

        // hosting panel (on-demand)
        card(ui, |ui| {
            ui.label(RichText::new(app.t("hostingTitle")).strong().color(Color32::WHITE));
            ui.label(RichText::new(app.t("hostingSub")).size(11.0).color(MUTED));
            ui.add_space(6.0);
            let running = app.data.hosting.status.as_ref().map(|s| s.running).unwrap_or(false);
            pill(ui, if running { app.t("serverRunning") } else { app.t("serverStopped") }, running);
            if let Some(err) = &app.data.hosting.error {
                ui.label(RichText::new(err.as_str()).size(11.0).color(RED));
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if gold_button(
                    ui,
                    if app.data.hosting.pending { app.t("loading") } else { app.t("startServer") },
                    !app.data.hosting.pending && !running,
                ) {
                    app.data.hosting.pending = true;
                    app.data.hosting.error = None;
                    let root = app.data_root.clone();
                    backend::hosting_start(ctx, &root, move |r| {
                        if let Ok(mut g) = OWEN_HOST.lock() {
                            *g = Some(r);
                        }
                    });
                }
                if gold_button(ui, app.t("stopServer"), !app.data.hosting.pending && running) {
                    app.data.hosting.pending = true;
                    let root = app.data_root.clone();
                    backend::hosting_stop(ctx, &root, move |r| {
                        if let Ok(mut g) = OWEN_HOST.lock() {
                            *g = Some(r.map(|_| backend::HostingStatus {
                                running: false,
                                detail: "stopped".into(),
                            }));
                        }
                    });
                }
            });
        });
        ui.add_space(6.0);

        // redirecter menu — fetch the config once
        if app.data.redirecter.versions.is_empty() && !app.data.redirecter.testing {
            backend::redirecter_get(
                ctx,
                move |r: Result<(String, Option<String>, Vec<String>), String>| {
                    if let Ok(mut g) = OWEN_REDIR.lock() {
                        *g = Some(r);
                    }
                },
            );
        }
        card(ui, |ui| {
            ui.label(RichText::new(app.t("redirecter")).strong().color(Color32::WHITE));
            ui.label(RichText::new(app.t("redirecterSub")).size(10.0).color(MUTED));
            ui.add_space(6.0);
            let modes = [
                ("auto".to_string(), app.t("autoMode").to_string(), app.t("autoDesc").to_string()),
                ("fixed".to_string(), app.t("fixedMode").to_string(), app.t("fixedDesc").to_string()),
                ("off".to_string(), app.t("offMode").to_string(), app.t("offDesc").to_string()),
            ];
            ui.label(RichText::new(app.t("redirecterMode")).size(11.0).color(MUTED));
            ui.horizontal(|ui| {
                for (id, label, _) in modes.iter() {
                    let active = &app.data.redirecter.mode == id;
                    if ui
                        .add(
                            Button::new(
                                RichText::new(label.as_str())
                                    .strong()
                                    .color(if active { CHARCOAL } else { MUTED }),
                            )
                            .fill(if active { GOLD } else { Color32::TRANSPARENT })
                            .corner_radius(8),
                        )
                        .clicked()
                    {
                        app.data.redirecter.mode = id.to_string();
                    }
                }
            });
            let desc = modes
                .iter()
                .find(|(id, _, _)| *id == app.data.redirecter.mode)
                .map(|(_, _, d)| d.clone())
                .unwrap_or_default();
            ui.label(RichText::new(desc).size(10.0).color(MUTED));
            if app.data.redirecter.mode == "fixed" {
                ui.label(RichText::new(app.t("targetVersion")).size(11.0).color(MUTED));
                let cur = app.data.redirecter.target.clone().unwrap_or_default();
                egui::ComboBox::from_id_salt("target")
                    .selected_text(cur)
                    .show_ui(ui, |ui| {
                        for v in app.data.redirecter.versions.iter() {
                            ui.selectable_value(
                                &mut app.data.redirecter.target,
                                Some(v.clone()),
                                v.as_str(),
                            );
                        }
                    });
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if gold_button(ui, app.t("save"), true) {
                    let mode = app.data.redirecter.mode.clone();
                    let target = app.data.redirecter.target.clone();
                    backend::redirecter_set(ctx, &mode, target.as_deref(), move |r| {
                        if let Err(e) = r {
                            if let Ok(mut g) = OWEN_HOST.lock() {
                                *g = Some(Err(e));
                            }
                        }
                    });
                }
                if ui.button(app.t("testRedirecter")).clicked() && !app.data.redirecter.testing {
                    app.data.redirecter.testing = true;
                    app.data.redirecter.result = None;
                    backend::selftest(ctx, move |r| {
                        if let Ok(mut g) = OWEN_TEST.lock() {
                            *g = Some(r);
                        }
                    });
                }
            });
            if app.data.redirecter.testing {
                ui.label(RichText::new(app.t("testing")).size(11.0).color(MUTED));
            }
            if let Some(checks) = app.data.redirecter.result.clone() {
                let all_ok = checks.iter().all(|(_, ok, _)| *ok);
                if all_ok {
                    pill(ui, app.t("testsPassed"), true);
                }
                for (id, ok, detail) in checks.iter() {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(if *ok { "✓" } else { "✗" })
                                .color(if *ok { GREEN } else { RED }),
                        );
                        ui.label(RichText::new(id.as_str()).size(11.0).color(Color32::WHITE));
                        ui.label(RichText::new(detail.as_str()).size(9.0).color(MUTED));
                    });
                }
            }
        });
        ui.add_space(6.0);

        // GameServer config
        card(ui, |ui| {
            ui.label(RichText::new(app.t("gameServerConfig")).strong().color(Color32::WHITE));
            let mut settings = owen_launcher::core::lawin::settings::load(&app.data_root);
            ui.label(app.t("ipLabel"));
            ui.add(
                egui::TextEdit::singleline(&mut settings.game_server.ip).desired_width(140.0),
            );
            ui.label(app.t("portLabel"));
            ui.add(egui::DragValue::new(&mut settings.game_server.port).range(1..=65535));
            if gold_button(ui, app.t("save"), true) {
                owen_launcher::core::lawin::settings::save(&app.data_root, &settings);
                app.status = format!("{} — {}", app.t("savedWord"), app.t("backendHint"));
            }
        });
        ui.add_space(6.0);

        // about
        card(ui, |ui| {
            ui.label(RichText::new(app.t("about")).strong().color(GOLD));
            ui.label(RichText::new(app.t("aboutBody")).size(11.0).color(MUTED));
            ui.label(
                RichText::new(format!("owen-app {} · {:?}", crate::APP_VERSION, app.host))
                    .size(10.0)
                    .color(MUTED),
            );
            if app.host.is_wasm() {
                ui.label(RichText::new(app.t("previewNote")).size(10.0).color(MUTED));
            }
            if ui.button(app.t("reload")).clicked() {
                app.data.stats = None;
                app.data.seasons = None;
                app.data.hosting.status = None;
            }
        });
    });
}

fn to_patch_version(v: &version_catalog::OgVersion) -> owen_patcher::owen_manager::OgVersion {
    owen_patcher::owen_manager::OgVersion {
        season: v.season as i32,
        version: v.version.to_string(),
        cl: v.cl,
        is_dev: v.is_dev,
        fixed_package: v.fixed_package.to_string(),
    }
}

fn install_flow(app: &mut OwenApp, ctx: &egui::Context, v: &version_catalog::OgVersion) {
    if app.install_busy {
        return;
    }
    *app.install_state.lock().unwrap() = InstallState::Resolving {
        status: "Starting…".into(),
    };
    app.status = app.t("loading").into();

    let ctx2 = ctx.clone();
    let state_arc = app.install_state.clone();
    let data_root = app.data_root.clone();
    let work = platform::cache_dir().join("patch_work");
    let _ = std::fs::create_dir_all(&work);
    let pv = to_patch_version(v);
    let iv = InstallerVersion {
        version: v.version.to_string(),
        cl: v.cl,
        uptodown_id: v.uptodown_id,
        direct_apk_url: v.direct_apk_url.to_string(),
        fixed_package: v.fixed_package.to_string(),
    };
    // signing key: the original ogfn pair when bundled, else a fresh local key
    let ogfn = app.bundled.join("signing/ogfn-key.pem");
    let key_path = if ogfn.is_file() {
        ogfn
    } else {
        data_root.join("tools").join("owen-key.pem")
    };
    let owen_lib = platform::owen_lib_path();

    std::thread::Builder::new()
        .name("owen-install".into())
        .spawn(move || {
            use owen_launcher::uptodown::installer::UptodownInstaller;
            // hand the patch context to the host installer
            platform::set_install_ctx(platform::InstallCtx {
                version: pv,
                backend_url: owen_launcher::uptodown::installer::DEFAULT_BACKEND_URL.to_string(),
                login_url: owen_launcher::core::lawin::login_web_server::login_url().unwrap_or_default(),
                cache_dir: data_root.join("patch_cache"),
                work_dir: work.clone(),
                owen_lib,
                key_path,
            });
            let mut inst = platform::make_installer();
            let http = UptodownInstaller::default_backend();
            let mut ui_inst = UptodownInstaller::new();
            let state2 = state_arc.clone();
            let ctx3 = ctx2.clone();
            let result = ui_inst.install(
                &http,
                &mut *inst,
                &work,
                &iv,
                |_ev| {},
                move |st: InstallState| {
                    *state2.lock().unwrap() = st;
                    ctx3.request_repaint();
                },
            );
            match result {
                Ok(pkg) => {
                    let mut mgr = owen_patcher::owen_manager::OwenManager::open(&data_root);
                    mgr.mark_patched(&pkg, owen_launcher::uptodown::installer::DEFAULT_BACKEND_URL);
                    *state_arc.lock().unwrap() = InstallState::Completed { package_name: pkg };
                }
                Err(e) => {
                    *state_arc.lock().unwrap() = InstallState::Error { message: e };
                }
            }
            ctx2.request_repaint_after(std::time::Duration::from_millis(100));
        })
        .ok();
}

fn launch_flow(app: &mut OwenApp, ctx: &egui::Context, v: &version_catalog::OgVersion, pkg: &str) {
    app.status = app.t("checkingInjection").into();
    let data = app.data_root.clone();
    let bundled = app.bundled.clone();
    let lv = v.clone();
    let pkg2 = pkg.to_string();
    let player = owen_launcher::core::player_store::authorize_url(&app.data_root).unwrap_or_else(
        || format!("com.epicgames.fortnite://authorize/?code={}", urlencoded(&app.player_name)),
    );
    app.status = app.t("startingBackend").into();
    let ctx2 = ctx.clone();
    std::thread::Builder::new()
        .name("owen-launch".into())
        .spawn(move || {
            let st = owen_launcher::core::lawin::backend_manager::ensure_running(&data, &bundled, Some(&lv));
            let ok = matches!(
                st,
                owen_launcher::core::lawin::backend_manager::BackendState::Running { .. }
            );
            ctx2.request_repaint();
            if ok {
                let launched = platform::launch_game(&pkg2, &player);
                if !launched {
                    platform::open_url(&player);
                    app_fallback_note();
                }
            } else {
                no_backend_note();
            }
        })
        .ok();
    app.status = app.t("startingWord").into();
}

fn app_fallback_note() {}
fn no_backend_note() {}

fn urlencoded(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        let c = b as char;
        if c.is_ascii_alphanumeric() || "-_.~".contains(c) {
            out.push(c);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

/* -------------------------------- textures -------------------------------- */

fn season_texture(
    app: &mut OwenApp,
    ctx: &egui::Context,
    n: u32,
) -> Option<egui::TextureHandle> {
    if app.host.is_wasm() {
        return None;
    }
    if let Some(tex) = app.tex.get(&n) {
        return Some(tex.clone());
    }
    let path = app.data_root.join("images").join(format!("season_{n}_folder.png"));
    let bytes = std::fs::read(&path).ok()?;
    let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    let tex = ctx.load_texture(format!("season_{n}"), color, egui::TextureOptions::default());
    app.tex.insert(n, tex.clone());
    Some(tex)
}

// keep Path import referenced (used by signatures below in future passes)
#[allow(dead_code)]
fn _path_marker(_: &Path) {}
