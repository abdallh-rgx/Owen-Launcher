//! Server state: accounts, sessions, profiles, manifests, battle pass
//! data, redirect rules and the admin log — in memory with a data
//! directory for LawinServer-compatible assets. Accounts are persisted to
//! `accounts.json` (atomic write) on every mutation; sessions stay in
//! memory only.

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

/// Today's date as `YYYY-MM-DD` (for season active-window checks).
pub fn today() -> String {
    iso_now().get(..10).unwrap_or("2017-10-25").to_string()
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

/// Derive a 32-hex-char account id from a seed string (sha256, first
/// 16 bytes). Used for both quick accounts (seed = display name) and
/// registered accounts (seed = username domain, see
/// [`registered_account_id`]).
fn account_id_from_seed(seed: &str, salt: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    hasher.update(salt);
    let hash = hasher.finalize();
    hash.iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()[..32]
        .to_string()
}

/// A player account. Quick accounts (created implicitly by the game
/// login flow) have empty `username` / `email` / `password_hash`;
/// registered accounts (launcher flow) have all three.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub account_id: String,
    pub display_name: String,
    /// lowercase, unique, "" for quick accounts
    #[serde(default)]
    pub email: String,
    /// unique, case-insensitive; "" for quick accounts
    #[serde(default)]
    pub username: String,
    /// "" = quick account (no password, any login accepted)
    #[serde(default)]
    pub password_hash: String,
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
    /// PartyHub user settings (graphql `UserSettings.updateSetting`)
    #[serde(default)]
    pub settings: HashMap<String, Value>,
    /// friend account ids
    #[serde(default)]
    pub friends: Vec<String>,
    /// active battle pass season (None = server current)
    #[serde(default)]
    pub selected_season: Option<u32>,
}

impl Account {
    /// Legacy quick account keyed by display name (game login flow).
    pub fn new(display_name: String) -> Self {
        Self {
            account_id: account_id_from_seed(&display_name, b"owen-salt"),
            display_name,
            email: String::new(),
            username: String::new(),
            password_hash: String::new(),
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
            settings: HashMap::new(),
            friends: Vec::new(),
            selected_season: None,
        }
    }

    fn empty() -> Self {
        Self::new(String::new())
    }
}

/// Registered account id — sha256 of the username under a separate salt so
/// it never collides with a quick account id while keeping the 32-hex
/// format.
fn registered_account_id(username: &str) -> String {
    account_id_from_seed(&format!("owen-user:{username}"), b"owen-account-salt")
}

/// Registration validation failures (Epic-style error codes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationError {
    InvalidEmail,
    EmailTaken,
    InvalidUsername,
    UsernameTaken,
    InvalidDisplayName,
    WeakPassword,
}

impl RegistrationError {
    pub fn error_code(&self) -> &'static str {
        match self {
            RegistrationError::InvalidEmail => "errors.com.owen.account.invalid_email",
            RegistrationError::EmailTaken => "errors.com.owen.account.email_taken",
            RegistrationError::InvalidUsername => "errors.com.owen.account.invalid_username",
            RegistrationError::UsernameTaken => "errors.com.owen.account.username_taken",
            RegistrationError::InvalidDisplayName => "errors.com.owen.account.invalid_display_name",
            RegistrationError::WeakPassword => "errors.com.owen.account.weak_password",
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            RegistrationError::InvalidEmail => "Enter a valid email address.",
            RegistrationError::EmailTaken => "That email is already registered.",
            RegistrationError::InvalidUsername => "Usernames are 3-16 characters (letters, numbers, underscore, dot).",
            RegistrationError::UsernameTaken => "That username is taken.",
            RegistrationError::InvalidDisplayName => "Display names are 3-32 characters.",
            RegistrationError::WeakPassword => "Passwords must be 8-128 characters.",
        }
    }
}

/// `^[^@\s]+@[^@\s]+\.[^@\s]{2,}$` — implemented without a regex crate.
pub fn valid_email(email: &str) -> bool {
    let email = email.trim();
    if !(5..=254).contains(&email.len()) {
        return false;
    }
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    if local.is_empty() || domain.is_empty() {
        return false;
    }
    let bad = |c: char| c.is_whitespace();
    if local.chars().any(bad) || domain.chars().any(|c| bad(c) || c == '@') {
        return false;
    }
    match domain.rfind('.') {
        Some(i) => {
            let (name, tld) = domain.split_at(i);
            let tld = &tld[1..];
            !name.is_empty() && tld.chars().count() >= 2
        }
        None => false,
    }
}

/// 3..=16 chars of `[a-zA-Z0-9_.]`.
pub fn valid_username(username: &str) -> bool {
    let len = username.chars().count();
    (3..=16).contains(&len)
        && username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
}

/// 3..=32 chars.
pub fn valid_display_name(name: &str) -> bool {
    (3..=32).contains(&name.chars().count())
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
    pub by_username: RwLock<HashMap<String, String>>, // lowercase username -> account id
    pub by_email: RwLock<HashMap<String, String>>,  // email -> account id
    pub sessions: RwLock<HashMap<String, Session>>, // access token -> session
    pub exchange_codes: RwLock<HashMap<String, String>>, // code -> account id
    pub manifests: RwLock<ManifestCatalog>,
    pub battlepass_seasons: RwLock<HashMap<String, Value>>, // "Season2" -> json
    pub pak_reports: RwLock<Vec<owen_core::battlepass::SeasonBattlePass>>,
    pub scan_reports: RwLock<Vec<ScanRecord>>,
    pub log: RwLock<Vec<LogEntry>>,
    pub started: std::time::Instant,
    pub request_counter: AtomicU64,
    /// PartyHub static root (None = /partyhub files return 404).
    pub partyhub_dir: Option<PathBuf>,
    /// Extra battle pass search dir (CLI `--battlepass-dir`).
    pub extra_battlepass_dirs: Vec<PathBuf>,
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
        Self::load_with(data_dir, config, None, Vec::new())
    }

    pub fn load_with(
        data_dir: PathBuf,
        config: OwenConfig,
        partyhub_dir: Option<PathBuf>,
        extra_battlepass_dirs: Vec<PathBuf>,
    ) -> Self {
        let state = Self {
            redirect: RwLock::new(config.redirect.clone()),
            config: RwLock::new(config),
            data_dir,
            accounts: RwLock::new(HashMap::new()),
            by_name: RwLock::new(HashMap::new()),
            by_username: RwLock::new(HashMap::new()),
            by_email: RwLock::new(HashMap::new()),
            sessions: RwLock::new(HashMap::new()),
            exchange_codes: RwLock::new(HashMap::new()),
            manifests: RwLock::new(ManifestCatalog::new()),
            battlepass_seasons: RwLock::new(HashMap::new()),
            pak_reports: RwLock::new(Vec::new()),
            scan_reports: RwLock::new(Vec::new()),
            log: RwLock::new(Vec::new()),
            started: std::time::Instant::now(),
            request_counter: AtomicU64::new(0),
            partyhub_dir,
            extra_battlepass_dirs,
        };
        state.load_data_dir();
        state.load_accounts();
        state.load_sessions();
        state
    }

    /// Battle pass search dirs in priority order (first match wins).
    fn battlepass_dirs(&self) -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = self.extra_battlepass_dirs.clone();
        dirs.push(self.data_dir.join("battlepass"));
        dirs.push(self.data_dir.join("responses/Athena/BattlePass"));
        dirs
    }

    fn load_data_dir(&self) {
        let dir = &self.data_dir;
        if !dir.is_dir() {
            self.log_info(format!("data dir {} not found, using built-in defaults", dir.display()));
            return;
        }
        // BattlePass season responses from every search dir (first wins).
        {
            let mut seasons = self.battlepass_seasons.write().unwrap();
            let mut loaded = 0usize;
            for bp_dir in self.battlepass_dirs() {
                if !bp_dir.is_dir() {
                    continue;
                }
                for entry in std::fs::read_dir(&bp_dir).into_iter().flatten().flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let Some(season) = name.strip_suffix(".json") else {
                        continue;
                    };
                    if !season.starts_with("Season") || seasons.contains_key(season) {
                        continue;
                    }
                    if let Ok(data) = std::fs::read(entry.path()) {
                        if let Ok(v) = serde_json::from_slice::<Value>(&data) {
                            seasons.insert(season.to_string(), v);
                            loaded += 1;
                        }
                    }
                }
            }
            if loaded > 0 {
                self.log_info(format!("loaded {loaded} battle pass seasons"));
            }
        }
        // Launcher manifest catalog (`manifests.json`, the version/CL/CDN
        // list) — auto-loaded at boot so /launcher/api/public/assets/...
        // answers with the real Epic CDN manifest without any admin action.
        // POST /api/admin/manifests still upserts on top of these entries.
        let manifest_load = self
            .data_file("manifests.json")
            .map(|data| self.manifests.write().unwrap().merge_manifest_list(&data));
        match manifest_load {
            Some(Ok(count)) if count > 0 => {
                self.log_info(format!("loaded {count} manifest catalog entries from manifests.json"));
            }
            Some(Ok(_)) => {}
            Some(Err(e)) => {
                self.log_warn(format!("manifests.json is corrupt, manifest redirect disabled: {e}"))
            }
            None => {}
        }
    }

    fn accounts_path(&self) -> PathBuf {
        self.data_dir.join("accounts.json")
    }

    fn sessions_path(&self) -> PathBuf {
        self.data_dir.join("sessions.json")
    }

    /// Load persisted OAuth sessions so tokens survive server restarts
    /// (the launcher web UI and the PartyHub bridge both restore sessions).
    fn load_sessions(&self) {
        let Ok(data) = std::fs::read(self.sessions_path()) else {
            return;
        };
        let Ok(list) = serde_json::from_slice::<Vec<Session>>(&data) else {
            self.log_warn("sessions.json is corrupt, starting fresh".into());
            return;
        };
        let count = list.len();
        let mut sessions = self.sessions.write().unwrap();
        for s in list {
            sessions.insert(s.access_token.clone(), s);
        }
        if count > 0 {
            self.log_info(format!("restored {count} sessions from sessions.json"));
        }
    }

    /// Persist the session table atomically (tmp file + rename).
    pub fn persist_sessions(&self) {
        let list: Vec<Session> = self.sessions.read().unwrap().values().cloned().collect();
        let json = serde_json::to_string_pretty(&list).unwrap_or_else(|_| "[]".into());
        let path = self.sessions_path();
        let tmp = self.data_dir.join("sessions.json.tmp");
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = std::fs::write(&tmp, json).and_then(|_| std::fs::rename(&tmp, &path)) {
            self.log_warn(format!("could not persist sessions: {e}"));
        }
    }

    fn load_accounts(&self) {
        let Ok(data) = std::fs::read(self.accounts_path()) else {
            return;
        };
        let Ok(list) = serde_json::from_slice::<Vec<Account>>(&data) else {
            self.log_warn("accounts.json is corrupt, starting with no persisted accounts".into());
            return;
        };
        let mut accounts = self.accounts.write().unwrap();
        let mut by_name = self.by_name.write().unwrap();
        let mut by_username = self.by_username.write().unwrap();
        let mut by_email = self.by_email.write().unwrap();
        let count = list.len();
        for acc in list {
            if !acc.username.is_empty() {
                by_username.insert(acc.username.to_lowercase(), acc.account_id.clone());
            }
            if !acc.email.is_empty() {
                by_email.insert(acc.email.clone(), acc.account_id.clone());
            }
            by_name.insert(acc.display_name.clone(), acc.account_id.clone());
            accounts.insert(acc.account_id.clone(), acc);
        }
        self.log_info(format!("loaded {count} accounts from accounts.json"));
    }

    /// Persist the account list atomically (tmp file + rename).
    pub fn persist_accounts(&self) {
        let list: Vec<Account> = self.accounts.read().unwrap().values().cloned().collect();
        let json = serde_json::to_string_pretty(&list).unwrap_or_else(|_| "[]".into());
        let path = self.accounts_path();
        let tmp = self.data_dir.join("accounts.json.tmp");
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = std::fs::write(&tmp, json).and_then(|_| std::fs::rename(&tmp, &path)) {
            self.log_warn(format!("could not persist accounts: {e}"));
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

    /// Find-or-create an account by display name (legacy quick-account
    /// flow). Registered accounts are found by username, email or display
    /// name before a new quick account is created.
    pub fn account_by_name(&self, name: &str) -> Account {
        let existing = self
            .by_name
            .read()
            .unwrap()
            .get(name)
            .cloned()
            .or_else(|| {
                let lower = name.to_lowercase();
                self.by_username
                    .read()
                    .unwrap()
                    .get(&lower)
                    .cloned()
                    .or_else(|| self.by_email.read().unwrap().get(&lower).cloned())
            });
        if let Some(id) = existing {
            if let Some(acc) = self.accounts.read().unwrap().get(&id) {
                return acc.clone();
            }
        }
        let acc = Account::new(name.to_string());
        self.accounts
            .write()
            .unwrap()
            .insert(acc.account_id.clone(), acc.clone());
        self.by_name
            .write()
            .unwrap()
            .insert(name.to_string(), acc.account_id.clone());
        self.log_info(format!("created account '{name}' ({})", acc.account_id));
        self.persist_accounts();
        acc
    }

    pub fn account_by_id(&self, id: &str) -> Option<Account> {
        self.accounts.read().unwrap().get(id).cloned()
    }

    /// Look up an account by username (case-insensitive), email or display
    /// name — no creation, no password check.
    pub fn account_by_login(&self, login: &str) -> Option<Account> {
        let trimmed = login.trim();
        let lower = trimmed.to_lowercase();
        let id = self
            .by_username
            .read()
            .unwrap()
            .get(&lower)
            .cloned()
            .or_else(|| self.by_email.read().unwrap().get(&lower).cloned())
            .or_else(|| self.by_name.read().unwrap().get(trimmed).cloned());
        id.and_then(|id| self.account_by_id(&id))
    }

    /// Does any account (quick or registered) hold this login string as
    /// username, email or display name?
    pub fn login_exists(&self, login: &str) -> bool {
        let trimmed = login.trim();
        let lower = trimmed.to_lowercase();
        self.by_username.read().unwrap().contains_key(&lower)
            || self.by_email.read().unwrap().contains_key(&lower)
            || self.by_name.read().unwrap().contains_key(trimmed)
    }

    /// Register a new account with email/username/password. All identifiers
    /// are validated (see [`RegistrationError`]); the account is persisted
    /// immediately.
    pub fn register(
        &self,
        email: &str,
        username: &str,
        display_name: &str,
        password: &str,
    ) -> Result<Account, RegistrationError> {
        let email = email.trim().to_lowercase();
        let username = username.trim();
        let display_name = display_name.trim();
        if !valid_email(&email) {
            return Err(RegistrationError::InvalidEmail);
        }
        if self.by_email.read().unwrap().contains_key(&email) {
            return Err(RegistrationError::EmailTaken);
        }
        if !valid_username(username) {
            return Err(RegistrationError::InvalidUsername);
        }
        if self
            .by_username
            .read()
            .unwrap()
            .contains_key(&username.to_lowercase())
        {
            return Err(RegistrationError::UsernameTaken);
        }
        if !valid_display_name(display_name) {
            return Err(RegistrationError::InvalidDisplayName);
        }
        let pw_len = password.chars().count();
        if !(8..=128).contains(&pw_len) {
            return Err(RegistrationError::WeakPassword);
        }
        let acc = Account {
            account_id: registered_account_id(username),
            display_name: display_name.to_string(),
            email: email.clone(),
            username: username.to_string(),
            password_hash: crate::pbkdf2::hash_password(password),
            ..Account::empty()
        };
        self.accounts
            .write()
            .unwrap()
            .insert(acc.account_id.clone(), acc.clone());
        self.by_name
            .write()
            .unwrap()
            .insert(display_name.to_string(), acc.account_id.clone());
        self.by_username
            .write()
            .unwrap()
            .insert(username.to_lowercase(), acc.account_id.clone());
        self.by_email
            .write()
            .unwrap()
            .insert(email, acc.account_id.clone());
        self.persist_accounts();
        self.log_info(format!("registered account '{username}' ({})", acc.account_id));
        Ok(acc)
    }

    /// Verify a login (username, email or display name) + password.
    /// Quick accounts (no stored hash) accept any password — legacy game
    /// compatibility. Returns None when the credentials don't match.
    pub fn verify_login(&self, login: &str, password: &str) -> Option<Account> {
        let acc = self.account_by_login(login)?;
        if acc.password_hash.is_empty() {
            return Some(acc);
        }
        if crate::pbkdf2::verify_password(password, &acc.password_hash) {
            Some(acc)
        } else {
            None
        }
    }

    /// Apply a mutation to a stored account and persist. Returns the
    /// updated account.
    pub fn update_account<F>(&self, account_id: &str, f: F) -> Option<Account>
    where
        F: FnOnce(&mut Account),
    {
        let snapshot = {
            let mut accounts = self.accounts.write().unwrap();
            let acc = accounts.get_mut(account_id)?;
            f(acc);
            acc.clone()
        };
        self.persist_accounts();
        Some(snapshot)
    }

    pub fn set_selected_season(&self, account_id: &str, season: u32) -> Option<Account> {
        self.update_account(account_id, |a| a.selected_season = Some(season))
    }

    /// Store a PartyHub user setting (graphql `updateSetting`).
    pub fn set_setting(&self, account_id: &str, key: &str, value: Value) -> Option<Account> {
        self.update_account(account_id, |a| {
            a.settings.insert(key.to_string(), value);
        })
    }

    pub fn create_session(&self, account_id: &str) -> Session {
        use rand::Rng;
        // Alphanumeric tokens only: they travel in HTTP headers
        // (`Authorization: Bearer eg1~...`) and header values must stay
        // visible-ASCII — arbitrary `gen::<char>()` output breaks that.
        let rng = rand::thread_rng();
        let token: String = rng
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        let rng2 = rand::thread_rng();
        let refresh: String = rng2
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
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
        self.persist_sessions();
        session
    }

    pub fn session_for(&self, token: &str) -> Option<Session> {
        self.sessions.read().unwrap().get(token).cloned()
    }

    pub fn end_session(&self, token: &str) {
        self.sessions.write().unwrap().remove(token);
        self.persist_sessions();
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

    pub fn partyhub_dir(&self) -> Option<&Path> {
        self.partyhub_dir.as_deref()
    }

    /// Battle pass JSON for a season number (from the loaded cache).
    pub fn battlepass_for(&self, season: u32) -> Option<Value> {
        self.battlepass_seasons
            .read()
            .unwrap()
            .get(&format!("Season{season}"))
            .cloned()
    }

    /// Highest season number with a battle pass file (default 1).
    pub fn current_season(&self) -> u32 {
        self.battlepass_seasons
            .read()
            .unwrap()
            .keys()
            .filter_map(|k| {
                k.strip_prefix("Season")
                    .and_then(|s| s.parse::<u32>().ok())
            })
            .max()
            .unwrap_or(1)
    }

    /// Number of entries in the launcher `manifests.json` (0 when absent).
    pub fn manifests_count(&self) -> usize {
        self.data_file("manifests.json")
            .and_then(|d| serde_json::from_slice::<Value>(&d).ok())
            .and_then(|v| v.as_array().map(|a| a.len()))
            .unwrap_or(0)
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
                // Synthesize the LawinServer template defaults when
                // profiles/athena.json is missing.
                if !map.contains_key("wipeNumber") {
                    map.insert("wipeNumber".into(), Value::Number(7.into()));
                }
                if !map.contains_key("version") {
                    map.insert("version".into(), Value::Number(1.into()));
                }
                if !map.contains_key("items") {
                    map.insert("items".into(), Value::Array(Vec::new()));
                }
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

/// Shared test helper: a fresh [`AppState`] over a unique temp dir.
#[cfg(test)]
pub(crate) mod testing {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    static TEST_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

    pub fn test_state() -> (AppState, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "owen-server-test-{}-{}",
            std::process::id(),
            TEST_DIR_COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create test dir");
        let state = AppState::load(dir.clone(), OwenConfig::default());
        (state, dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testing::test_state;

    // ---- validators ------------------------------------------------------

    #[test]
    fn email_validator() {
        for good in [
            "user@example.com",
            "a.b+c@sub.domain.org",
            "player@fortnite.co.uk",
            "x@y.io",
            "UPPER@EXAMPLE.COM",
        ] {
            assert!(valid_email(good), "{good} should be valid");
        }
        for bad in [
            "",
            "plain",
            "a@b",
            "a @b.com",
            "a@b .com",
            "a@.com",
            "@b.com",
            "a@b.c",
            "a@b@c.com",
            "no-tld@domain",
        ] {
            assert!(!valid_email(bad), "{bad} should be invalid");
        }
        // length bounds (5..=254)
        assert!(valid_email(&format!("{}@b.cd", "a".repeat(248))), "253 chars ok");
        assert!(valid_email(&format!("{}@b.cd", "a".repeat(249))), "254 chars ok");
        assert!(!valid_email(&format!("{}@b.cd", "a".repeat(250))), "255 chars too long");
    }

    #[test]
    fn username_validator() {
        for good in ["abc", "user_1", "a.b", "Player99", "abcdefghijklmnop"] {
            assert!(valid_username(good), "{good} should be valid");
        }
        for bad in ["ab", "abcdefghijklmnopq", "has space", "dash-user", "üser", ""] {
            assert!(!valid_username(bad), "{bad} should be invalid");
        }
    }

    #[test]
    fn display_name_validator() {
        assert!(!valid_display_name("ab"));
        assert!(valid_display_name("abc"));
        assert!(valid_display_name(&"x".repeat(32)));
        assert!(!valid_display_name(&"x".repeat(33)));
    }

    #[test]
    fn account_id_is_32_hex() {
        let acc = Account::new("SomePlayer".into());
        assert_eq!(acc.account_id.len(), 32);
        assert!(acc.account_id.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(acc.account_id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
        let registered = registered_account_id("alice");
        assert_eq!(registered.len(), 32);
        // quick and registered namespaces never collide
        assert_ne!(registered, Account::new("owen-user:alice".into()).account_id);
    }

    // ---- registration ----------------------------------------------------

    #[test]
    fn register_validation_errors() {
        let (state, _dir) = test_state();
        // success first so duplicates can be tested
        state
            .register("alice@example.com", "alice", "Alice", "password123")
            .expect("register should succeed");

        let cases = [
            ("not-an-email", "bob", "Bob", "password123", RegistrationError::InvalidEmail),
            ("a@b.c", "bob", "Bob", "password123", RegistrationError::InvalidEmail),
            ("alice@example.com", "bob", "Bob", "password123", RegistrationError::EmailTaken),
            ("ALICE@EXAMPLE.COM", "bob", "Bob", "password123", RegistrationError::EmailTaken),
            ("bob@example.com", "bo", "Bob", "password123", RegistrationError::InvalidUsername),
            ("bob@example.com", "bad name!", "Bob", "password123", RegistrationError::InvalidUsername),
            ("bob@example.com", "alice", "Bob", "password123", RegistrationError::UsernameTaken),
            ("bob@example.com", "ALICE", "Bob", "password123", RegistrationError::UsernameTaken),
            ("bob@example.com", "bob", "bb", "password123", RegistrationError::InvalidDisplayName),
            ("bob@example.com", "bob", "Bob", "short", RegistrationError::WeakPassword),
            (
                "bob@example.com",
                "bob",
                "Bob",
                &"x".repeat(129),
                RegistrationError::WeakPassword,
            ),
        ];
        for (email, username, display, password, expected) in cases {
            let err = state
                .register(email, username, display, password)
                .expect_err(&format!("{username} should fail"));
            assert_eq!(err, expected, "case {username}/{email}");
            assert!(err.error_code().starts_with("errors.com.owen.account."));
        }
    }

    #[test]
    fn register_success_and_persistence() {
        let (state, dir) = test_state();
        let acc = state
            .register("carol@example.com", "carol", "Carol", "hunter2hunter2")
            .expect("register");
        assert_eq!(acc.username, "carol");
        assert_eq!(acc.email, "carol@example.com");
        assert_eq!(acc.display_name, "Carol");
        assert!(!acc.password_hash.is_empty());
        assert!(acc.password_hash.starts_with("pbkdf2$"));
        assert_eq!(acc.account_id.len(), 32);

        // persisted to disk...
        assert!(dir.join("accounts.json").is_file());
        // ...and reloads with all indexes intact
        let state2 = AppState::load(dir.clone(), OwenConfig::default());
        let reloaded = state2.account_by_login("carol").expect("carol after reload");
        assert_eq!(reloaded.account_id, acc.account_id);
        assert_eq!(reloaded.email, "carol@example.com");
        let by_email = state2.account_by_login("carol@example.com").expect("by email");
        assert_eq!(by_email.account_id, acc.account_id);
        let by_name = state2.account_by_login("Carol").expect("by display name");
        assert_eq!(by_name.account_id, acc.account_id);
    }

    #[test]
    fn register_defaults_display_name_to_username() {
        let (state, _dir) = test_state();
        let acc = state
            .register("dave@example.com", "dave", "dave", "password123")
            .expect("register");
        assert_eq!(acc.display_name, "dave");
    }

    // ---- verify_login ----------------------------------------------------

    #[test]
    fn verify_login_registered_account() {
        let (state, _dir) = test_state();
        // low-rounds hash inserted directly to keep the test fast
        let mut acc = Account::empty();
        acc.username = "erin".into();
        acc.email = "erin@example.com".into();
        acc.display_name = "Erin".into();
        acc.password_hash = crate::pbkdf2::hash_password_with_rounds("super-secret", 64);
        state
            .accounts
            .write()
            .unwrap()
            .insert(acc.account_id.clone(), acc.clone());
        state
            .by_username
            .write()
            .unwrap()
            .insert("erin".into(), acc.account_id.clone());
        state
            .by_email
            .write()
            .unwrap()
            .insert("erin@example.com".into(), acc.account_id.clone());
        state
            .by_name
            .write()
            .unwrap()
            .insert("Erin".into(), acc.account_id.clone());

        // by username
        assert_eq!(
            state.verify_login("erin", "super-secret").map(|a| a.account_id),
            Some(acc.account_id.clone())
        );
        // by email, case-insensitive
        assert_eq!(
            state.verify_login("ERIN@EXAMPLE.COM", "super-secret").map(|a| a.account_id),
            Some(acc.account_id.clone())
        );
        // by display name
        assert_eq!(
            state.verify_login("Erin", "super-secret").map(|a| a.account_id),
            Some(acc.account_id.clone())
        );
        // wrong password
        assert!(state.verify_login("erin", "wrong-password").is_none());
        assert!(state.verify_login("erin@example.com", "").is_none());
    }

    #[test]
    fn verify_login_quick_account_accepts_any_password() {
        let (state, _dir) = test_state();
        let acc = state.account_by_name("QuickPlayer");
        assert_eq!(
            state.verify_login("QuickPlayer", "anything").map(|a| a.account_id),
            Some(acc.account_id.clone())
        );
        assert_eq!(
            state.verify_login("QuickPlayer", "").map(|a| a.account_id),
            Some(acc.account_id)
        );
        // quick accounts are persisted too
        assert!(state.login_exists("QuickPlayer"));
    }

    #[test]
    fn account_by_name_finds_registered_before_creating_quick() {
        let (state, _dir) = test_state();
        let registered = state
            .register("frank@example.com", "frank", "Frank", "password123")
            .expect("register");
        // the game flow posts the display name — must find the registered
        // account, not create a duplicate quick one
        let found = state.account_by_name("Frank");
        assert_eq!(found.account_id, registered.account_id);
        // ...and by username too
        let by_user = state.account_by_name("frank");
        assert_eq!(by_user.account_id, registered.account_id);
        assert_eq!(state.accounts.read().unwrap().len(), 1);
    }

    // ---- seasons / battlepass --------------------------------------------

    #[test]
    fn current_season_is_highest_loaded() {
        let (state, _dir) = test_state();
        assert_eq!(state.current_season(), 1, "no battlepass files → 1");
        state.battlepass_seasons.write().unwrap().insert(
            "Season12".into(),
            serde_json::json!({"paidRewards": [{}], "freeRewards": []}),
        );
        state.battlepass_seasons.write().unwrap().insert(
            "Season9".into(),
            serde_json::json!({}),
        );
        assert_eq!(state.current_season(), 12);
        assert!(state.battlepass_for(12).is_some());
        assert!(state.battlepass_for(13).is_none());
    }

    #[test]
    fn battlepass_loaded_from_both_dirs_first_wins() {
        let (_state, dir) = test_state();
        std::fs::create_dir_all(dir.join("battlepass")).unwrap();
        std::fs::create_dir_all(dir.join("responses/Athena/BattlePass")).unwrap();
        std::fs::write(
            dir.join("battlepass/Season5.json"),
            r#"{"source": "battlepass-dir"}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("responses/Athena/BattlePass/Season5.json"),
            r#"{"source": "responses-dir"}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("responses/Athena/BattlePass/Season6.json"),
            r#"{"source": "responses-dir-6"}"#,
        )
        .unwrap();
        // unrelated files are ignored
        std::fs::write(dir.join("battlepass/seasonPasses.json"), r#"{}"#).unwrap();

        let state2 = AppState::load(dir.clone(), OwenConfig::default());
        assert_eq!(state2.battlepass_for(5).unwrap()["source"], "battlepass-dir");
        assert_eq!(state2.battlepass_for(6).unwrap()["source"], "responses-dir-6");
        assert!(state2.battlepass_for(7).is_none());
        assert_eq!(state2.current_season(), 6);
    }

    #[test]
    fn manifests_json_autoloads_into_catalog() {
        let (state, dir) = test_state();
        // no manifests.json → empty catalog (the old, broken 12.41 flow)
        assert!(state.manifests.read().unwrap().entries.is_empty());

        std::fs::write(
            dir.join("manifests.json"),
            r#"[{
                "version": "12.41",
                "cl": "12910313",
                "releaseString": "++Fortnite+Release-12.41-CL-12910313-Android",
                "manifestHash": "aEzFZyiSFDrqiS_7jkqD6K8KN4zQEg",
                "label": "12.41-CL-12910313",
                "date": "2020-02-20",
                "cdn": "https://epicgames-download1.akamaized.net/Builds/Fortnite/Content/CloudDir/aEzFZyiSFDrqiS_7jkqD6K8KN4zQEg.manifest"
            }]"#,
        )
        .unwrap();

        let state2 = AppState::load(dir.clone(), OwenConfig::default());
        let manifests = state2.manifests.read().unwrap();
        assert_eq!(manifests.entries.len(), 1);
        let e = manifests
            .find_release("++Fortnite+Release-12.41-CL-12910313-Android")
            .expect("12.41 entry after autoload");
        assert_eq!(
            e.manifest_url,
            "https://epicgames-download1.akamaized.net/Builds/Fortnite/Content/CloudDir/aEzFZyiSFDrqiS_7jkqD6K8KN4zQEg.manifest"
        );
    }

    #[test]
    fn profile_for_synthesizes_athena_template() {
        let (state, _dir) = test_state();
        let acc = state.account_by_name("Profiler");
        let profile = state.profile_for(&acc, "athena");
        assert_eq!(profile["_id"], acc.account_id.as_str());
        assert_eq!(profile["accountId"], acc.account_id.as_str());
        assert_eq!(profile["wipeNumber"], 7);
        assert_eq!(profile["version"], 1);
        assert_eq!(profile["items"].as_array().unwrap().len(), 0);
        assert_eq!(profile["stats"]["attributes"]["level"], 1);
        assert_eq!(profile["stats"]["attributes"]["battlepass_purchased"], false);
    }

    #[test]
    fn set_selected_season_and_setting_persist() {
        let (state, dir) = test_state();
        let acc = state.account_by_name("Setter");
        state.set_selected_season(&acc.account_id, 12).expect("set season");
        state
            .set_setting(&acc.account_id, "favoriteMode", serde_json::json!("solo"))
            .expect("set setting");

        let state2 = AppState::load(dir, OwenConfig::default());
        let acc2 = state2.account_by_id(&acc.account_id).expect("persisted");
        assert_eq!(acc2.selected_season, Some(12));
        assert_eq!(acc2.settings.get("favoriteMode"), Some(&serde_json::json!("solo")));
    }
}
