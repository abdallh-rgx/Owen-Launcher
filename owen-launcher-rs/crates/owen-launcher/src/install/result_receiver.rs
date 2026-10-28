#![forbid(unsafe_code)]
//! 1:1 port of `InstallResultReceiver.kt` (`BroadcastReceiver`) as
//! [`InstallResultBus`].
//!
//! The Android broadcast Intent becomes [`InstallBroadcast`] (plain data),
//! and `onReceive` becomes [`InstallResultBus::on_receive`] with the same
//! routing and the same English messages:
//!
//! * no status extras → logged and ignored;
//! * `STATUS_PENDING_USER_ACTION` without the dialog intent →
//!   `onResult(sessionId, STATUS_FAILURE, "The system asked for install
//!   confirmation but sent no dialog intent")`;
//! * `STATUS_PENDING_USER_ACTION` with the intent → the original performed
//!   `startActivity(pending)`; here the confirmation is surfaced to the
//!   registered callbacks as [`PackageInstallStatus::PendingUserAction`] and
//!   the app layer presents the dialog (the JNI bridge later turns
//!   [`UserAction`] back into the `startActivity` call). A failure to open
//!   the dialog is reported back via
//!   [`InstallResultBus::report_user_action_failed`] with the original
//!   message;
//! * any final status → `PackageInstallHelper.onResult` (here:
//!   [`Installer::on_result`]) plus a [`PackageInstallStatus`] event for the
//!   registered callbacks.

use super::helper::Installer;

/// `PackageInstaller.STATUS_PENDING_USER_ACTION` (= -1).
pub const STATUS_PENDING_USER_ACTION: i32 = -1;
/// `PackageInstaller.STATUS_SUCCESS` (= 0).
pub const STATUS_SUCCESS: i32 = 0;
/// `PackageInstaller.STATUS_FAILURE` (= 1).
pub const STATUS_FAILURE: i32 = 1;

/// The system install-confirmation dialog extracted from a
/// pending-user-action broadcast (`Intent.EXTRA_INTENT` in the original
/// receiver). Opaque here; the app layer / JNI bridge presents it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserAction {
    /// Opaque handle of the dialog to open (e.g. the Android Intent URI).
    pub intent_uri: String,
}

/// The status events delivered to the registered callbacks — the same three
/// outcomes the original receiver distinguished.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageInstallStatus {
    /// `status == 0` — the session installed successfully.
    Success { session_id: i32 },
    /// A final failure status with the `PackageInstaller` status code and
    /// message.
    Failure {
        session_id: i32,
        status_code: i32,
        message: String,
    },
    /// `STATUS_PENDING_USER_ACTION` — the system wants the user to confirm;
    /// present `confirmation` to the user (the original opened the system
    /// dialog with `startActivity`).
    PendingUserAction {
        session_id: i32,
        confirmation: UserAction,
    },
}

/// The broadcast Intent delivered to `InstallResultReceiver`, as plain data.
#[derive(Debug, Clone)]
pub struct InstallBroadcast {
    /// `PackageInstaller.EXTRA_SESSION_ID` (default -1 in the original).
    pub session_id: i32,
    /// `PackageInstaller.EXTRA_STATUS` — `None` when the broadcast arrived
    /// without status extras (the original logs and ignores it).
    pub status: Option<i32>,
    /// `PackageInstaller.EXTRA_STATUS_MESSAGE` (`""` default).
    pub status_message: Option<String>,
    /// `Intent.EXTRA_INTENT` — present only for pending-user-action
    /// broadcasts.
    pub user_action: Option<UserAction>,
}

/// Callback type registered on the bus.
pub type InstallResultCallback = Box<dyn Fn(&PackageInstallStatus) + Send + Sync>;

/// The receiver-side bus: register callbacks on [`PackageInstallStatus`] and
/// feed incoming broadcasts through [`InstallResultBus::on_receive`].
#[derive(Default)]
pub struct InstallResultBus {
    callbacks: Vec<InstallResultCallback>,
}

impl InstallResultBus {
    pub fn new() -> Self {
        Self {
            callbacks: Vec::new(),
        }
    }

    /// Register a callback fired for every [`PackageInstallStatus`]
    /// (Success / Failure / PendingUserAction, same semantics as the original
    /// receiver's branches).
    pub fn register(&mut self, callback: InstallResultCallback) {
        self.callbacks.push(callback);
    }

    /// `InstallResultReceiver.onReceive(context, intent)` — 1:1 routing.
    pub fn on_receive(&mut self, installer: &mut dyn Installer, broadcast: &InstallBroadcast) {
        let session_id = broadcast.session_id;
        let Some(status) = broadcast.status else {
            // DiagLog.e(TAG, "session $sessionId: result arrived without status extras")
            eprintln!(
                "[InstallResult] session {session_id}: result arrived without status extras"
            );
            return;
        };
        let message = broadcast.status_message.clone().unwrap_or_default();

        if status == STATUS_PENDING_USER_ACTION {
            let Some(pending) = &broadcast.user_action else {
                // DiagLog.e(TAG, "session $sessionId: pending user action without an intent")
                eprintln!("[InstallResult] session {session_id}: pending user action without an intent");
                let failure_message =
                    "The system asked for install confirmation but sent no dialog intent";
                installer.on_result(session_id, STATUS_FAILURE, Some(failure_message.to_string()));
                self.emit(PackageInstallStatus::Failure {
                    session_id,
                    status_code: STATUS_FAILURE,
                    message: failure_message.to_string(),
                });
                return;
            };
            // Original: pending.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            // context.startActivity(pending) → here the app layer presents the
            // confirmation.
            self.emit(PackageInstallStatus::PendingUserAction {
                session_id,
                confirmation: pending.clone(),
            });
            // DiagLog.i(TAG, "session $sessionId: opened the system install confirmation")
            eprintln!(
                "[InstallResult] session {session_id}: opened the system install confirmation"
            );
            return;
        }

        // DiagLog.i(TAG, "session $sessionId status=$status msg=$message")
        eprintln!(
            "[InstallResult] session {session_id} status={status}{}",
            if message.trim().is_empty() {
                String::new()
            } else {
                format!(" msg={message}")
            }
        );
        // PackageInstallHelper.onResult(sessionId, status, message)
        installer.on_result(session_id, status, Some(message.clone()));
        if status == STATUS_SUCCESS {
            self.emit(PackageInstallStatus::Success { session_id });
        } else {
            self.emit(PackageInstallStatus::Failure {
                session_id,
                status_code: status,
                message,
            });
        }
    }

    /// Report that presenting the confirmation dialog failed — the original's
    /// `startActivity` catch branch
    /// (`"Could not open the system install dialog: " + t.message`).
    pub fn report_user_action_failed(
        &mut self,
        installer: &mut dyn Installer,
        session_id: i32,
        error: &str,
    ) {
        let message = format!("Could not open the system install dialog: {error}");
        installer.on_result(session_id, STATUS_FAILURE, Some(message.clone()));
        self.emit(PackageInstallStatus::Failure {
            session_id,
            status_code: STATUS_FAILURE,
            message,
        });
    }

    fn emit(&mut self, status: PackageInstallStatus) {
        // Iterate by index: a callback may (de)register callbacks of its own.
        let mut index = 0;
        while index < self.callbacks.len() {
            self.callbacks[index](&status);
            index += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::install::{InstallResult, InstallSession};
    use std::cell::RefCell;
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    /// Recorder standing in for the future JNI installer — records every
    /// `on_result` delivery.
    struct RecordingInstaller {
        results: RefCell<Vec<(i32, i32, Option<String>)>>,
    }

    impl Installer for RecordingInstaller {
        fn create_session(&mut self) -> Result<InstallSession, String> {
            Ok(InstallSession {
                id: 1,
                apks: Vec::new(),
            })
        }
        fn submit(&mut self, _session: &mut InstallSession, _apk: &Path) -> Result<(), String> {
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
        fn on_result(&mut self, session_id: i32, status: i32, message: Option<String>) -> bool {
            self.results.borrow_mut().push((session_id, status, message));
            true
        }
    }

    fn bus_with_recorder()
    -> (InstallResultBus, RecordingInstaller, Arc<Mutex<Vec<PackageInstallStatus>>>) {
        let received: Arc<Mutex<Vec<PackageInstallStatus>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&received);
        let mut bus = InstallResultBus::new();
        bus.register(Box::new(move |status: &PackageInstallStatus| {
            sink.lock().unwrap().push(status.clone());
        }));
        (
            bus,
            RecordingInstaller {
                results: RefCell::new(Vec::new()),
            },
            received,
        )
    }

    #[test]
    fn broadcast_without_status_is_ignored() {
        let (mut bus, mut installer, received) = bus_with_recorder();
        bus.on_receive(
            &mut installer,
            &InstallBroadcast {
                session_id: 3,
                status: None,
                status_message: None,
                user_action: None,
            },
        );
        assert!(installer.results.borrow().is_empty(), "no on_result");
        assert!(received.lock().unwrap().is_empty(), "no callback events");
    }

    #[test]
    fn pending_user_action_without_intent_fails_the_session() {
        let (mut bus, mut installer, received) = bus_with_recorder();
        bus.on_receive(
            &mut installer,
            &InstallBroadcast {
                session_id: 7,
                status: Some(STATUS_PENDING_USER_ACTION),
                status_message: None,
                user_action: None,
            },
        );
        assert_eq!(
            installer.results.borrow().as_slice(),
            &[(7, STATUS_FAILURE, Some("The system asked for install confirmation but sent no dialog intent".to_string()))]
        );
        assert_eq!(
            received.lock().unwrap().as_slice(),
            &[PackageInstallStatus::Failure {
                session_id: 7,
                status_code: STATUS_FAILURE,
                message: "The system asked for install confirmation but sent no dialog intent".to_string(),
            }]
        );
    }

    #[test]
    fn pending_user_action_surfaces_the_confirmation() {
        let (mut bus, mut installer, received) = bus_with_recorder();
        bus.on_receive(
            &mut installer,
            &InstallBroadcast {
                session_id: 9,
                status: Some(STATUS_PENDING_USER_ACTION),
                status_message: Some("confirm".into()),
                user_action: Some(UserAction {
                    intent_uri: "intent:#Intent;action=INSTALL".into(),
                }),
            },
        );
        // No on_result: the original waits for the dialog's final broadcast.
        assert!(installer.results.borrow().is_empty());
        assert_eq!(
            received.lock().unwrap().as_slice(),
            &[PackageInstallStatus::PendingUserAction {
                session_id: 9,
                confirmation: UserAction {
                    intent_uri: "intent:#Intent;action=INSTALL".into()
                },
            }]
        );
    }

    #[test]
    fn final_statuses_route_to_on_result_and_callbacks() {
        let (mut bus, mut installer, received) = bus_with_recorder();
        bus.on_receive(
            &mut installer,
            &InstallBroadcast {
                session_id: 11,
                status: Some(STATUS_SUCCESS),
                status_message: None,
                user_action: None,
            },
        );
        assert_eq!(
            installer.results.borrow().as_slice(),
            &[(11, STATUS_SUCCESS, Some(String::new()))]
        );
        assert_eq!(
            received.lock().unwrap().as_slice(),
            &[PackageInstallStatus::Success { session_id: 11 }]
        );

        bus.on_receive(
            &mut installer,
            &InstallBroadcast {
                session_id: 12,
                status: Some(5),
                status_message: Some("INSTALL_FAILED_UPDATE_INCOMPATIBLE".into()),
                user_action: None,
            },
        );
        assert_eq!(
            installer.results.borrow().as_slice(),
            &[
                (11, STATUS_SUCCESS, Some(String::new())),
                (12, 5, Some("INSTALL_FAILED_UPDATE_INCOMPATIBLE".into()))
            ]
        );
        assert_eq!(
            received.lock().unwrap()[1],
            PackageInstallStatus::Failure {
                session_id: 12,
                status_code: 5,
                message: "INSTALL_FAILED_UPDATE_INCOMPATIBLE".into(),
            }
        );
    }

    #[test]
    fn user_action_failure_report() {
        let (mut bus, mut installer, _received) = bus_with_recorder();
        bus.report_user_action_failed(&mut installer, 21, "ActivityNotFoundException");
        assert_eq!(
            installer.results.borrow().as_slice(),
            &[(21, STATUS_FAILURE, Some("Could not open the system install dialog: ActivityNotFoundException".to_string()))]
        );
    }
}
