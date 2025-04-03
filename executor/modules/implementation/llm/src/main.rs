use anyhow::{Context, Result};
use clap::Parser;
use std::{collections::HashMap, sync::Arc};

mod config;
mod handler;
mod scripting;

#[derive(clap::Parser)]
#[command(version = genvm_common::VERSION)]
#[clap(rename_all = "kebab_case")]
struct CliArgs {
    #[arg(long, default_value_t = String::from("${genvmRoot}/etc/genvm-module-llm.yaml"))]
    config: String,
}

fn main() -> Result<()> {
    let args = CliArgs::parse();

    let config = genvm_common::load_config(HashMap::new(), &args.config)
        .with_context(|| "loading config")?;
    let mut config: config::Config = serde_yaml::from_value(config)?;

    config.base.setup_logging(std::io::stdout())?;

    for (k, v) in config.backends.iter_mut() {
        if !v.enabled {
            continue;
        }

        if v.key.is_empty() {
            log::warn!(backend = k; "could not detect key for backend");
            v.enabled = false;
        }
    }

    config.backends.retain(|_k, v| v.enabled);

    if config.backends.is_empty() {
        anyhow::bail!("no valid backend detected");
    }

    log::info!(backends:serde = config.backends.keys().collect::<Vec<_>>(); "backends left after filter");

    let runtime = config.base.create_rt()?;

    let (token, canceller) = genvm_common::cancellation::make();

    let handle_sigterm = move || {
        log::warn!("sigterm received");
        canceller();
    };
    unsafe {
        signal_hook::low_level::register(signal_hook::consts::SIGTERM, handle_sigterm.clone())?;
        signal_hook::low_level::register(signal_hook::consts::SIGINT, handle_sigterm)?;
    }

    let config = Arc::new(config);

    let user_vm = scripting::UserVM::new(&config)?;

    let loop_future = genvm_modules_impl_common::run_loop(
        config.bind_address.clone(),
        token,
        Arc::new(handler::HandlerProvider { config, user_vm }),
    );

    runtime.block_on(loop_future)?;

    std::mem::drop(runtime);

    Ok(())
}
