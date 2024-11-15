use anyhow::Result;
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
    std::fs::create_dir_all(&cache_dir)?;

    let test_path = cache_dir.join(".test");
    std::fs::write(test_path, "")?;
    Ok(cache_dir)
}

pub static CACHE_DIR: Lazy<Option<PathBuf>> = Lazy::new(|| match get_cache_dir() {
    Err(e) => {
        eprintln!("can't get cache dir {e:?}");
        None
    }
    Ok(p) => Some(p),
});

pub static PRECOMPILE_DIR: Lazy<Option<(PathBuf, PathBuf)>> =
    Lazy::new(|| match Lazy::force(&CACHE_DIR) {
        None => None,
        Some(dir) => Some((
            dir.join("precompile").join("det").to_path_buf(),
            dir.join("precompile").join("non-det").to_path_buf(),
        )),
    });
