use anyhow::{Context, Result};
use genvm_common::*;
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use crate::{common, scripting};

pub mod config;
mod handler;
pub mod merge;
pub mod prompt;
pub mod providers;

type LlmSubContext = crate::manager::execution_context::LlmSubContext;
type UserVM = scripting::UserVM<ctx::VMData, sync::DArc<ctx::CtxPart>, LlmSubContext>;

#[derive(serde::Serialize, Debug, Default)]
pub struct Metrics {
    pub scripting: scripting::Metrics,
    pub tokens: stats::metric::TokenMetricsMap,
}

impl<W: calldata::Writer> calldata::codec::Encode<W> for Metrics {
    type Error = W::Error;

    fn encode(&self, enc: &mut calldata::Encoder<W>) -> std::result::Result<(), Self::Error> {
        enc.start_map(2)?;
        enc.push_map_k("scripting")?;
        calldata::codec::Encode::encode(&self.scripting, enc)?;
        enc.push_map_k("tokens")?;
        calldata::codec::Encode::encode(&self.tokens, enc)?;
        Ok(())
    }
}

#[derive(clap::Args, Debug)]
pub struct CliArgsRun {
    #[arg(long, default_value_t = String::from("${exeDir}/../config/genvm-module-llm.yaml"))]
    config: String,

    #[arg(long, default_value_t = false)]
    allow_empty_backends: bool,

    #[arg(long, default_value_t = false)]
    die_with_parent: bool,
}

pub mod ctx;

pub const TEST_PROMPT_FOR_OK: &str = "I am testing that your API works and you are capable for understanding the simplest request. For it I need you to respond with two letters \"ok\" (without quotes) and nothing else. Lowercase, no repetition or punctuation";

pub async fn create_vm(config: &sync::DArc<config::Config>) -> anyhow::Result<UserVM> {
    let user_vm = crate::scripting::UserVM::create(
        &config.mod_base,
        move |vm: mlua::Lua| async move {
            // set llm-related globals
            vm.globals()
                .set("__llm", ctx::create_global(&vm, config)?)?;

            scripting::load_script(&vm, &config.mod_base.lua_script_path)
                .await
                .with_context(|| {
                    format!("loading script from {}", &config.mod_base.lua_script_path)
                })?;

            // get functions populated by script
            let exec_prompt: mlua::Function = vm.globals().get("ExecPrompt")?;
            let exec_prompt_template: mlua::Function = vm.globals().get("ExecPromptTemplate")?;
            let setup: Option<mlua::Function> = vm.globals().get("Setup").ok();
            let teardown: Option<mlua::Function> = vm.globals().get("Teardown").ok();

            Ok(ctx::VMData {
                exec_prompt,
                exec_prompt_template,
                setup,
                teardown,
            })
        },
        Box::new(move |vm, table, sub_ctx: &sync::DArc<LlmSubContext>| {
            let scripting = sub_ctx.gep(|x| &x.scripting);
            let module = sub_ctx.gep(|x| &x.module);
            scripting::setup_lua_default_ctx(scripting, vm, table)?;
            table.set(
                "__ctx_llm",
                vm.create_userdata(scripting::LuaDArc(module.clone()))?,
            )?;
            Ok(module)
        }),
    )
    .await?;

    Ok(user_vm)
}

/// Creates the LLM module and returns the stream handler.
/// The returned future runs the bind loop if bind_address is Some.
pub async fn create_llm_module(
    cancel: Arc<cancellation::Token>,
    mut config: config::Config,
    allow_empty_backends: bool,
) -> Result<(
    crate::manager::modules::StreamHandler,
    impl std::future::Future<Output = Result<()>>,
    sync::DArc<config::Config>,
    Arc<BTreeMap<String, Box<dyn providers::Provider + Send + Sync>>>,
)> {
    for (k, v) in config.backends.iter_mut() {
        if !v.enabled {
            continue;
        }

        v.script_config.models.retain(|_k, v| v.enabled);

        if v.script_config.models.is_empty() {
            log_warn!(backend = k; "models are empty");
            v.enabled = false;
        } else if v.key.is_empty() {
            log_warn!(backend = k; "could not detect key for backend");
            v.enabled = false;
        }
    }

    config.backends.retain(|_k, v| v.enabled);

    if config.backends.is_empty() {
        log_error!("no valid backend detected")
    }

    if !allow_empty_backends && config.backends.is_empty() {
        anyhow::bail!("no valid backend detected");
    }

    let config = sync::DArc::new(config);

    log_info!(backends:serde = config.backends.keys().collect::<Vec<_>>(); "backends left after filter");

    let backends: BTreeMap<_, _> = config
        .backends
        .iter()
        .map(|(k, v)| (k.clone(), v.to_provider()))
        .collect();

    let backends = Arc::new(backends);

    let moved_config = config.clone();

    let vm_pool = scripting::pool::new(config.mod_base.vm_count, move || {
        let moved_config = moved_config.clone();
        async move {
            create_vm(&moved_config)
                .await
                .with_context(|| "creating user VM")
        }
    })
    .await?;

    let handler_provider = Arc::new(handler::Provider {
        vm_pool,
        config: config.clone(),
        providers: backends.clone(),
    });

    // Create the type-erased stream handler
    let stream_handler: crate::manager::modules::StreamHandler = {
        let hp = handler_provider.clone();
        Arc::new(move |stream: Box<dyn genvm_common::io::Stream>, exec_ctx| {
            let hp = hp.clone();
            Box::pin(async move {
                let sub_ctx = exec_ctx.map(|ctx| ctx.gep(|x| x.llm.as_ref().unwrap()));
                crate::common::handle_stream(hp, stream, "relay", sub_ctx).await;
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        })
    };

    let bind_future = crate::common::run_loop(
        config.mod_base.bind_address.clone(),
        cancel,
        handler_provider,
    );

    Ok((stream_handler, bind_future, config, backends))
}

pub async fn run_llm_module(
    cancel: Arc<cancellation::Token>,
    config: config::Config,
    allow_empty_backends: bool,
) -> Result<()> {
    let (_handler, bind_future, _config, _providers) =
        create_llm_module(cancel, config, allow_empty_backends).await?;
    bind_future.await
}

fn handle_run(config: config::Config, args: CliArgsRun) -> Result<()> {
    let runtime = config.base.create_rt()?;

    let token = common::setup_cancels(&runtime, args.die_with_parent)?;

    runtime.block_on(run_llm_module(token, config, args.allow_empty_backends))?;

    std::mem::drop(runtime);

    Ok(())
}

pub fn entrypoint_run(args: CliArgsRun) -> Result<()> {
    let config = genvm_common::load_config(HashMap::new(), &args.config)
        .with_context(|| "loading config")?;
    let config: config::Config = serde_yaml::from_value(config)?;

    config.base.setup_logging(std::io::stdout())?;

    handle_run(config, args)
}
