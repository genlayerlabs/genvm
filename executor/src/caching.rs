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
