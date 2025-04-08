use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use clap::Parser;

mod config;
mod domains;
mod handler;

#[derive(clap::Parser)]
#[command(version = genvm_common::VERSION)]
#[clap(rename_all = "kebab_case")]
struct CliArgs {
    #[arg(long, default_value_t = String::from("${genvmRoot}/etc/genvm-module-web.yaml"))]
    config: String,
}

async fn check_status(webdriver_host: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let status_res = client
        .get(format!("{}/status", webdriver_host))
        .header("Content-Type", "application/json; charset=utf-8")
        .send()
        .await
        .with_context(|| "creating sessions request")?;

    let body = genvm_modules_impl_common::read_response(status_res)
        .await
        .with_context(|| "reading response")?;

    let val: serde_json::Value = serde_json::from_str(&body)?;

    if val.pointer("/value/ready").and_then(|v| v.as_bool()) != Some(true) {
        anyhow::bail!("not ready {}", val)
    }

    Ok(())
}

fn main() -> Result<()> {
    let args = CliArgs::parse();

    let config = genvm_common::load_config(HashMap::new(), &args.config)
        .with_context(|| "loading config")?;
    let config: config::Config = serde_yaml::from_value(config)?;

    config.base.setup_logging(std::io::stdout())?;

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

    let webdriver_host = config.webdriver_host.clone();

    let loop_future = genvm_modules_impl_common::run_loop(
        config.bind_address.clone(),
        token,
        Arc::new(handler::HandlerProvider {
            config: Arc::new(config),
        }),
    );

    runtime
        .block_on(check_status(&webdriver_host))
        .with_context(|| "initial health check")?;

    log::info!("health is OK");

    runtime.block_on(loop_future)?;

    std::mem::drop(runtime);

    Ok(())
}
