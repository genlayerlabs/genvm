use anyhow::{Context, Result};
use std::path::PathBuf;

/// tries to get cache directory
pub fn get_cache_dir(base_path: &str) -> Result<PathBuf> {
    let base_path = std::path::Path::new(base_path);

    std::fs::create_dir_all(base_path).with_context(|| "creating cache dir")?;

    let test_path = base_path.join(".test");
    std::fs::write(test_path, "").with_context(|| "creating test file")?;
    Ok(base_path.to_owned())
}

pub struct DetNonDetSuffixes {
    pub det: &'static str,
    pub non_det: &'static str,
}

pub const PRECOMPILE_DIR_NAME: &str = "pc";

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
