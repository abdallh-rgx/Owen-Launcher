//! `/proc/self/maps` parsing — which modules are loaded and where.
//!
//! Pure safe Rust: read the file, split lines, no raw memory access.

use std::fs;

#[derive(Debug, Clone)]
pub struct MapRegion {
    pub start: u64,
    pub end: u64,
    pub perms: String,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub path: String,
    /// Lowest mapping start == load base.
    pub base: u64,
    pub size: u64,
    /// First executable region.
    pub text_start: u64,
    pub text_size: u64,
}

pub fn read_maps() -> Vec<MapRegion> {
    let text = fs::read_to_string("/proc/self/maps").unwrap_or_default();
    let mut out = Vec::new();
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        let Some(range) = parts.next() else { continue };
        let Some(perms) = parts.next() else { continue };
        let mut offsets = range.splitn(2, '-');
        let (Some(s), Some(e)) = (offsets.next(), offsets.next()) else {
            continue;
        };
        let (Ok(start), Ok(end)) = (u64::from_str_radix(s, 16), u64::from_str_radix(e, 16)) else {
            continue;
        };
        let path = parts
            .skip(3)
            .next()
            .unwrap_or("")
            .to_string();
        out.push(MapRegion {
            start,
            end,
            perms: perms.to_string(),
            path,
        });
    }
    out
}

/// Find a loaded module by library file name (e.g. `libUnreal.so`).
pub fn find_module(name: &str) -> Option<ModuleInfo> {
    module_info(&format!("/{name}"))
}

/// Aggregate mappings whose path contains `needle`.
pub fn module_info(needle: &str) -> Option<ModuleInfo> {
    let maps = read_maps();
    let mut min_start = u64::MAX;
    let mut max_end = 0u64;
    let mut text_start = 0u64;
    let mut text_size = 0u64;
    let mut path = String::new();
    let mut found = false;
    for r in &maps {
        if r.path.contains(needle) {
            found = true;
            if path.is_empty() {
                path = r.path.clone();
            }
            min_start = min_start.min(r.start);
            max_end = max_end.max(r.end);
            if r.perms.starts_with('r') && r.perms.contains('x') && text_start == 0 {
                text_start = r.start;
                text_size = r.end - r.start;
            }
        }
    }
    if !found {
        return None;
    }
    Some(ModuleInfo {
        path,
        base: min_start,
        size: max_end - min_start,
        text_start,
        text_size,
    })
}

/// All loaded modules (unique paths).
pub fn loaded_modules() -> Vec<ModuleInfo> {
    let maps = read_maps();
    let mut names: Vec<String> = maps
        .iter()
        .filter(|r| !r.path.is_empty() && r.path.contains(".so"))
        .map(|r| r.path.clone())
        .collect();
    names.dedup();
    names
        .iter()
        .filter_map(|n| module_info(&n.clone()))
        .collect()
}
