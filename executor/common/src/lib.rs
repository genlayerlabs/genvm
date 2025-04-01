use std::collections::HashMap;

use anyhow::Context;

pub mod templater;

pub fn load_config(
    vars: &HashMap<String, String>,
    path: &str,
) -> anyhow::Result<serde_yaml::Value> {
    let config_path = templater::patch_str(vars, path, &templater::DOLLAR_UNFOLDER_RE)?;

    let file =
        std::fs::File::open(&config_path).with_context(|| format!("reading `{}`", config_path))?;
    let value: serde_yaml::Value = serde_yaml::from_reader(file)?;
    let patched = templater::patch_yaml(vars, value, &templater::DOLLAR_UNFOLDER_RE)?;

    Ok(patched)
}
