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

pub static PRECOMPILE_DIR: Lazy<Option<PathBuf>> = Lazy::new(|| Lazy::force(&CACHE_DIR).as_ref().map(|dir| dir.join("precompile").to_path_buf()));

pub struct DetNonDetSuffixes {
    pub det: &'static str,
    pub non_det: &'static str,
}

pub const DET_NON_DET_PRECOMPILED_SUFFIX: DetNonDetSuffixes = DetNonDetSuffixes {
    det: "det",
    non_det: "non-det",
};

pub fn path_in_zip_to_hash(path: &str) -> String {
    use sha3::digest::FixedOutput;
    use sha3::{Digest, Sha3_224};

    let mut hasher = Sha3_224::new();
    hasher.update(path.as_bytes());
    let digits = hasher.finalize_fixed();

    let digits = digits.as_slice();

    base32::encode(base32::Alphabet::Rfc4648 { padding: false }, digits)
}

pub fn validate_wasm(engines: &crate::vm::Engines, wasm: &[u8]) -> Result<()> {
    use wasmparser::*;

    // FIXME: find source of this. why call_indirect requires tables?
    let add_features = WasmFeatures::REFERENCE_TYPES.bits() | WasmFeatures::FLOATS.bits();

    let det_features = engines.det.config().get_features().bits() | add_features;

    let non_det_features = engines.non_det.config().get_features().bits() | add_features;

    let mut det_validator =
        wasmparser::Validator::new_with_features(WasmFeatures::from_bits(det_features).unwrap());
    let mut non_det_validator = wasmparser::Validator::new_with_features(
        WasmFeatures::from_bits(non_det_features).unwrap(),
    );
    det_validator.validate_all(wasm).with_context(|| {
        format!(
            "validating {}",
            &String::from_utf8_lossy(&wasm[..10.min(wasm.len())])
        )
    })?;
    non_det_validator.validate_all(wasm).with_context(|| {
        format!(
            "validating {}",
            &String::from_utf8_lossy(&wasm[..10.min(wasm.len())])
        )
    })?;

    Ok(())
}
