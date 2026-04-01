use anyhow::Result;
use genvm_common::*;
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use std::sync::Arc;

use crate::manager::{
    modules::{self, Ctx},
    run, versioning,
};
use crate::{common, llm, scripting};

use super::AppContext;

pub async fn handle_status(ctx: sync::DArc<AppContext>) -> Result<impl warp::Reply> {
    Ok(warp::reply::json(&serde_json::json!({
        "llm_module": ctx.mod_ctx.get_status(modules::Type::Llm).await,
        "web_module": ctx.mod_ctx.get_status(modules::Type::Web).await,
        "permits": {
            "current": ctx.run_ctx.get_current_permits(),
            "max": ctx.run_ctx.get_max_permits().await,
        },
        "executions": ctx.run_ctx.status_executions(),
    })))
}

#[derive(Debug, serde::Deserialize)]
struct StopRequest {
    module_type: modules::Type,
}

pub async fn handle_module_stop(
    ctx: sync::DArc<AppContext>,
    calldata: serde_json::Value,
) -> Result<impl warp::Reply, anyhow::Error> {
    let stop_request = serde_json::from_value::<StopRequest>(calldata.clone())?;

    let res = ctx.mod_ctx.stop(stop_request.module_type).await?;

    let res = if res {
        "module_stopped"
    } else {
        "module_not_running"
    };

    Ok(warp::reply::json(&serde_json::json!({"result": res})))
}

pub async fn handle_module_start(
    ctx: sync::DArc<AppContext>,
    calldata: serde_json::Value,
) -> Result<impl warp::Reply> {
    let req = serde_json::from_value::<modules::StartRequest>(calldata)?;

    ctx.mod_ctx.start(req).await?;

    Ok(warp::reply::json(
        &serde_json::json!({"result": "module_started"}),
    ))
}

pub async fn handle_module_restart(
    ctx: sync::DArc<AppContext>,
    calldata: serde_json::Value,
) -> Result<impl warp::Reply> {
    let req = serde_json::from_value::<modules::StartRequest>(calldata)?;

    ctx.mod_ctx.restart(req).await?;

    Ok(warp::reply::json(
        &serde_json::json!({"result": "module_restarted"}),
    ))
}

pub async fn handle_genvm_run(
    ctx: sync::DArc<AppContext>,
    data: &[u8],
) -> Result<impl warp::Reply> {
    let data = calldata::decode(data)?;
    let res: super::run::Request = calldata::from_value(data)?;

    let modules_lock = if res.needs_modules() {
        let lock = Ctx::get_module_locks(ctx.gep(|x| &x.mod_ctx)).await;
        if lock.is_none() {
            anyhow::bail!(
                "modules are required but not running (is_sync=false with 'n' permission)"
            );
        }
        lock
    } else {
        None
    };

    let (id, _) = super::run::start_genvm(ctx, res, Box::new(modules_lock)).await?;

    Ok(warp::reply::json(
        &serde_json::json!({"result": "started", "id": id}),
    ))
}

pub async fn handle_contract_detect_version(
    ctx: sync::DArc<AppContext>,
    contract_code: bytes::Bytes,
    deployment_timestamp: String,
) -> Result<impl warp::Reply> {
    let deployment_timestamp =
        chrono::DateTime::parse_from_rfc3339(&deployment_timestamp)?.with_timezone(&chrono::Utc);
    let major = versioning::detect_major_spec(&ctx, &contract_code, deployment_timestamp).await?;
    Ok(warp::reply::json(&serde_json::json!({
        "specified_major": major,
    })))
}

pub async fn handle_set_log_level(
    _ctx: sync::DArc<AppContext>,
    data: serde_json::Value,
) -> Result<impl warp::Reply> {
    let level = data
        .get("level")
        .and_then(|v| v.as_str())
        .and_then(|s| genvm_common::logger::Level::from_str(s).ok())
        .ok_or_else(|| anyhow::anyhow!("invalid log level"))?;

    let Some(logger) = genvm_common::logger::__LOGGER.get() else {
        anyhow::bail!("logger_not_initialized");
    };
    logger.set_filter(level);

    Ok(warp::reply::json(
        &serde_json::json!({"result": "log_level_set", "level": level}),
    ))
}

pub async fn handle_manifest_reload(ctx: sync::DArc<AppContext>) -> Result<impl warp::Reply> {
    ctx.ver_ctx.reload_manifest().await?;

    Ok(warp::reply::json(
        &serde_json::json!({"result": "manifest_reloaded"}),
    ))
}

pub async fn handle_set_env(
    _ctx: sync::DArc<AppContext>,
    data: serde_json::Value,
) -> Result<impl warp::Reply> {
    let key = data
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("invalid env var key"))?;

    let value = data
        .get("value")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("invalid env var value"))?;

    std::env::set_var(key, value);

    Ok(warp::reply::json(
        &serde_json::json!({"result": "env_var_set", "key": key}),
    ))
}

pub async fn handle_get_permits(ctx: sync::DArc<AppContext>) -> Result<impl warp::Reply> {
    let permits = ctx.run_ctx.get_max_permits().await;
    Ok(warp::reply::json(&serde_json::json!({"permits": permits})))
}

pub async fn handle_set_permits(
    ctx: sync::DArc<AppContext>,
    data: serde_json::Value,
) -> Result<impl warp::Reply> {
    let permits = data
        .get("permits")
        .and_then(|v| v.as_u64())
        .and_then(|v| usize::try_from(v).ok())
        .ok_or_else(|| anyhow::anyhow!("invalid permits"))?;

    let new_permits = ctx.run_ctx.set_permits(permits).await;

    Ok(warp::reply::json(
        &serde_json::json!({"result": "permits_set", "permits": new_permits}),
    ))
}

#[derive(Debug, serde::Deserialize)]
pub struct ShutdownRequest {}

pub async fn handle_genvm_shutdown(
    ctx: sync::DArc<AppContext>,
    genvm_id: run::GenVMId,
    _req: ShutdownRequest,
) -> Result<impl warp::Reply> {
    let result = ctx.run_ctx.graceful_shutdown(genvm_id).await;

    match result {
        Ok(()) => Ok(warp::reply::json(&serde_json::json!({
            "result": "shutdown_completed",
            "genvm_id": genvm_id
        }))),
        Err(e) => Ok(warp::reply::json(&serde_json::json!({
            "error": format!("{}", e),
            "genvm_id": genvm_id
        }))),
    }
}

pub async fn handle_genvm_status(
    ctx: sync::DArc<AppContext>,
    genvm_id: run::GenVMId,
) -> Result<impl warp::Reply> {
    let status = ctx.run_ctx.fetch_genvm_status(genvm_id).await;

    Ok(warp::reply::json(&serde_json::json!({
        "genvm_id": genvm_id,
        "status": status
    })))
}

#[derive(serde::Deserialize)]
pub struct LlmCheckRequest {
    pub configs: Vec<LlmProviderConfig>,
    pub test_prompts: Vec<llm::prompt::Internal>,
}

#[derive(serde::Deserialize)]
pub struct LlmProviderConfig {
    pub host: String,
    pub provider: llm::config::Provider,
    pub model: String,
    pub key: String,
}

#[derive(serde::Serialize)]
pub struct LlmAvailabilityResult {
    pub config_index: usize,
    pub prompt_index: usize,
    pub available: bool,
    pub error: Option<String>,
    pub response: Option<String>,
}

pub async fn handle_llm_check(
    _ctx: sync::DArc<AppContext>,
    data: serde_json::Value,
) -> Result<impl warp::Reply> {
    let request: LlmCheckRequest = serde_json::from_value(data)?;

    let mut results = Vec::new();

    for (config_idx, config_data) in request.configs.iter().enumerate() {
        for (prompt_idx, test_prompt) in request.test_prompts.iter().enumerate() {
            let result = check_llm_availability(config_data, test_prompt).await;

            let availability_result = match result {
                Ok(response) => LlmAvailabilityResult {
                    config_index: config_idx,
                    prompt_index: prompt_idx,
                    available: true,
                    error: None,
                    response: Some(response),
                },
                Err(error) => LlmAvailabilityResult {
                    config_index: config_idx,
                    prompt_index: prompt_idx,
                    available: false,
                    error: Some(error.to_string()),
                    response: None,
                },
            };

            results.push(availability_result);
        }
    }

    Ok(warp::reply::json(&results))
}

#[derive(Debug, serde::Deserialize)]
pub struct DescribeVmErrorRequest {
    pub error: String,
}

fn is_sub_error(err: &str, sub_err_of: &str) -> bool {
    if !err.starts_with(sub_err_of) {
        return false;
    }
    sub_err_of.len() == err.len() || err.as_bytes()[sub_err_of.len()] == b' '
}

fn describe_vm_error(error: &str) -> Option<&'static str> {
    if is_sub_error(error, "wasm_trap") {
        Some("Web Assembly trap reached")
    } else if is_sub_error(error, "exit_code") {
        Some("Non-zero exit code from the contract. Check stderr for contract-provided details")
    } else if is_sub_error(error, "invalid_contract not_utf8_text") {
        Some(
            r##"The contract was detected to be a plain text contract, however it contains non-UTF8 bytes and hence cannot be parsed. Is deployed runner a valid contract?"##,
        )
    } else if is_sub_error(error, "invalid_contract absent_runner_comment") {
        Some(
            r##"The contract was detected to be a plain text contract, however it does not start with a runner comment (such as `# { "Depends": "py-genlayer:..." }`), hence it is impossible to run. Have you forgotten to add it or is there other content before it?"##,
        )
    } else if is_sub_error(error, "invalid_contract") {
        Some("Execution failed before running the contract, likely due to invalid or malformed contract runner")
    } else if is_sub_error(error, "OOM storage") {
        Some("Contract ran out of storage pages it could (re)write")
    } else if is_sub_error(error, "OOM") {
        Some("Contract exceeded allowed execution memory (RAM) limit")
    } else {
        None
    }
}

pub async fn handle_describe_vm_error(
    _ctx: sync::DArc<AppContext>,
    request: DescribeVmErrorRequest,
) -> Result<impl warp::Reply> {
    let description = describe_vm_error(&request.error);

    Ok(warp::reply::json(
        &serde_json::json!({ "description": description }),
    ))
}

async fn check_llm_availability(
    config_data: &LlmProviderConfig,
    test_prompt: &llm::prompt::Internal,
) -> Result<String> {
    let backend = serde_json::json!({
        "host": config_data.host,
        "provider": config_data.provider,
        "models": {
            &config_data.model: {}
        },
        "key": config_data.key
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

    let backend: llm::config::BackendConfig = serde_json::from_value(backend)?;
    let provider = backend.to_provider();

    let ctx = scripting::CtxPart {
        client: common::create_client()?,
        metrics: sync::DArc::new(scripting::Metrics::default()),
        node_address: "test_node".to_owned(),
        sign_headers: Arc::new(BTreeMap::new()),
        sign_url: Arc::from("test_url"),
        sign_vars: BTreeMap::new(),
        hello: Arc::new(genvm_modules_interfaces::GenVMHello {
            genvm_id: genvm_modules_interfaces::GenVMId(999),
            host_data: genvm_modules_interfaces::HostData {
                node_address: "test_node".to_owned(),
                tx_id: "test_tx".to_owned(),
                rest: serde_json::Map::new(),
            },
        }),
    };

    let response = provider
        .exec_prompt_text(&ctx, test_prompt, &config_data.model)
        .await?;

    Ok(response.result)
}
