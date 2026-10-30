//! App core — the Rust port of `com.abdallhrgx.owenlauncher.core.*`.

pub mod diag;
pub mod player_store;

pub mod catalogs {
    //! Generated 1:1 from the original Kotlin catalogs.
    pub mod season_catalog;
    pub mod version_catalog;
    // Runtime layer over the generated catalog (direct APK URL overrides,
    // read from `owen-direct.json` in the data root).
    pub mod direct_overrides;
}

pub mod lawin {
    //! Port of `core/lawin/*` — server settings, config.ini, data deployer,
    //! login web server and backend manager. The module keeps its original
    //! placement for the 1:1 map; all user-facing strings say "Owen Server".
    pub mod backend_manager;
    pub mod config;
    pub mod deployer;
    pub mod login_web_server;
    pub mod settings;
}
