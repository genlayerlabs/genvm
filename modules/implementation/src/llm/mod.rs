use anyhow::{Context, Result};
use genvm_common::*;
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use crate::{common, scripting};

pub(crate) mod config;
mod handler;
pub(crate) mod merge;
pub(crate) mod prompt;
pub(crate) mod providers;

type LlmSubContext = crate::manager::execution_context::LlmSubContext;
type UserVM = scripting::UserVM<ctx::VMData, sync::DArc<ctx::CtxPart>, LlmSubContext>;

#[derive(serde::Serialize, Debug, Default)]
pub(crate) struct Metrics {
    pub(crate) scripting: scripting::Metrics,
    pub(crate) tokens: stats::metric::TokenMetricsMap,
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

pub(crate) mod ctx;

pub const TEST_PROMPT_FOR_OK: &str = "I am testing that your API works and you are capable for understanding the simplest request. For it I need you to respond with two letters \"ok\" (without quotes) and nothing else. Lowercase, no repetition or punctuation";

async fn create_vm(config: &sync::DArc<config::Config>) -> anyhow::Result<UserVM> {
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

            Ok(ctx::VMData {
                exec_prompt,
                exec_prompt_template,
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

#[cfg(test)]
mod tests {
    use genvm_common::logger;
    use genvm_modules_interfaces::llm::{self as llm_iface};
    use mlua::LuaSerdeExt;
    use std::collections::BTreeMap;
    use tokio::io::AsyncWriteExt;

    use crate::llm::config::ScriptBackendConfig;

    use super::*;

    #[tokio::test]
    async fn test_overloaded() {
        common::tests::setup();

        const BIND_ADDR: &str = "127.0.0.1:11434";
        const CONNECT_ADDR: &str = "http://127.0.0.1:11434";

        let server = tokio::net::TcpListener::bind(BIND_ADDR).await.unwrap();

        let made_request = Arc::new(std::sync::atomic::AtomicBool::new(false));

        let moved_made_request = made_request.clone();

        let server_task = tokio::spawn(async move {
            let (mut client, _) = server.accept().await.unwrap();

            client
                .write_all("HTTP/1.1 503 Service Unavailable\r\n\r\n".as_bytes())
                .await
                .unwrap();

            client.shutdown().await.unwrap();

            moved_made_request.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        let backend_test = config::BackendConfig {
            enabled: true,
            provider: config::Provider::OpenaiCompatible,
            key: "<empty>".to_owned(),
            script_config: ScriptBackendConfig {
                models: BTreeMap::from([(
                    "model".to_owned(),
                    config::ModelConfig {
                        enabled: true,
                        supports_json: true,
                        supports_image: true,
                        use_max_completion_tokens: false,
                        meta: serde_json::Value::Null,
                    },
                )]),
                meta: serde_json::Value::Null,
            },
            host: CONNECT_ADDR.to_owned(),
        };

        let backend_real = config::BackendConfig {
            enabled: true,
            provider: config::Provider::OpenaiCompatible,
            key: std::env::var("OPENAIKEY").unwrap(),
            script_config: ScriptBackendConfig {
                models: BTreeMap::from([(
                    "openrouter/auto".to_owned(),
                    config::ModelConfig {
                        enabled: true,
                        supports_json: true,
                        supports_image: true,
                        use_max_completion_tokens: false,
                        meta: serde_json::Value::Null,
                    },
                )]),
                meta: serde_json::json!({
                    "priority": -10,
                }),
            },
            host: "https://openrouter.ai/api".to_owned(),
        };

        let provider_test = backend_test.to_provider();
        let provider_real = backend_real.to_provider();

        let mut extra_path = std::path::PathBuf::from("../install/lib/genvm-lua")
            .canonicalize()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        extra_path.push_str("/?.lua");

        let config = sync::DArc::new(config::Config {
            base: genvm_common::BaseConfig {
                log_level: logger::Level::Debug,
                threads: 1,
                blocking_threads: 3,
                log_disable: "".to_owned(),
            },
            mod_base: common::ModuleBaseConfig {
                vm_count: 1,
                lua_script_path: "../install/config/genvm-llm-default.lua".to_string(),
                bind_address: None,
                lua_path: extra_path,
                signer_url: Arc::from(""),
                signer_headers: Arc::new(BTreeMap::new()),
            },
            prompt_templates: config::PromptTemplates {
                eq_comparative: serde_json::Value::Null,
                eq_non_comparative_leader: serde_json::Value::Null,
                eq_non_comparative_validator: serde_json::Value::Null,
            },
            backends: BTreeMap::from([
                ("1".to_owned(), backend_test),
                ("2".to_owned(), backend_real),
            ]),
            meta: serde_json::Value::Null,
        });

        let providers = std::sync::Arc::new(BTreeMap::from([
            ("1".to_owned(), provider_test),
            ("2".to_owned(), provider_real),
        ]));

        let user_vm = create_vm(&config).await.unwrap();

        // this ensures order
        user_vm
            .vm
            .load(
                r#"
                    local llm = require("lib-llm")
                    setmetatable(llm.providers, {
                        __pairs = function(t)
                            local keys = {}
                            for k in next,t,nil do
                                table.insert(keys, k)
                            end

                            table.sort(keys)

                            local i = 0
                            return function()
                                i = i + 1
                                local key = keys[i]
                                if key ~= nil then
                                    return key, t[key]
                                end
                            end, t, nil
                        end
                    })
                "#,
            )
            .exec()
            .unwrap();

        let hello = common::tests::get_hello();

        let metrics = sync::DArc::new(Metrics::default());
        let scripting_ctx = scripting::create_ctx_part(
            &hello,
            &config.gep(|x| &x.mod_base),
            metrics.gep(|x| &x.scripting),
        )
        .unwrap();
        let llm_ctx = ctx::CtxPart {
            providers: providers.clone(),
            metrics,
        };
        let sub_ctx = sync::DArc::new(crate::manager::execution_context::LlmSubContext {
            scripting: scripting_ctx,
            module: llm_ctx,
        });

        let (_ctx, ctx_lua) = user_vm.create_ctx(&sub_ctx).unwrap();

        let payload = llm_iface::PromptPayload {
            images: Vec::new(),
            response_format: llm_iface::OutputFormat::Text,
            prompt: TEST_PROMPT_FOR_OK.to_owned(),
        };

        let payload = user_vm
            .vm
            .to_value_with(&payload, scripting::DEFAULT_LUA_SER_OPTIONS)
            .unwrap();
        let fuel = user_vm
            .vm
            .to_value_with(&0u64, scripting::DEFAULT_LUA_SER_OPTIONS)
            .unwrap(); // Mock fuel value

        let res = user_vm
            .call_fn(&user_vm.data.exec_prompt, (ctx_lua, payload, fuel))
            .await
            .unwrap();
        let res: llm_iface::PromptAnswer = user_vm.vm.from_value(res).unwrap();

        match res.data {
            llm_iface::PromptAnswerData::Text(text) => {
                assert_eq!(text.trim().to_lowercase(), "ok");
            }
            _ => panic!("unexpected response format"),
        }

        assert!(made_request.load(std::sync::atomic::Ordering::SeqCst));

        server_task.await.unwrap();
    }
}
