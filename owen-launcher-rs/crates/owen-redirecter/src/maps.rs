//! /proc/self/maps parsing — module bases and extents, pure safe code.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct Module {
    pub base: usize,
    pub end: usize,
    /// End of the executable (r-x) region, absolute address.
    pub exec_end: usize,
}

pub fn read_modules() -> HashMap<String, Module> {
    let mut out: HashMap<String, Module> = HashMap::new();
    let Ok(text) = std::fs::read_to_string("/proc/self/maps") else {
        return out;
    };
    for line in text.lines() {
        // "<start>-<end> <perms> <off> <dev> <inode> <path>"
        let mut parts = line.split_whitespace();
        let Some(range) = parts.next() else { continue };
        let Some(perms) = parts.next() else { continue };
        let _off = parts.next();
        let _dev = parts.next();
        let _ino = parts.next();
        let path = parts.next().unwrap_or("").to_string();
        if path.is_empty() {
            continue;
        }
        // Only the last path component identifies the module.
        let name = match path.rsplit('/').next() {
            Some(n) if !n.is_empty() => n.to_string(),
            _ => continue,
        };
        let Some((start_s, end_s)) = range.split_once('-') else { continue };
        let (Ok(start), Ok(end)) = (usize::from_str_radix(start_s, 16), usize::from_str_radix(end_s, 16))
        else {
            continue;
        };
        let entry = out.entry(name).or_insert(Module {
            base: start,
            end,
            exec_end: 0,
        });
        if start < entry.base {
            entry.base = start;
        }
        if end > entry.end {
            entry.end = end;
        }
        if perms.len() >= 3 && perms.as_bytes()[2] == b'x' && end > entry.exec_end {
            entry.exec_end = end;
        }
    }
    out
}

/// Wait until any of `names` is mapped; returns the first one that appears
/// together with its module info.
pub fn wait_for_module(names: &[&str], timeout_secs: u64) -> Option<(String, Module)> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    loop {
        let maps = read_modules();
        for name in names {
            if let Some(m) = maps.get(*name) {
                return Some(((*name).to_string(), *m));
            }
        }
        if std::time::Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

/// Return module info when already mapped.
pub fn find_module(name: &str) -> Option<Module> {
    read_modules().into_iter().find(|(n, _)| n == name).map(|(_, m)| m)
}
