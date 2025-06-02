use anyhow::{Context, Result};
use std::{collections::HashMap, sync::Arc};

use crate::{
    common,
    scripting::{self, RSContext},
};

mod config;
mod handler;
mod prompt;
mod providers;

type UserVM = scripting::UserVM<ctx::VMData, ctx::CtxPart>;

#[derive(clap::Args, Debug)]
pub struct CliArgsRun {
    #[arg(long, default_value_t = String::from("${genvmRoot}/config/genvm-module-llm.yaml"))]
    config: String,

    #[arg(long, default_value_t = false)]
    allow_empty_backends: bool,

    #[arg(long, default_value_t = false)]
    die_with_parent: bool,
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

mod ctx;

fn handle_run(mut config: config::Config, args: CliArgsRun) -> Result<()> {
    for (k, v) in config.backends.iter_mut() {
        if !v.enabled {
            continue;
        }

        v.script_config.models.retain(|_k, v| v.enabled);

        if v.script_config.models.is_empty() {
            log::warn!(backend = k; "models are empty");
            v.enabled = false;
        } else if v.key.is_empty() {
            log::warn!(backend = k; "could not detect key for backend");
            v.enabled = false;
        }
    }

    config.backends.retain(|_k, v| v.enabled);

    if config.backends.is_empty() {
        log::error!("no valid backend detected")
    }

    if !args.allow_empty_backends && config.backends.is_empty() {
        anyhow::bail!("no valid backend detected");
    }

    log::info!(backends:serde = config.backends.keys().collect::<Vec<_>>(); "backends left after filter");

    let runtime = config.base.create_rt()?;

    let token = common::setup_cancels(&runtime, args.die_with_parent)?;

    let config = Arc::new(config);

    let backends = config
        .backends
        .iter()
        .map(|(k, v)| (k.clone(), v.to_provider()))
        .collect();

    let moved_config = config.clone();

    let vm_pool = runtime.block_on(scripting::pool::new(config.vm_count, move || {
        let moved_config = moved_config.clone();
        async {
            let mut user_vm =
                crate::scripting::UserVM::create("", move |vm: mlua::Lua| async move {
                    // set llm-related globals
                    vm.globals()
                        .set("__llm", ctx::create_global(&vm, &moved_config)?)?;

                    // load script
                    scripting::load_script(&vm, &moved_config.lua_script_path).await?;

                    // get functions populated by script
                    let exec_prompt: mlua::Function = vm.globals().get("exec_prompt")?;
                    let exec_prompt_template: mlua::Function =
                        vm.globals().get("exec_prompt_template")?;

                    Ok(ctx::VMData {
                        exec_prompt,
                        exec_prompt_template,
                    })
                })
                .await?;

            user_vm.add_ctx_creator(Box::new(|ctx: &RSContext<ctx::CtxPart>, vm, table| {
                table.set("__ctx_web", vm.create_userdata(ctx.data.clone())?)?;

                Ok(())
            }));

            Ok(user_vm)
        }
    }))?;

    let loop_future = crate::common::run_loop(
        config.bind_address.clone(),
        token,
        Arc::new(handler::Provider {
            vm_pool,
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
        "models": {
            args.model: {}
        },
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
    let provider = backend.to_provider();

    let client = common::create_client()?;

    let res = runtime.block_on(
        provider.exec_prompt_text(
            &client,
            &prompt::Internal {
                system_message: None,
                temperature: 0.7,
                user_message:
                    "Respond with two letters \"ok\" (without quotes) and only this word, lowercase"
                        .to_owned(),
                images: Vec::new(),
                max_tokens: 30,
                use_max_completion_tokens: true,
            },
            backend.script_config.models.first_key_value().unwrap().0,
        ),
    )?;

    let res = res.trim().to_lowercase();

    if res != "ok" {
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
