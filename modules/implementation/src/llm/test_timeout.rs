use genvm_common::logger;
use genvm_common::*;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

use super::config::ScriptBackendConfig;
use super::*;
use crate::common;
use crate::scripting;

#[tokio::test]
async fn test_timeout() {
    common::tests::setup();

    const BIND_ADDR: &str = "127.0.0.1:11435";
    const CONNECT_ADDR: &str = "http://127.0.0.1:11435";

    let server = tokio::net::TcpListener::bind(BIND_ADDR).await.unwrap();

    let server_task = tokio::spawn(async move {
        let (mut client, _) = server.accept().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        client
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"choices\":[{\"message\":{\"content\":\"ok\"}},],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1,\"total_tokens\":2}}")
            .await
            .unwrap();
        client.shutdown().await.unwrap();
    });

    let backend = config::BackendConfig {
        enabled: true,
        provider: config::Provider::OpenaiCompatible,
        key: "<empty>".to_owned(),
        script_config: ScriptBackendConfig {
            models: BTreeMap::from([(
                "model".to_owned(),
                config::ModelConfig {
                    enabled: true,
                    supports_json: false,
                    supports_image: false,
                    use_max_completion_tokens: false,
                    meta: serde_json::Value::Null,
                    timeout: None,
                },
            )]),
            meta: serde_json::Value::Null,
            timeout: None,
        },
        host: CONNECT_ADDR.to_owned(),
    };

    let provider = backend.to_provider();

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
        backends: BTreeMap::from([("1".to_owned(), backend)]),
        meta: serde_json::Value::Null,
        timeout: None,
    });

    let providers = Arc::new(BTreeMap::from([("1".to_owned(), provider)]));

    let user_vm = create_vm(&config).await.unwrap();

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

    let (_, ctx_lua) = user_vm.create_ctx(&sub_ctx).unwrap();

    let exec_prompt_in_provider: mlua::Function = user_vm
        .vm
        .load(
            r#"
            local llm = require("lib-llm")
            return function(ctx)
                return llm.rs.exec_prompt_in_provider(ctx, {
                    provider = "1",
                    model = "model",
                    format = "text",
                    timeout = 1.0,
                    prompt = {
                        system_message = nil,
                        user_message = "hello",
                        temperature = 0.0,
                        images = {},
                        max_tokens = 100,
                        use_max_completion_tokens = false,
                    },
                })
            end
            "#,
        )
        .eval()
        .unwrap();

    let res: Result<mlua::Value, _> = user_vm.call_fn(&exec_prompt_in_provider, (ctx_lua,)).await;

    assert!(res.is_err(), "expected timeout error, got success");

    server_task.abort();
}
