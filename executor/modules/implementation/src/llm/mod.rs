use anyhow::{Context, Result};
use genvm_common::*;
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

async fn create_vm(config: &config::Config, extra_path: &str) -> anyhow::Result<UserVM> {
    let mut user_vm =
        crate::scripting::UserVM::create(extra_path, move |vm: mlua::Lua| async move {
            // set llm-related globals
            vm.globals()
                .set("__llm", ctx::create_global(&vm, config)?)?;

            scripting::load_script(&vm, &config.lua_script_path).await?;

            // get functions populated by script
            let exec_prompt: mlua::Function = vm.globals().get("ExecPrompt")?;
            let exec_prompt_template: mlua::Function = vm.globals().get("ExecPromptTemplate")?;

            Ok(ctx::VMData {
                exec_prompt,
                exec_prompt_template,
            })
        })
        .await?;

    user_vm.add_ctx_creator(Box::new(|ctx: &RSContext<ctx::CtxPart>, vm, table| {
        table.set("__ctx_llm", vm.create_userdata(ctx.data.clone())?)?;

        Ok(())
    }));

    Ok(user_vm)
}

fn handle_run(mut config: config::Config, args: CliArgsRun) -> Result<()> {
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

    if !args.allow_empty_backends && config.backends.is_empty() {
        anyhow::bail!("no valid backend detected");
    }

    log_info!(backends:serde = config.backends.keys().collect::<Vec<_>>(); "backends left after filter");

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
        async move {
            create_vm(&moved_config, "")
                .await
                .with_context(|| "creating user VM")
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
            "provider is not functional, answer is `{}` instead of `ok`",
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

        let mut extra_path = std::path::PathBuf::from("scripting/")
            .canonicalize()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        extra_path.push_str("/?.lua");

        let config = Arc::new(config::Config {
            bind_address: "".to_owned(),
            base: genvm_common::BaseConfig {
                log_level: logger::Level::Debug,
                threads: 1,
                blocking_threads: 3,
                log_disable: "".to_owned(),
            },
            vm_count: 1,
            lua_script_path: "scripting/llm-default.lua".to_string(),
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

        let user_vm = create_vm(&config, &extra_path).await.unwrap();

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

        let client = reqwest::Client::new();
        let rs_ctx = scripting::RSContext {
            client: client.clone(),
            data: Arc::new(ctx::CtxPart {
                hello: genvm_modules_interfaces::GenVMHello {
                    cookie: "test_cookie".to_string(),
                    host_data: Arc::new(serde_json::Value::Null),
                },
                providers: Arc::new(BTreeMap::from([
                    ("1".to_owned(), provider_test),
                    ("2".to_owned(), provider_real),
                ])),
                client,
            }),
        };

        let ctx_lua = user_vm.create_ctx(&rs_ctx).unwrap();

        let payload = llm_iface::PromptPayload {
            images: Vec::new(),
            response_format: llm_iface::OutputFormat::Text,
            prompt: "respond with two letters \"ok\" (without quotes) and nothing else. Lowercase, no repetition or punctuation".to_owned(),
        };

        let payload = user_vm.vm.to_value(&payload).unwrap();

        let res = user_vm
            .call_fn(&user_vm.data.exec_prompt, (ctx_lua, payload))
            .await
            .unwrap();
        let res: llm_iface::PromptAnswer = user_vm.vm.from_value(res).unwrap();

        match res {
            llm_iface::PromptAnswer::Text(text) => {
                assert_eq!(text.trim().to_lowercase(), "ok");
            }
            _ => panic!("unexpected response format"),
        }

        assert!(made_request.load(std::sync::atomic::Ordering::SeqCst));

        server_task.await.unwrap();
    }
}
