use anyhow::{Context, Result};
use clap::Parser;
use std::{collections::HashMap, sync::Arc};

mod config;
mod handler;

#[cfg(not(debug_assertions))]
fn default_log_level() -> log::LevelFilter {
    log::LevelFilter::Info
}

#[cfg(debug_assertions)]
fn default_log_level() -> log::LevelFilter {
    log::LevelFilter::Trace
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

#[derive(clap::Parser)]
#[command(version = concat!(env!("CARGO_PKG_VERSION"), " ", env!("PROFILE"), " ", env!("GENVM_BUILD_ID")))]
#[clap(rename_all = "kebab_case")]
struct CliArgs {
    #[arg(long, default_value_t = default_log_level())]
    log_level: log::LevelFilter,

    #[arg(long, default_value = "tracing*,polling*")]
    log_disable: String,

    #[arg(long, default_value_t = String::from("${genvmRoot}/etc/genvm-module-llm.yaml"))]
    config: String,
}

fn main() -> Result<()> {
    let args = CliArgs::parse();

    structured_logger::Builder::with_level(args.log_level.as_str())
        .with_default_writer(structured_logger::json::new_writer(std::io::stdout()))
        .with_target_writer(&args.log_disable, Box::new(NullWiriter))
        .init();

    let mut root_path = std::env::current_exe().with_context(|| "getting current exe")?;
    root_path.pop();
    root_path.pop();
    let root_path = root_path
        .into_os_string()
        .into_string()
        .map_err(|e| anyhow::anyhow!("can't convert path to string `{e:?}`"))?;

    let vars: HashMap<String, String> = HashMap::from([("genvmRoot".into(), root_path)]);

    let config =
        genvm_common::load_config(&vars, &args.config).with_context(|| "loading config")?;
    let mut config: config::Config = serde_yaml::from_value(config)?;

    for (k, v) in config.backends.iter_mut() {
        if !v.enabled {
            continue;
        }

        if !v.key.is_empty() {
            continue;
        }

        v.key = std::env::var_os(&v.key_env_name)
            .map(|val| val.into_string())
            .unwrap_or(Ok("".into()))
            .map_err(|_e| anyhow::anyhow!("can't convert OsString to String"))?;

        if v.key.is_empty() {
            log::warn!(backend = k;"could not detect key for backend");
            v.enabled = false;
        }
    }

    config.backends.retain(|_k, v| v.enabled);

    if config.backends.is_empty() {
        anyhow::bail!("no valid backend detected");
    }

    let (token, canceller) = genvm_common::cancellation::make();

    let handle_sigterm = move || {
        log::warn!("sigterm received");
        canceller();
    };
    unsafe {
        signal_hook::low_level::register(signal_hook::consts::SIGTERM, handle_sigterm.clone())?;
        signal_hook::low_level::register(signal_hook::consts::SIGINT, handle_sigterm)?;
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .worker_threads(config.threads)
        .max_blocking_threads(config.blocking_threads)
        .build()?;

    let loop_fututre = genvm_modules_impl_common::run_loop(
        config.bind_address.clone(),
        token,
        Arc::new(handler::HandlerProvider {
            config: Arc::new(config),
        }),
    );

    runtime.block_on(loop_fututre)?;

    std::mem::drop(runtime);

    Ok(())
}
