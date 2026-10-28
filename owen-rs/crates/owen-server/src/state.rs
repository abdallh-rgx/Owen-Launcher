//! Server state: accounts, sessions, profiles, manifests, battle pass
//! data, redirect rules and the admin log — all in memory with a data
//! directory for LawinServer-compatible assets.

use owen_core::config::OwenConfig;
use owen_core::manifest::ManifestCatalog;
use owen_core::urlrule::RedirectRules;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn iso_now() -> String {
    let ms = now_ms();
    let secs = ms / 1000;
    let millis = ms % 1000;
    let (y, mo, d, h, mi, s) = civil_from_unix(secs as i64);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{millis:03}Z")
}

fn civil_from_unix(t: i64) -> (i64, u32, u32, u32, u32, u32) {
    let days = t.div_euclid(86400);
    let secs = t.rem_euclid(86400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (
        y,
        m,
        d,
        (secs / 3600) as u32,
        ((secs % 3600) / 60) as u32,
        (secs % 60) as u32,
    )
}

/// A player account (the web panel manages these — replacing the Discord
/// bot with a professional interface).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub account_id: String,
    pub display_name: String,
    pub created: String,
    #[serde(default)]
    pub banned: bool,
    #[serde(default)]
    pub whitelisted: bool,
    #[serde(default)]
    pub vbucks: i64,
    #[serde(default)]
    pub level: u32,
    #[serde(default)]
    pub battle_pass_purchased: bool,
    #[serde(default)]
    pub sac_code: String,
    /// profile rvn per profileId
    #[serde(default)]
    pub rvn: HashMap<String, i32>,
    #[serde(default)]
    pub locker: HashMap<String, Value>,
    #[serde(default)]
    pub items: Vec<String>,
}

impl Account {
    pub fn new(display_name: String) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(display_name.as_bytes());
        hasher.update(b"owen-salt");
        let hash = hasher.finalize();
        let account_id = hash
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()[..32]
            .to_string();
        Self {
            account_id,
            display_name,
            created: iso_now(),
            banned: false,
            whitelisted: false,
            vbucks: 5000,
            level: 1,
            battle_pass_purchased: false,
            sac_code: String::new(),
            rvn: HashMap::new(),
            locker: HashMap::new(),
            items: Vec::new(),
        }
    }
}

/// A live OAuth session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    pub account_id: String,
    pub created: u64,
    pub expires_in: u64,
    pub client_id: String,
    pub token_type: String,
    pub application: String,
}

/// Admin log ring buffer entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub ts: String,
    pub level: String,
    pub message: String,
}

pub struct AppState {
    pub data_dir: PathBuf,
    pub config: RwLock<OwenConfig>,
    pub redirect: RwLock<RedirectRules>,
    pub accounts: RwLock<HashMap<String, Account>>, // by account_id
    pub by_name: RwLock<HashMap<String, String>>,   // display name -> account id
    pub sessions: RwLock<HashMap<String, Session>>, // access token -> session
    pub exchange_codes: RwLock<HashMap<String, String>>, // code -> account id
    pub manifests: RwLock<ManifestCatalog>,
    pub battlepass_seasons: RwLock<HashMap<String, Value>>, // "Season2" -> json
    pub pak_reports: RwLock<Vec<owen_core::battlepass::SeasonBattlePass>>,
    pub scan_reports: RwLock<Vec<ScanRecord>>,
    pub log: RwLock<Vec<LogEntry>>,
    pub started: std::time::Instant,
    pub request_counter: AtomicU64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanRecord {
    pub ts: String,
    pub file_name: String,
    pub size: u64,
    pub report: owen_core::ScanReport,
}

impl AppState {
    pub fn load(data_dir: PathBuf, config: OwenConfig) -> Self {
        let state = Self {
            redirect: RwLock::new(config.redirect.clone()),
            config: RwLock::new(config),
            data_dir,
            accounts: RwLock::new(HashMap::new()),
            by_name: RwLock::new(HashMap::new()),
            sessions: RwLock::new(HashMap::new()),
            exchange_codes: RwLock::new(HashMap::new()),
            manifests: RwLock::new(ManifestCatalog::new()),
            battlepass_seasons: RwLock::new(HashMap::new()),
            pak_reports: RwLock::new(Vec::new()),
            scan_reports: RwLock::new(Vec::new()),
            log: RwLock::new(Vec::new()),
            started: std::time::Instant::now(),
            request_counter: AtomicU64::new(0),
        };
        state.load_data_dir();
        state
    }

    fn load_data_dir(&self) {
        let dir = &self.data_dir;
        if !dir.is_dir() {
            self.log_info(format!("data dir {} not found, using built-in defaults", dir.display()));
            return;
        }
        // BattlePass season responses
        let bp_dir = dir.join("responses/Athena/BattlePass");
        if bp_dir.is_dir() {
            let mut seasons = self.battlepass_seasons.write().unwrap();
            for entry in std::fs::read_dir(&bp_dir).into_iter().flatten().flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if let Some(season) = name.strip_suffix(".json") {
                    if let Ok(data) = std::fs::read(entry.path()) {
                        if let Ok(v) = serde_json::from_slice::<Value>(&data) {
                            seasons.insert(season.to_string(), v);
                        }
                    }
                }
            }
            self.log_info(format!("loaded {} battle pass seasons", seasons.len()));
        }
    }

    pub fn log_info(&self, msg: String) {
        println!("[i] {msg}");
        self.push_log("info", msg);
    }

    pub fn log_warn(&self, msg: String) {
        println!("[w] {msg}");
        self.push_log("warn", msg);
    }

    fn push_log(&self, level: &str, msg: String) {
        let mut log = self.log.write().unwrap();
        log.push(LogEntry {
            ts: iso_now(),
            level: level.to_string(),
            message: msg,
        });
        if log.len() > 500 {
            log.drain(..100);
        }
    }

    pub fn log_request(&self, line: String) {
        self.request_counter.fetch_add(1, Ordering::Relaxed);
        self.push_log("http", line);
    }

    /// Find-or-create an account by display name.
    pub fn account_by_name(&self, name: &str) -> Account {
        {
            let by_name = self.by_name.read().unwrap();
            if let Some(id) = by_name.get(name) {
                let accounts = self.accounts.read().unwrap();
                if let Some(acc) = accounts.get(id) {
                    return acc.clone();
                }
            }
        }
        let acc = Account::new(name.to_string());
        self.accounts.write().unwrap().insert(acc.account_id.clone(), acc.clone());
        self.by_name.write().unwrap().insert(name.to_string(), acc.account_id.clone());
        self.log_info(format!("created account '{name}' ({})", acc.account_id));
        acc
    }

    pub fn account_by_id(&self, id: &str) -> Option<Account> {
        self.accounts.read().unwrap().get(id).cloned()
    }

    pub fn create_session(&self, account_id: &str) -> Session {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let token: String = (0..32).map(|_| rng.gen::<char>()).collect();
        let refresh: String = (0..32).map(|_| rng.gen::<char>()).collect();
        let session = Session {
            access_token: format!("eg1~{token}"),
            refresh_token: format!("eg1~{refresh}"),
            account_id: account_id.to_string(),
            created: now_ms(),
            expires_in: 28800,
            client_id: "owen-client".into(),
            token_type: "bearer".into(),
            application: "fortnite".into(),
        };
        self.sessions
            .write()
            .unwrap()
            .insert(session.access_token.clone(), session.clone());
        session
    }

    pub fn session_for(&self, token: &str) -> Option<Session> {
        self.sessions.read().unwrap().get(token).cloned()
    }

    pub fn create_exchange_code(&self, account_id: &str) -> String {
        use rand::Rng;
        let code: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        self.exchange_codes
            .write()
            .unwrap()
            .insert(code.clone(), account_id.to_string());
        code
    }

    pub fn exchange_account(&self, code: &str) -> Option<String> {
        self.exchange_codes.read().unwrap().get(code).cloned()
    }

    /// Resolve the bearer token of a request to an account.
    pub fn auth_account(&self, headers: &axum::http::HeaderMap) -> Option<Account> {
        let auth = headers.get("authorization")?.to_str().ok()?;
        let token = auth.strip_prefix("Bearer ").or(auth.strip_prefix("bearer "))?;
        let session = self.session_for(token)?;
        self.account_by_id(&session.account_id)
    }

    /// Load a Lawin data file (profile, response, cloudstorage...).
    pub fn data_file(&self, rel: &str) -> Option<Vec<u8>> {
        let p: PathBuf = self.data_dir.join(rel);
        std::fs::read(p).ok()
    }

    pub fn data_dir_exists(&self) -> bool {
        Path::new(&self.data_dir).is_dir()
    }

    /// Load a profile JSON for an account, overlaying account state.
    pub fn profile_for(&self, account: &Account, profile_id: &str) -> Value {
        let template = self
            .data_file(&format!("profiles/{profile_id}.json"))
            .and_then(|d| serde_json::from_slice::<Value>(&d).ok())
            .unwrap_or_else(|| Value::Object(Default::default()));

        let mut v = template;
        if let Value::Object(map) = &mut v {
            map.insert("_id".into(), Value::String(account.account_id.clone()));
            map.insert("accountId".into(), Value::String(account.account_id.clone()));
            map.insert(
                "rvn".into(),
                Value::Number((account.rvn.get(profile_id).copied().unwrap_or(1)).into()),
            );
            map.insert("updated".into(), Value::String(iso_now()));
            map.insert("created".into(), Value::String(account.created.clone()));
            if profile_id == "athena" {
                let mut attrs = map
                    .get("stats")
                    .and_then(|s| s.get("attributes"))
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({}));
                if let Value::Object(attrs_map) = &mut attrs {
                    attrs_map.insert(
                        "battlepass_purchased".into(),
                        Value::Bool(account.battle_pass_purchased),
                    );
                    attrs_map.insert("level".into(), Value::Number(account.level.into()));
                    if !account.locker.is_empty() {
                        let mut locker = serde_json::json!({});
                        for (k, val) in &account.locker {
                            locker[k.clone()] = val.clone();
                        }
                        attrs_map.insert("locker".into(), locker);
                    }
                }
                map.insert(
                    "stats".into(),
                    serde_json::json!({ "attributes": attrs }),
                );
            }
        }
        v
    }
}
