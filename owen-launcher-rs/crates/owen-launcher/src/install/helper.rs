#![forbid(unsafe_code)]
//! 1:1 port of `PackageInstallHelper.kt` (object `PackageInstallHelper`) as
//! the [`Installer`] trait + [`InstallSession`].
//!
//! The Android `PackageInstaller` session lifecycle (createSession →
//! openWrite/copy per APK → commit with a PendingIntent → onResult) becomes:
//!
//! | Kotlin (`PackageInstallHelper`)            | Rust (here)                       |
//! |---------------------------------------------|-----------------------------------|
//! | `InstallResult(ok, statusCode, message)`    | [`InstallResult`]                 |
//! | `install(context, apks, onStatus)`          | [`Installer::install`]            |
//! | `installWithTimeout(context, apks)`         | [`Installer::install_with_timeout`] |
//! | `waiting[sessionId] = callback` + `onResult`| [`Installer::on_result`]          |
//! | `installer.createSession(params)`           | [`Installer::create_session`]     |
//! | `session.openWrite("apk_N") + copyTo`       | [`Installer::submit`]             |
//! | `session.commit(pendingIntent.intentSender)`| [`Installer::commit`]             |
//! | `session.abandon()` (cancellation)          | [`Installer::abandon`]            |
//!
//! The real Android implementation arrives later via JNI; for desktop and
//! preview builds use [`NoopInstaller`] (simulates success immediately).

use std::path::{Path, PathBuf};

/// `private const val RESULT_ACTION` (also re-declared by
/// `InstallResultReceiver`).
pub const RESULT_ACTION: &str = "com.abdallhrgx.owenlauncher.INSTALL_RESULT";

/// `private const val WAIT_TIMEOUT_MS = 900_000L` — the cap of
/// `installWithTimeout`. Implementations that actually block on the result
/// broadcast enforce it inside [`Installer::commit`].
pub const WAIT_TIMEOUT_MS: u64 = 900_000;

/// `PackageInstallHelper.InstallResult` — same fields, same `describe()`
/// messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallResult {
    pub ok: bool,
    pub status_code: i32,
    pub message: Option<String>,
}

impl InstallResult {
    /// Kotlin constructor `InstallResult(ok, statusCode, message)`.
    pub fn new(ok: bool, status_code: i32, message: Option<String>) -> Self {
        Self {
            ok,
            status_code,
            message,
        }
    }

    /// `InstallResult(true, 0, …)` — success.
    pub fn success() -> Self {
        Self::new(true, 0, None)
    }

    /// `InstallResult(false, code, message)` — failure.
    pub fn failure(status_code: i32, message: impl Into<String>) -> Self {
        Self::new(false, status_code, Some(message.into()))
    }

    /// `val signatureConflict: Boolean` — same substrings.
    pub fn signature_conflict(&self) -> bool {
        let message = self.message.as_deref().unwrap_or("");
        message.contains("INSTALL_FAILED_UPDATE_INCOMPATIBLE")
            || message.contains("signatures do not match")
            || message.contains("INSTALL_FAILED_VERIFICATION_FAILURE")
    }

    /// `fun describe(): String` — same messages, same order.
    pub fn describe(&self) -> String {
        if self.ok {
            return "Installed successfully.".to_string();
        }
        if self.signature_conflict() {
            return "The installed game has a different signature. Uninstall it first, then install again from Owen.".to_string();
        }
        match self.status_code {
            -2 => "Installation failed: the package is blocked by the system.".to_string(),
            _ => {
                let message = self.message.as_deref().unwrap_or("");
                format!(
                    "Installation failed (status {}{}).",
                    self.status_code,
                    if message.trim().is_empty() {
                        String::new()
                    } else {
                        format!(": {message}")
                    }
                )
            }
        }
    }
}

/// One open install session (`PackageInstaller.Session` + its staged APKs).
#[derive(Debug, Clone)]
pub struct InstallSession {
    /// The id Android's `createSession` returns (used by `onResult` routing).
    pub id: i32,
    /// APKs staged through `submit` so far (`openWrite("apk_N")` order).
    pub apks: Vec<PathBuf>,
}

/// The install backend — the trait replacing `object PackageInstallHelper` +
/// Android's `Context.packageManager.packageInstaller`.
pub trait Installer {
    /// `installer.createSession(SessionParams(MODE_FULL_INSTALL))`.
    fn create_session(&mut self) -> Result<InstallSession, String>;

    /// `session.openWrite("apk_$index", 0, apk.length())` + stream copy for
    /// one APK.
    fn submit(&mut self, session: &mut InstallSession, apk: &Path) -> Result<(), String>;

    /// `session.commit(pendingIntent.intentSender)` + waiting for the result
    /// broadcast. The status message `"Committing install session..."` is
    /// emitted by [`Installer::install`] right before this runs (the original
    /// emission point). Session errors map to
    /// `InstallResult(false, -99, message)` — the original `catch` branch.
    fn commit(
        &mut self,
        session: InstallSession,
        on_status: &mut dyn FnMut(String),
    ) -> InstallResult;

    /// `session.abandon()` — the `invokeOnCancellation` path.
    fn abandon(&mut self, session: InstallSession);

    /// `PackageInstallHelper.onResult(sessionId, status, message)` — deliver
    /// a final status to the waiting callback of a session
    /// (`InstallResult(status == 0, status, message ?: "")`). Returns
    /// whether a waiter was found for the session.
    fn on_result(&mut self, session_id: i32, status: i32, message: Option<String>) -> bool;

    /// `PackageInstallHelper.install(context, apks, onStatus)` — the full
    /// lifecycle, same step order:
    ///
    /// 1. `createSession`
    /// 2. `openWrite` + copy for every APK
    /// 3. `onStatus("Committing install session...")`
    /// 4. `commit` and wait for the result
    ///
    /// Any session error aborts (abandon + `waiting.remove`) and yields
    /// `InstallResult(false, -99, error)` like the original `catch`.
    fn install(&mut self, apks: &[PathBuf], on_status: &mut dyn FnMut(String)) -> InstallResult {
        let mut session = match self.create_session() {
            Ok(session) => session,
            Err(err) => return InstallResult::new(false, -99, Some(err)),
        };
        for apk in apks {
            if let Err(err) = self.submit(&mut session, apk) {
                self.abandon(session);
                return InstallResult::new(false, -99, Some(err));
            }
        }
        on_status("Committing install session...".to_string());
        let session_id = session.id;
        let result = self.commit(session, on_status);
        // DiagLog.i(TAG, "Session $sessionId committed with N apk(s)")
        eprintln!(
            "[PackageInstaller] Session {session_id} committed with {} apk(s)",
            apks.len()
        );
        result
    }

    /// `PackageInstallHelper.installWithTimeout(context, apks)` —
    /// [`Installer::install`] capped by [`WAIT_TIMEOUT_MS`]. The default
    /// delegates to `install`; blocking implementations (the future JNI
    /// bridge) enforce the timeout inside `commit`.
    fn install_with_timeout(&mut self, apks: &[PathBuf]) -> Option<InstallResult> {
        Some(self.install(apks, &mut |_| {}))
    }
}

/// Desktop / preview installer — simulates immediate success, as the Android
/// `PackageInstaller` port (JNI) does not exist yet. `submit` only verifies
/// the APK exists (the real session would stream it).
#[derive(Debug, Clone, Default)]
pub struct NoopInstaller {
    next_session_id: i32,
}

impl NoopInstaller {
    pub fn new() -> Self {
        Self {
            next_session_id: 0,
        }
    }
}

impl Installer for NoopInstaller {
    fn create_session(&mut self) -> Result<InstallSession, String> {
        self.next_session_id += 1;
        Ok(InstallSession {
            id: self.next_session_id,
            apks: Vec::new(),
        })
    }

    fn submit(&mut self, session: &mut InstallSession, apk: &Path) -> Result<(), String> {
        if !apk.is_file() {
            return Err(format!(
                "NoopInstaller: {} does not exist",
                apk.display()
            ));
        }
        session.apks.push(apk.to_path_buf());
        Ok(())
    }

    fn commit(
        &mut self,
        _session: InstallSession,
        _on_status: &mut dyn FnMut(String),
    ) -> InstallResult {
        InstallResult::success()
    }

    fn abandon(&mut self, _session: InstallSession) {}

    fn on_result(&mut self, _session_id: i32, _status: i32, _message: Option<String>) -> bool {
        // No waiters: NoopInstaller resolves `commit` synchronously.
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describe_success_and_conflicts() {
        assert_eq!(InstallResult::success().describe(), "Installed successfully.");
        for message in [
            "INSTALL_FAILED_UPDATE_INCOMPATIBLE",
            "signatures do not match the previously installed version",
            "INSTALL_FAILED_VERIFICATION_FAILURE",
        ] {
            let result = InstallResult::failure(5, message);
            assert!(result.signature_conflict());
            assert_eq!(
                result.describe(),
                "The installed game has a different signature. Uninstall it first, then install again from Owen."
            );
        }
        let clean = InstallResult::failure(5, "boom");
        assert!(!clean.signature_conflict());
    }

    #[test]
    fn describe_status_codes() {
        assert_eq!(
            InstallResult::failure(-2, "blocked").describe(),
            "Installation failed: the package is blocked by the system."
        );
        assert_eq!(
            InstallResult::failure(5, "some message").describe(),
            "Installation failed (status 5: some message)."
        );
        assert_eq!(
            InstallResult::new(false, 5, None).describe(),
            "Installation failed (status 5)."
        );
        assert_eq!(
            InstallResult::failure(5, "   ").describe(),
            "Installation failed (status 5).",
            "blank message is dropped"
        );
    }

    #[test]
    fn noop_installer_full_lifecycle() {
        let dir = std::env::temp_dir().join(format!("owen-noop-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let apk = dir.join("base.apk");
        std::fs::write(&apk, b"PK\x03\x04fake").unwrap();

        let mut installer = NoopInstaller::new();
        let mut statuses: Vec<String> = Vec::new();
        let result = installer.install(&[apk.clone()], &mut |s| statuses.push(s));

        assert!(result.ok, "NoopInstaller simulates success: {:?}", result);
        assert_eq!(result.describe(), "Installed successfully.");
        assert_eq!(statuses, vec!["Committing install session...".to_string()]);

        // Session ids grow like Android's createSession.
        let session = installer.create_session().unwrap();
        assert_eq!(session.id, 2);

        // install_with_timeout returns the same success.
        assert!(installer.install_with_timeout(&[apk]).unwrap().ok);

        // Submitting a missing APK fails the whole install with -99.
        let result = installer.install(&[dir.join("missing.apk")], &mut |_| {});
        assert!(!result.ok);
        assert_eq!(result.status_code, -99);
        assert!(result.message.unwrap().contains("missing.apk"));

        // on_result has no waiters to wake.
        assert!(!installer.on_result(1, 0, None));

        std::fs::remove_dir_all(&dir).ok();
    }
}
