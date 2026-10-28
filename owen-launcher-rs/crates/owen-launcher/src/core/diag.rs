#![forbid(unsafe_code)]
//! 1:1 port of `app/src/main/kotlin/com/abdallhrgx/owenlauncher/core/diag/DiagLog.kt`.
//!
//! Circular in-memory buffer (600 entries) + append-only `owen_log.txt`
//! (truncated at 512 KB down to the last 180 000 chars) + panic hook
//! (the port of `installCrashHandler`).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_ENTRIES: usize = 600;
const MAX_FILE_CHARS: usize = 180_000;
const TRUNCATE_THRESHOLD: u64 = 512_000;

/// Port of `DiagLog.Entry`.
#[derive(Debug, Clone)]
pub struct Entry {
    pub time: u64, // epoch millis
    pub level: char,
    pub tag: String,
    pub message: String,
}

impl Entry {
    /// `Entry.format()` — "HH:mm:ss.SSS L/TAG: message".
    pub fn format(&self) -> String {
        format!(
            "{} {}/{}: {}",
            fmt_hms_millis(self.time),
            self.level,
            self.tag,
            self.message
        )
    }
}

/// "HH:mm:ss.SSS" from epoch millis (the `SimpleDateFormat("HH:mm:ss.SSS")` port).
pub fn fmt_hms_millis(epoch_ms: u64) -> String {
    let secs_of_day = (epoch_ms / 1000) % 86_400;
    let ms = epoch_ms % 1000;
    let h = secs_of_day / 3600;
    let m = (secs_of_day % 3600) / 60;
    let s = secs_of_day % 60;
    format!("{:02}:{:02}:{:02}.{:03}", h, m, s, ms)
}

fn now_ms() -> u64 {
    SystemTime::duration_since(UNIX_EPOCH, SystemTime::now())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

struct State {
    buffer: std::collections::VecDeque<Entry>,
    log_file: Option<PathBuf>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

/// `DiagLog.init(context)` — the Rust port takes the data dir directly.
pub fn init(dir: &Path) {
    let mut guard = STATE.lock().unwrap();
    let file = dir.join("owen_log.txt");
    if file.is_dir() {
        let _ = fs::remove_dir_all(&file);
    }
    if file.is_file() {
        if let Ok(len) = file.metadata().map(|m| m.len()) {
            if len > TRUNCATE_THRESHOLD {
                if let Ok(content) = fs::read_to_string(&file) {
                    let tail: String = content
                        .chars()
                        .skip(content.chars().count().saturating_sub(MAX_FILE_CHARS))
                        .collect();
                    let _ = fs::write(&file, tail);
                }
            }
        }
    }
    *guard = Some(State {
        buffer: std::collections::VecDeque::with_capacity(MAX_ENTRIES),
        log_file: Some(file),
    });
}

/// `DiagLog.i`
pub fn i(tag: &str, msg: &str) {
    add('I', tag, msg);
}

/// `DiagLog.e`
pub fn e(tag: &str, msg: &str) {
    add('E', tag, msg);
}

/// `DiagLog.e(tag, msg, tr)` — the two-line variant. The Rust port logs the
/// error twice: the caller's message, then the error itself (the Java stack
/// trace has no safe-Rust equivalent).
pub fn e_err(tag: &str, msg: &str, err: &dyn std::error::Error) {
    add('E', tag, msg);
    add('E', tag, &format!("{err}"));
}

/// `DiagLog.section`
pub fn section(title: &str) {
    add('I', "Owen", &format!("==== {} ====", title));
}

/// `DiagLog.add`
pub fn add(level: char, tag: &str, msg: &str) {
    let entry = Entry {
        time: now_ms(),
        level,
        tag: tag.to_string(),
        message: msg.to_string(),
    };
    let file = {
        let mut guard = STATE.lock().unwrap();
        let Some(state) = guard.as_mut() else { return };
        if state.buffer.len() >= MAX_ENTRIES {
            state.buffer.pop_front();
        }
        state.buffer.push_back(entry.clone());
        state.log_file.clone()
    };
    if let Some(file) = file {
        if let Some(parent) = file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        use std::io::Write;
        if let Ok(mut out) = fs::OpenOptions::new().create(true).append(true).open(&file) {
            let _ = writeln!(out, "{}", entry.format());
        }
    }
}

/// `DiagLog.snapshot`
pub fn snapshot() -> Vec<Entry> {
    STATE
        .lock()
        .unwrap()
        .as_ref()
        .map(|s| s.buffer.iter().cloned().collect())
        .unwrap_or_default()
}

/// `DiagLog.tail(lines)`
pub fn tail(lines: usize) -> Vec<Entry> {
    let guard = STATE.lock().unwrap();
    guard
        .as_ref()
        .map(|s| s.buffer.iter().rev().take(lines).rev().cloned().collect())
        .unwrap_or_default()
}

/// `DiagLog.tailText(lines)`
pub fn tail_text(lines: usize) -> String {
    tail(lines)
        .iter()
        .map(|e| e.format())
        .collect::<Vec<_>>()
        .join("\n")
}

/// `DiagLog.filePath`
pub fn file_path() -> Option<PathBuf> {
    STATE.lock().unwrap().as_ref().and_then(|s| s.log_file.clone())
}

/// `DiagLog.clear`
pub fn clear() {
    let mut guard = STATE.lock().unwrap();
    if let Some(state) = guard.as_mut() {
        state.buffer.clear();
        if let Some(file) = state.log_file.as_ref() {
            let _ = fs::remove_dir_all(file);
        }
    }
}

/// `DiagLog.installCrashHandler` — `std::panic::set_hook` port.
pub fn install_crash_handler() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "panic".to_string());
        add('E', "CRASH", &format!("Uncaught panic: {}", msg));
        if let Some(loc) = info.location() {
            add(
                'E',
                "CRASH",
                &format!("    at {}:{}:{}", loc.file(), loc.line(), loc.column()),
            );
        }
        let file = STATE
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|s| s.log_file.clone());
        if let Some(file) = file {
            if file.is_file() {
                if let Ok(content) = fs::read_to_string(&file) {
                    let tail: String = content
                        .chars()
                        .skip(content.chars().count().saturating_sub(MAX_FILE_CHARS))
                        .collect();
                    let _ = fs::write(&file, tail);
                }
            }
        }
        default(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_hms() {
        // 2025-01-01T00:00:00.123Z
        assert_eq!(fmt_hms_millis(1_735_689_600_123), "00:00:00.123");
        assert_eq!(fmt_hms_millis(1_735_689_600_000 + 3_723_456), "01:02:03.456");
    }

    #[test]
    fn tail_and_text() {
        let tmp = std::env::temp_dir().join(format!("owen_diag_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        init(&tmp);
        i("T", "hello");
        e("T", "boom");
        section("Owen Launcher start");
        let text = tail_text(3);
        assert!(text.contains("I/T: hello"));
        assert!(text.contains("E/T: boom"));
        assert!(text.contains("==== Owen Launcher start ===="));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
