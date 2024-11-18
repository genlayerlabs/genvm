use anyhow::{Context, Result};
use std::path::PathBuf;

use once_cell::sync::Lazy;

const GENVM_BUILD_ID: &str = env!("GENVM_BUILD_ID");

/// tries to get cache directory
fn get_cache_dir() -> Result<PathBuf> {
    let cache_dir = match directories_next::ProjectDirs::from("", "yagerai", "genvm") {
        None => {
            anyhow::bail!("can't determine platform cache directory")
        }
        Some(dirs) => dirs.cache_dir().join(GENVM_BUILD_ID),
    };
    std::fs::create_dir_all(&cache_dir).with_context(|| "creating cache dir")?;

    let test_path = cache_dir.join(".test");
    std::fs::write(test_path, "").with_context(|| "creating test file")?;
    Ok(cache_dir)
}

pub static CACHE_DIR: Lazy<Option<PathBuf>> = Lazy::new(|| match get_cache_dir() {
    Err(e) => {
        log::warn!(target: "cache", err:? = e; "can't get cache dir");
        None
    }
    Ok(p) => Some(p),
});

pub static PRECOMPILE_DIR: Lazy<Option<PathBuf>> = Lazy::new(|| match Lazy::force(&CACHE_DIR) {
    None => None,
    Some(dir) => Some(dir.join("precompile").to_path_buf()),
});

pub struct DetNonDetSuffixes {
    pub det: &'static str,
    pub non_det: &'static str,
}

pub const DET_NON_DET_PRECOMPILED_SUFFIX: DetNonDetSuffixes = DetNonDetSuffixes {
    det: "-det",
    non_det: "-non-det",
};
