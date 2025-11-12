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

pub async fn handle_genvm_run(
    ctx: sync::DArc<AppContext>,
    data: serde_json::Value,
) -> Result<impl warp::Reply> {
    let modules_lock = Ctx::get_module_locks(ctx.gep(|x| &x.mod_ctx)).await;

    if modules_lock.is_none() {
        log_warn!("modules are not running, but are most likely required for genvm_run");
    }

    let res: super::run::Request = serde_json::from_value(data)?;

    let (id, _) = super::run::start_genvm(ctx, res, Box::new(modules_lock)).await?;

    Ok(warp::reply::json(
        &serde_json::json!({"result": "started", "id": id}),
    ))
}

pub async fn handle_genvm_run_readonly(
    ctx: sync::DArc<AppContext>,
    contract_code: bytes::Bytes,
    timestamp: String,
) -> Result<impl warp::Reply> {
    if true {
        return Err(anyhow::anyhow!("readonly execution is not implemented yet"));
    }

    let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp)?.with_timezone(&chrono::Utc);

    let major = versioning::detect_major_spec(&ctx, &contract_code, timestamp).await?;

    let message = serde_json::json!({
        "contract_address": "AAAAAAAAAAAAAAAAAAAAAAAAAAA=",
        "sender_address": "AAAAAAAAAAAAAAAAAAAAAAAAAAA=",
        "origin_address": "AAAAAAAAAAAAAAAAAAAAAAAAAAA=",
        "chain_id": "0",
        "value": null,
        "is_init": false,
    });

    let req = run::Request {
        major,
        message,
        is_sync: false,
        capture_output: false,
        max_execution_minutes: 1,
        host_data: r#"{"tx_id": "0x", "node_address": "0x"}"#.to_owned(),
        timestamp,
        host: "TODO".to_owned(),
        extra_args: Vec::new(),
    };
    let (genvm_id, recv) = run::start_genvm(ctx.clone(), req, Box::new(())).await?;

    let _ = recv.await;

    let _ = ctx.run_ctx.get_genvm_status(genvm_id).await;

    Ok(warp::reply::json(
        &serde_json::json!({"schema": "contract_schema"}),
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
pub struct ShutdownRequest {
    #[serde(default = "default_wait_timeout")]
    wait_timeout_ms: u64,
}

fn default_wait_timeout() -> u64 {
    30000
}

pub async fn handle_genvm_shutdown(
    ctx: sync::DArc<AppContext>,
    genvm_id: run::GenVMId,
    req: ShutdownRequest,
) -> Result<impl warp::Reply> {
    let result = ctx
        .run_ctx
        .graceful_shutdown(genvm_id, req.wait_timeout_ms)
        .await;

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

#[derive(serde::Serialize)]
struct SingleWrite(
    #[serde(serialize_with = "serialize_bytes_as_base64")] [u8; 36],
    #[serde(serialize_with = "serialize_vec_as_base64")] Vec<u8>,
);

fn serialize_bytes_as_base64<S>(bytes: &[u8; 36], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    serializer.serialize_str(&encoded)
}

fn serialize_vec_as_base64<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    serializer.serialize_str(&encoded)
}

pub async fn handle_make_deployment_storage_writes(
    ctx: sync::DArc<AppContext>,
    deployment_timestamp: String,
    code: bytes::Bytes,
) -> Result<impl warp::Reply> {
    let deployment_timestamp =
        chrono::DateTime::parse_from_rfc3339(&deployment_timestamp)?.with_timezone(&chrono::Utc);

    let major = versioning::detect_major_spec(&ctx, &code, deployment_timestamp).await?;

    use sha3::{Digest, Sha3_256};

    let mut code_digest = Sha3_256::new();
    code_digest.update([0u8; 32]);
    const CODE_OFFSET: u32 = 1;
    code_digest.update(CODE_OFFSET.to_le_bytes());

    // Get the digest as code_slot
    let code_slot: [u8; 32] = code_digest.finalize().into();

    // Create storage writes
    let mut writes_seq = Vec::new();

    // r1: code_slot + offset 0, value = code length as little-endian bytes
    let mut key1 = [0u8; 36];
    key1[..32].copy_from_slice(&code_slot);
    key1[32..36].copy_from_slice(&0u32.to_le_bytes());
    let value1 = (code.len() as u32).to_le_bytes().to_vec();
    writes_seq.push(SingleWrite(key1, value1));

    // r2: code_slot + offset 4, value = code
    let mut key2 = [0u8; 36];
    key2[..32].copy_from_slice(&code_slot);
    key2[32..36].copy_from_slice(&4u32.to_le_bytes());
    let value2 = code.to_vec();
    writes_seq.push(SingleWrite(key2, value2));

    if major != 0 {
        anyhow::bail!("only major version 0 is supported for now");
    }

    Ok(warp::reply::json(&serde_json::json!({
        "writes": writes_seq,
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
            cookie: "test_cookie".to_owned(),
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

    Ok(response)
}
