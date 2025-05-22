use std::{
    backtrace::{self},
    collections::HashMap,
};

use anyhow::Context;
use serde::Deserialize;

pub mod calldata;
pub mod cancellation;
pub mod templater;

pub fn log_error(err: &anyhow::Error) -> impl log::kv::ToValue + '_ {
    use log::kv::{ToValue, Value};

    #[derive(serde::Serialize)]
    struct Error4Log<'a> {
        causes: Vec<Value<'a>>,
        trace: Option<String>,
    }

    impl ToValue for Error4Log<'_> {
        fn to_value(&self) -> Value {
            Value::from_serde(self)
        }
    }

    let mut all = Vec::new();

    for c in err.chain() {
        all.push(Value::from_dyn_error(c));
    }

    let bt = if let backtrace::BacktraceStatus::Captured = err.backtrace().status() {
        Some(err.backtrace().to_string())
    } else {
        None
    };

    Error4Log {
        causes: all,
        trace: bt,
    }
}

#[cfg(not(debug_assertions))]
fn default_log_level() -> log::LevelFilter {
    log::LevelFilter::Info
}

#[cfg(debug_assertions)]
fn default_log_level() -> log::LevelFilter {
    log::LevelFilter::Trace
}

#[derive(Deserialize)]
pub struct BaseConfig {
    pub threads: usize,
    pub blocking_threads: usize,

    #[serde(default = "default_log_level")]
    pub log_level: log::LevelFilter,
    pub log_disable: String,
}

struct NullWiriter;

impl structured_logger::Writer for NullWiriter {
    fn write_log(
        &self,
        _value: &std::collections::BTreeMap<log::kv::Key, log::kv::Value>,
    ) -> std::result::Result<(), std::io::Error> {
        Ok(())
    }
}

pub const VERSION: &str = env!("GENVM_BUILD_ID");

impl BaseConfig {
    pub fn setup_logging<W>(&self, writer: W) -> anyhow::Result<()>
    where
        W: std::io::Write + Sync + Send + 'static,
    {
        structured_logger::Builder::with_level(self.log_level.as_str())
            .with_default_writer(structured_logger::json::new_writer(writer))
            .with_target_writer(&self.log_disable, Box::new(NullWiriter))
            .init();

        if log::STATIC_MAX_LEVEL < log::max_level() {
            log::warn!(requested:? = log::max_level(), allowed:? = log::STATIC_MAX_LEVEL; "requested level is higher than allowed");
        }

        log::info!(version = VERSION; "logging initialized");

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
        std::fs::File::open(&config_path).with_context(|| format!("reading `{}`", config_path))?;
    let value: serde_yaml::Value = serde_yaml::from_reader(file)?;
    let patched = templater::patch_yaml(&vars, value, &templater::DOLLAR_UNFOLDER_RE)?;

    Ok(patched)
}
