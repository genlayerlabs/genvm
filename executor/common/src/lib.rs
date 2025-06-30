use std::collections::HashMap;

use anyhow::Context;
use serde::{Deserialize, Serialize};

pub mod calldata;
pub mod cancellation;
pub mod logger;
pub mod templater;
pub mod version;

#[cfg(not(debug_assertions))]
fn default_log_level() -> logger::Level {
    logger::Level::Info
}

#[cfg(debug_assertions)]
fn default_log_level() -> logger::Level {
    logger::Level::Trace
}

#[derive(Serialize, Deserialize)]
pub struct BaseConfig {
    pub threads: usize,
    pub blocking_threads: usize,

    #[serde(default = "default_log_level")]
    pub log_level: logger::Level,
    pub log_disable: String,
}

pub const VERSION: &str = env!("GENVM_BUILD_ID");

impl BaseConfig {
    pub fn setup_logging<W>(&self, writer: W) -> anyhow::Result<()>
    where
        W: std::io::Write + Sync + Send + 'static,
    {
        logger::initialize(self.log_level, &self.log_disable, writer);

        //structured_logger::Builder::with_level(self.log_level.as_str())
        //    .with_default_writer(structured_logger::json::new_writer(writer))
        //    .with_target_writer(&self.log_disable, Box::new(NullWiriter))
        //    .init();

        if logger::STATIC_MIN_LEVEL > self.log_level {
            log_warn!(requested:? = self.log_level, allowed:? = logger::STATIC_MIN_LEVEL; "requested level is higher than allowed");
        }

        log_info!(version = VERSION; "logging initialized");

        Ok(())
    }

    pub fn create_rt(&self) -> anyhow::Result<tokio::runtime::Runtime> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_io()
            .enable_time()
            .worker_threads(self.threads)
            .max_blocking_threads(self.blocking_threads)
            .build()?;

        Ok(rt)
    }
}

pub fn load_config(
    mut vars: HashMap<String, String>,
    path: &str,
) -> anyhow::Result<serde_yaml::Value> {
    let mut root_path = std::env::current_exe().with_context(|| "getting current exe")?;
    root_path.pop();
    root_path.pop();
    let root_path = root_path
        .into_os_string()
        .into_string()
        .map_err(|e| anyhow::anyhow!("can't convert path to string `{e:?}`"))?;

    vars.insert("genvmRoot".to_owned(), root_path);
    vars.insert("genvmVersion".to_owned(), VERSION.to_owned());

    for (mut name, value) in std::env::vars() {
        name.insert_str(0, "ENV[");
        name.push(']');

        vars.insert(name, value);
    }

    let config_path = templater::patch_str(&vars, path, &templater::DOLLAR_UNFOLDER_RE)?;

    let file =
        std::fs::File::open(&config_path).with_context(|| format!("reading `{config_path}`"))?;
    let value: serde_yaml::Value = serde_yaml::from_reader(file)?;
    let patched = templater::patch_yaml(&vars, value, &templater::DOLLAR_UNFOLDER_RE)?;

    Ok(patched)
}
