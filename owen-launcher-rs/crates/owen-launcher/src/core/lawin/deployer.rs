#![forbid(unsafe_code)]
//! 1:1 port of `core/lawin/LawinDataDeployer.kt`.
//!
//! Deploys the bundled data trees (lawin/dexopt/images/tools/video/Owen +
//! manifest.txt) into the data root. The Kotlin source was the APK `assets`
//! folder; the Rust port takes a `bundled` directory (repo `data/` or a
//! user-provided bundle). Missing trees become empty folders exactly like
//! the original's "Asset folder not bundled — creating empty folder.".

use std::fs;
use std::path::Path;

use crate::core::diag;
use crate::core::{lawin::config as lawin_config, player_store};

const TAG: &str = "LawinDataDeployer";

/// `DEPLOY_ROOTS`
pub const DEPLOY_ROOTS: &[&str] = &["lawin", "dexopt", "images", "tools", "video", "Owen"];
/// `DEPLOY_ROOT_FILES`
pub const DEPLOY_ROOT_FILES: &[&str] = &["manifest.txt"];

/// `LawinDataDeployer.lawinDir(context)`
pub fn lawin_dir(data_root: &Path) -> std::path::PathBuf {
    data_root.join("lawin")
}

/// `deployLocal(context, force)` — returns the `lawinCount > 0 && hasConfig`
/// verdict, logging the same per-root counts.
pub fn deploy_local(data_root: &Path, bundled: &Path, force: bool) -> bool {
    deploy_internal(data_root, bundled, force)
}

/// `ensureDeployedBlocking(context)`
pub fn ensure_deployed_blocking(data_root: &Path, bundled: &Path) -> bool {
    deploy_internal(data_root, bundled, false)
}

fn deploy_internal(data_root: &Path, bundled: &Path, force: bool) -> bool {
    let root = data_root;
    if fs::create_dir_all(root).is_err() {
        diag::e(TAG, &format!("Deploy failed: cannot create {}", root.display()));
        return false;
    }
    // dexopt: folder-for-folder + file-for-file cleanup, like the original
    let dexopt = root.join("dexopt");
    if dexopt.is_file() {
        let _ = fs::remove_file(&dexopt);
    }
    if !dexopt.exists() {
        let _ = fs::create_dir_all(&dexopt);
    }

    for name in DEPLOY_ROOTS {
        let target = root.join(name);
        let config = target.join("Config").join("config.ini");
        let config_valid = config.is_file() && config.metadata().map(|m| m.len()).unwrap_or(0) > 0;
        if *name != "lawin" || force || !config_valid {
            copy_asset_tree(&bundled.join(name), &target, *name == "lawin" || force);
            if *name == "lawin" {
                let player = player_store::get(data_root);
                if !player.is_empty() {
                    lawin_config::apply_display_name(data_root, &player);
                }
            }
        }
    }
    for name in DEPLOY_ROOT_FILES {
        copy_asset_file(&bundled.join(name), &root.join(*name), true);
    }

    let mut parts = Vec::new();
    for name in DEPLOY_ROOTS.iter().copied().chain(["manifest.txt"]) {
        let f = root.join(name);
        let count = count_files(&f);
        parts.push(format!("{name}={count}"));
    }
    let parts = parts.join(" ");
    let lawin = lawin_dir(data_root);
    let lawin_count = count_files(&lawin);
    let has_config = {
        let c = lawin.join("Config").join("config.ini");
        c.is_file() && c.metadata().map(|m| m.len()).unwrap_or(0) > 0
    };
    diag::i(
        TAG,
        &format!(
            "Data deployed at {} [{}] — server dir: {} ({} files, config.ini={})",
            root.display(),
            parts,
            lawin.display(),
            lawin_count,
            has_config
        ),
    );
    if lawin_count < 90 {
        diag::e(TAG, &format!("Lawin data looks incomplete ({lawin_count}/94 files)!"));
    }
    lawin_count > 0 && has_config
}

fn count_files(path: &Path) -> usize {
    if path.is_dir() {
        let mut n = 0;
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                n += count_files(&entry.path());
            }
        }
        n
    } else if path.is_file() {
        1
    } else {
        0
    }
}

/// `copyAssetTree(context, assetPath, target)` — with the same semantics:
/// empty listing → file asset (copy) or create empty dir; children recurse.
fn copy_asset_tree(src: &Path, target: &Path, force: bool) {
    let Ok(children) = fs::read_dir(src) else {
        if src.is_file() {
            if target.is_dir() {
                let _ = fs::remove_dir_all(target);
            }
            copy_asset_file(src, target, true);
        } else {
            if target.is_file() {
                let _ = fs::remove_file(target);
            }
            if !target.exists() {
                let _ = fs::create_dir_all(target);
            }
        }
        return;
    };
    let mut names: Vec<String> = children.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    if names.is_empty() {
        if src.is_file() {
            if target.is_dir() {
                let _ = fs::remove_dir_all(target);
            }
            copy_asset_file(src, target, true);
        } else {
            if target.is_file() {
                let _ = fs::remove_file(target);
            }
            if !target.exists() {
                let _ = fs::create_dir_all(target);
            }
        }
        return;
    }
    if target.is_file() {
        let _ = fs::remove_file(target);
    }
    let _ = fs::create_dir_all(target);
    names.sort();
    for child in names {
        copy_asset_tree(&src.join(&child), &target.join(&child), force);
    }
}

/// `copyAssetFile(context, assetPath, target, force)`
fn copy_asset_file(src: &Path, target: &Path, force: bool) {
    if !force && target.is_file() && target.metadata().map(|m| m.len()).unwrap_or(0) > 0 {
        return;
    }
    if let Err(e) = (|| -> std::io::Result<()> {
        if target.is_dir() {
            fs::remove_dir_all(target)?;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, target)?;
        Ok(())
    })() {
        diag::e(TAG, &format!("Failed to copy asset {}: {e}", src.display()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deploys_lawin_tree_with_config_verdict() {
        let tmp = std::env::temp_dir().join(format!("owen_dep_test_{}", std::process::id()));
        let bundled = tmp.join("bundled");
        let root = tmp.join("root");
        let _ = fs::create_dir_all(bundled.join("lawin").join("Config"));
        let _ = fs::create_dir_all(bundled.join("lawin").join("profiles"));
        fs::write(bundled.join("lawin/Config/config.ini"), "[Config]\nserver=owen\n").unwrap();
        fs::write(bundled.join("lawin/profiles/athena.json"), "{}").unwrap();
        fs::write(bundled.join("manifest.txt"), "versions list").unwrap();

        assert!(deploy_local(&root, &bundled, false));
        assert!(root.join("lawin/Config/config.ini").is_file());
        assert!(root.join("lawin/profiles/athena.json").is_file());
        assert!(root.join("manifest.txt").is_file());
        // second run is a no-op pass (config.ini already valid)
        assert!(deploy_local(&root, &bundled, false));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn missing_bundled_tree_becomes_empty_folder() {
        let tmp = std::env::temp_dir().join(format!("owen_dep_empty_{}", std::process::id()));
        let bundled = tmp.join("bundled");
        let root = tmp.join("root");
        let _ = fs::create_dir_all(&bundled);
        assert!(!deploy_local(&root, &bundled, false));
        assert!(root.join("dexopt").is_dir());
        assert!(root.join("Owen").is_dir());
        let _ = fs::remove_dir_all(&tmp);
    }
}
