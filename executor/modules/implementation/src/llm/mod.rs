use anyhow::{Context, Result};
use std::{collections::HashMap, sync::Arc};

mod config;
mod handler;
mod prompt;
mod providers;
mod scripting;

#[derive(clap::Args, Debug)]
pub struct CliArgsRun {
    #[arg(long, default_value_t = String::from("${genvmRoot}/config/genvm-module-llm.yaml"))]
    config: String,

    #[arg(long, default_value_t = false)]
    allow_empty_backends: bool,
}

#[derive(clap::Args, Debug)]
pub struct CliArgsCheck {
    #[arg(long, default_value_t = String::from("${genvmRoot}/config/genvm-module-llm.yaml"))]
    config: String,
    #[arg(long, help = "url")]
    host: String,
    #[arg(long)]
    model: String,
    #[arg(long)]
    provider: config::Provider,
    #[arg(long, help = "api key, supports `${ENV[...]}` syntax")]
    key: String,
}

fn handle_run(mut config: config::Config, args: CliArgsRun) -> Result<()> {
    for (k, v) in config.backends.iter_mut() {
        if !v.enabled {
            continue;
        }

        if v.script_config.models.is_empty() {
            log::warn!(backend = k; "models are empty");
            v.enabled = false;
        } else if v.key.is_empty() {
            log::warn!(backend = k; "could not detect key for backend");
            v.enabled = false;
        }
    }

    config.backends.retain(|_k, v| v.enabled);

    if !args.allow_empty_backends && config.backends.is_empty() {
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

    let client = reqwest::Client::new();

    let backends = config
        .backends
        .iter()
        .map(|(k, v)| (k.clone(), v.to_provider(client.clone())))
        .collect();

    let loop_future = crate::common::run_loop(
        config.bind_address.clone(),
        token,
        Arc::new(handler::HandlerProvider {
            config,
            user_vm,
            providers: Arc::new(backends),
        }),
    );

    runtime.block_on(loop_future)?;

    std::mem::drop(runtime);

    Ok(())
}

fn handle_check(config: config::Config, args: CliArgsCheck) -> Result<()> {
    let _ = config;

    let runtime = tokio::runtime::Runtime::new()?;

    let backend = serde_json::json!({
        "host": args.host,
        "provider": args.provider,
        "models": [args.model],
        "key": args.key
    });

    let mut vars = HashMap::new();
    for (mut name, value) in std::env::vars() {
        name.insert_str(0, "ENV[");
        name.push(']');

        vars.insert(name, value);
    }

    let backend = genvm_common::templater::patch_json(
        &vars,
        backend,
        &genvm_common::templater::DOLLAR_UNFOLDER_RE,
    )?;

    let backend: config::BackendConfig = serde_json::from_value(backend)?;
    let provider = backend.to_provider(reqwest::Client::new());

    let res = runtime.block_on(provider
        .exec_prompt_text(
            &prompt::Internal {
                system_message: None,
                temperature: 0.7,
                user_message: "Respond with a single word \"yes\" (without quotes) and only this word, lowercase".to_owned(),
            },
            &backend.script_config.models[0],
        ))?;

    let res = res.trim().to_lowercase();

    if res != "yes" {
        anyhow::bail!(
            "provider is not functional, answer is `{}` instead of `yes`",
            res
        );
    }

    Ok(())
}

pub fn entrypoint_run(args: CliArgsRun) -> Result<()> {
    let config = genvm_common::load_config(HashMap::new(), &args.config)
        .with_context(|| "loading config")?;
    let config: config::Config = serde_yaml::from_value(config)?;

    config.base.setup_logging(std::io::stdout())?;

    handle_run(config, args)
}

pub fn entrypoint_check(args: CliArgsCheck) -> Result<()> {
    let config = genvm_common::load_config(HashMap::new(), &args.config)
        .with_context(|| "loading config")?;
    let config: config::Config = serde_yaml::from_value(config)?;

    config.base.setup_logging(std::io::stdout())?;

    handle_check(config, args)
}
