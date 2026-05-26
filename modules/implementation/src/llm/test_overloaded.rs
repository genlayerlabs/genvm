use genvm_common::logger;
use genvm_common::*;
use genvm_modules_interfaces::llm::{self as llm_iface};
use mlua::LuaSerdeExt;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

use super::config::ScriptBackendConfig;
use super::*;
use crate::common;
use crate::scripting;

#[tokio::test]
async fn test_overloaded() {
    common::tests::setup();

    let server = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let connect_addr = format!("http://{}", server.local_addr().unwrap());

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
                    timeout: None,
                },
            )]),
            meta: serde_json::Value::Null,
            timeout: None,
        },
        host: connect_addr.clone(),
    };

    let backend_real = config::BackendConfig {
        enabled: true,
        provider: config::Provider::OpenaiCompatible,
        key: match std::env::var("OPENAIKEY") {
            Ok(v) => v,
            Err(_) => {
                eprintln!("skipping test_overloaded: OPENAIKEY is not set");
                return;
            }
        },
        script_config: ScriptBackendConfig {
            models: BTreeMap::from([(
                "openrouter/auto".to_owned(),
                config::ModelConfig {
                    enabled: true,
                    supports_json: true,
                    supports_image: true,
                    use_max_completion_tokens: false,
                    meta: serde_json::Value::Null,
                    timeout: None,
                },
            )]),
            meta: serde_json::json!({
                "priority": -10,
            }),
            timeout: None,
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
        timeout: None,
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
        .create_userdata(scripting::rat::LuaRat(num_rational::BigRational::from(
            num_bigint::BigInt::from(0),
        )))
        .unwrap();

    let res: mlua::Value = user_vm
        .call_fn(&user_vm.data.exec_prompt, (ctx_lua, payload, fuel))
        .await
        .unwrap();
    let table = res.as_table().unwrap();
    let data: llm_iface::PromptAnswerData =
        user_vm.vm.from_value(table.get("data").unwrap()).unwrap();

    match data {
        llm_iface::PromptAnswerData::Text(text) => {
            assert_eq!(text.trim().to_lowercase(), "ok");
        }
        _ => panic!("unexpected response format"),
    }

    assert!(made_request.load(std::sync::atomic::Ordering::SeqCst));

    server_task.await.unwrap();
}
