use anyhow::{Context, Result};
use genvm_common::*;
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use crate::{common, scripting};

pub(crate) mod config;
mod handler;
pub(crate) mod prompt;
mod providers;

type UserVM = scripting::UserVM<ctx::VMData, Arc<ctx::CtxPart>>;

#[derive(serde::Serialize, Debug, Default)]
struct Metrics {
    pub scripting: scripting::Metrics,
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

mod ctx;

async fn create_vm(
    config: &sync::DArc<config::Config>,
    providers: Arc<BTreeMap<String, Box<dyn providers::Provider + Send + Sync>>>,
) -> anyhow::Result<UserVM> {
    let moved_config = config.clone();
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
        Box::new(move |vm, table, hello| {
            let metrics = sync::DArc::new(Metrics::default());

            let dflt_ctx = scripting::create_default_ctx(
                hello,
                moved_config.gep(|x| &x.mod_base),
                metrics.gep(|x| &x.scripting),
                vm,
                table,
            )?;

            //for

            let ctx = Arc::new(ctx::CtxPart {
                dflt: dflt_ctx.clone(),
                providers: providers.clone(),
                metrics: metrics.clone(),
            });

            table.set("__ctx_llm", vm.create_userdata(ctx.clone())?)?;

            Ok(ctx)
        }),
    )
    .await?;

    Ok(user_vm)
}

pub async fn run_llm_module(
    cancel: Arc<cancellation::Token>,
    mut config: config::Config,
    allow_empty_backends: bool,
) -> Result<()> {
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
        let backends = backends.clone();
        async move {
            create_vm(&moved_config, backends)
                .await
                .with_context(|| "creating user VM")
        }
    })
    .await?;

    crate::common::run_loop(
        config.mod_base.bind_address.clone(),
        cancel,
        Arc::new(handler::Provider { vm_pool }),
    )
    .await
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
            },
            host: CONNECT_ADDR.to_owned(),
        };

        let backend_real = config::BackendConfig {
            enabled: true,
            provider: config::Provider::OpenaiCompatible,
            key: std::env::var("OPENAIKEY").unwrap(),
            script_config: ScriptBackendConfig {
                models: BTreeMap::from([(
                    "gpt-4o".to_owned(),
                    config::ModelConfig {
                        enabled: true,
                        supports_json: true,
                        supports_image: true,
                        use_max_completion_tokens: false,
                        meta: serde_json::Value::Null,
                    },
                )]),
            },
            host: "https://api.openai.com".to_owned(),
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
                bind_address: "".to_owned(),
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
        });

        let providers = std::sync::Arc::new(BTreeMap::from([
            ("1".to_owned(), provider_test),
            ("2".to_owned(), provider_real),
        ]));

        let user_vm = create_vm(&config, providers).await.unwrap();

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

        let (_ctx, ctx_lua) = user_vm.create_ctx(&hello).unwrap();

        let payload = llm_iface::PromptPayload {
            images: Vec::new(),
            response_format: llm_iface::OutputFormat::Text,
            prompt: "respond with two letters \"ok\" (without quotes) and nothing else. Lowercase, no repetition or punctuation".to_owned(),
        };

        let payload = user_vm.vm.to_value(&payload).unwrap();
        let fuel = user_vm.vm.to_value(&0u64).unwrap(); // Mock fuel value

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
