use genvm_common::*;
use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use warp::Filter;

use crate::common;

mod handlers;
mod modules;
mod run;
mod versioning;

#[derive(Debug)]
struct AnyhowRejection(anyhow::Error);

impl warp::reject::Reject for AnyhowRejection {}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Config {
    #[serde(flatten)]
    pub base: genvm_common::BaseConfig,
    pub manifest_path: String,
    #[serde(skip_deserializing)]
    pub reroute_to: Arc<str>,

    pub permits: Option<usize>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ExecutorVersion {
    pub available_after: String,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct VersionParts {
    pub major: Option<u32>,
    pub minor: Option<u32>,
    pub patch: Option<u32>,
}

pub fn parse_version(version: &str) -> Result<VersionParts> {
    let version_re = regex::Regex::new(r"^v(\d+|\*)\.(\d+|\*)\.(\d+|\*)$").unwrap();
    let captures = version_re
        .captures(version)
        .ok_or_else(|| anyhow::anyhow!("Invalid version format: {}", version))?;

    let parse_part = |part: &str| -> Option<u32> {
        if part == "*" {
            None
        } else {
            part.parse().ok()
        }
    };

    Ok(VersionParts {
        major: parse_part(&captures[1]),
        minor: parse_part(&captures[2]),
        patch: parse_part(&captures[3]),
    })
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Manifest {
    pub executor_versions: std::collections::BTreeMap<String, ExecutorVersion>,
}

pub struct AppContext {
    pub cancel: Arc<cancellation::Token>,
    pub config: sync::DArc<Config>,
    pub mod_ctx: modules::Ctx,
    pub run_ctx: run::Ctx,
    pub ver_ctx: versioning::Ctx,
}

#[derive(clap::Args, Debug)]
pub struct CliArgs {
    #[arg(long, default_value_t = 3999)]
    pub port: u16,
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    #[arg(long, default_value = "")]
    pub reroute_to: String,

    #[arg(long, default_value_t = false)]
    die_with_parent: bool,

    #[arg(long, default_value_t = String::from("${exeDir}/../config/genvm-manager.yaml"))]
    pub config: String,

    #[arg(long)]
    pub manifest_path: Option<String>,
}

fn unwrap_all_anyhow<R: warp::Reply + 'static>(
    route: impl warp::Filter<Extract = (anyhow::Result<R>,), Error = warp::Rejection>
        + Clone
        + Send
        + Sync
        + 'static,
) -> impl warp::Filter<Error = warp::Rejection, Extract: warp::Reply> + Clone + Send + Sync + 'static
{
    route
        .boxed()
        .and_then(|x: anyhow::Result<R>| async move {
            x.map_err(|e| warp::reject::custom(AnyhowRejection(e)))
        })
        .recover(|err: warp::reject::Rejection| async move {
            let Some(AnyhowRejection(inner)) = err.find::<AnyhowRejection>() else {
                return Err(err);
            };
            log_error!(err:ah = inner; "internal server error");
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({"error": format!("{:#}", inner)})),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        })
}

async fn run_http_server(
    cancel: Arc<cancellation::Token>,
    config: sync::DArc<Config>,
    args: &CliArgs,
) -> Result<()> {
    let app_ctx = sync::DArc::new(AppContext {
        cancel: cancel.clone(),
        mod_ctx: modules::Ctx::new(cancel.clone()),
        config: config.clone(),
        run_ctx: run::Ctx::new(&config)?,
        ver_ctx: versioning::Ctx::new(config.clone()).await?,
    });

    run::start_service(app_ctx.gep(|x| &x.run_ctx), cancel.clone()).await?;

    let ctx = app_ctx.clone();
    let status_route = unwrap_all_anyhow(warp::path("status").and(warp::get()).then(move || {
        let ctx = ctx.clone();
        async move { handlers::handle_status(ctx).await }
    }));

    let ctx = app_ctx.clone();
    let start_route = unwrap_all_anyhow(
        warp::path!("module" / "start")
            .and(warp::post())
            .and(warp::body::json())
            .then(move |calldata| {
                let ctx = ctx.clone();
                async move { handlers::handle_module_start(ctx, calldata).await }
            }),
    );

    let ctx = app_ctx.clone();
    let stop_route = unwrap_all_anyhow(
        warp::path!("module" / "stop")
            .and(warp::post())
            .and(warp::body::json())
            .then(move |calldata| {
                let ctx = ctx.clone();
                async move { handlers::handle_module_stop(ctx, calldata).await }
            }),
    );

    let ctx = app_ctx.clone();
    let genvm_run_route = unwrap_all_anyhow(
        warp::path!("genvm" / "run")
            .and(warp::post())
            .and(warp::body::json())
            .then(move |data| {
                let ctx = ctx.clone();
                async move { handlers::handle_genvm_run(ctx, data).await }
            }),
    );

    let ctx = app_ctx.clone();
    let genvm_run_readonly = unwrap_all_anyhow(
        warp::path!("genvm" / "run" / "readonly")
            .and(warp::post())
            .and(warp::body::bytes())
            .and(warp::header::<String>("Deployment-Timestamp"))
            .then(move |contract_code, deployment_timestamp: String| {
                let ctx = ctx.clone();
                async move {
                    handlers::handle_genvm_run_readonly(ctx, contract_code, deployment_timestamp)
                        .await
                }
            }),
    );

    let ctx = app_ctx.clone();
    let contract_detect_version_route = unwrap_all_anyhow(
        warp::path!("contract" / "detect-version")
            .and(warp::post())
            .and(warp::body::bytes())
            .and(warp::header::<String>("Deployment-Timestamp"))
            .then(move |contract_code, deployment_timestamp| {
                let ctx = ctx.clone();
                async move {
                    handlers::handle_contract_detect_version(
                        ctx,
                        contract_code,
                        deployment_timestamp,
                    )
                    .await
                }
            }),
    );

    let ctx = app_ctx.clone();
    let set_log_level_route = unwrap_all_anyhow(
        warp::path!("log" / "level")
            .and(warp::post())
            .and(warp::body::json())
            .then(move |data| {
                let ctx = ctx.clone();
                async move { handlers::handle_set_log_level(ctx, data).await }
            }),
    );

    let ctx = app_ctx.clone();
    let manifest_reload_route = unwrap_all_anyhow(
        warp::path!("manifest" / "reload")
            .and(warp::post())
            .then(move || {
                let ctx = ctx.clone();
                async move { handlers::handle_manifest_reload(ctx).await }
            }),
    );

    let ctx = app_ctx.clone();
    let set_env_route = unwrap_all_anyhow(
        warp::path("env")
            .and(warp::post())
            .and(warp::body::json())
            .then(move |data| {
                let ctx = ctx.clone();
                async move { handlers::handle_set_env(ctx, data).await }
            }),
    );

    let ctx = app_ctx.clone();
    let get_permits_route =
        unwrap_all_anyhow(warp::path("permits").and(warp::get()).then(move || {
            let ctx = ctx.clone();
            async move { handlers::handle_get_permits(ctx).await }
        }));

    let ctx = app_ctx.clone();
    let set_permits_route = unwrap_all_anyhow(
        warp::path("permits")
            .and(warp::post())
            .and(warp::body::json())
            .then(move |data| {
                let ctx = ctx.clone();
                async move { handlers::handle_set_permits(ctx, data).await }
            }),
    );

    let ctx = app_ctx.clone();
    let genvm_shutdown_route = unwrap_all_anyhow(
        warp::path!("genvm" / u64)
            .and(warp::delete())
            .and(warp::query::<handlers::ShutdownRequest>())
            .then(move |genvm_id, shutdown_req| {
                let ctx = ctx.clone();
                async move {
                    handlers::handle_genvm_shutdown(ctx, run::GenVMId(genvm_id), shutdown_req).await
                }
            }),
    );

    let ctx = app_ctx.clone();
    let genvm_status_route = unwrap_all_anyhow(warp::path!("genvm" / u64).and(warp::get()).then(
        move |genvm_id| {
            let ctx = ctx.clone();
            async move { handlers::handle_genvm_status(ctx, run::GenVMId(genvm_id)).await }
        },
    ));

    let ctx = app_ctx.clone();
    let make_deployment_storage_writes_route = unwrap_all_anyhow(
        warp::path!("contract" / "pre-deploy-writes")
            .and(warp::post())
            .and(warp::body::bytes())
            .and(warp::header::<String>("Deployment-Timestamp"))
            .then(move |code, deployment_timestamp: String| {
                let ctx = ctx.clone();
                async move {
                    handlers::handle_make_deployment_storage_writes(ctx, deployment_timestamp, code)
                        .await
                }
            }),
    );

    let ctx = app_ctx.clone();
    let llm_check_route = unwrap_all_anyhow(
        warp::path!("llm" / "check")
            .and(warp::post())
            .and(warp::body::json())
            .then(move |data| {
                let ctx = ctx.clone();
                async move { handlers::handle_llm_check(ctx, data).await }
            }),
    );

    let routes = status_route
        .or(start_route)
        .or(stop_route)
        .or(genvm_run_route)
        .or(genvm_run_readonly)
        .or(contract_detect_version_route)
        .or(set_log_level_route)
        .or(manifest_reload_route)
        .or(set_env_route)
        .or(get_permits_route)
        .or(set_permits_route)
        .or(genvm_shutdown_route)
        .or(genvm_status_route)
        .or(make_deployment_storage_writes_route)
        .or(llm_check_route);

    let routes = routes.recover(|err: warp::reject::Rejection| async move {
        if err.is_not_found() {
            Ok::<_, std::convert::Infallible>(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({"error": "Not Found"})),
                warp::http::StatusCode::NOT_FOUND,
            ))
        } else {
            let err_format = format!("{:?}", err);
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({"error": err_format})),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    });

    let cancellation = cancel.clone();

    let serv = warp::serve(routes);
    let (addr, fut) = serv.bind_with_graceful_shutdown(
        (args.host.parse::<std::net::IpAddr>()?, args.port),
        async move { cancellation.chan.closed().await },
    );

    log_info!(address:? = addr; "HTTP server started");
    fut.await;
    log_info!(address:? = addr; "HTTP server stopped");

    Ok(())
}

async fn main_loop(
    cancel: Arc<cancellation::Token>,
    args: &CliArgs,
    config: sync::DArc<Config>,
) -> Result<()> {
    run_http_server(cancel, config, args).await
}

pub fn entrypoint(args: CliArgs) -> Result<()> {
    let config = genvm_common::load_config(HashMap::new(), &args.config)
        .with_context(|| "loading config")?;
    let mut config: Config = serde_yaml::from_value(config)?;

    if let Some(manifest_path) = &args.manifest_path {
        config.manifest_path = manifest_path.clone();
    }

    config.reroute_to = Arc::from(args.reroute_to.as_str());

    let config: sync::DArc<Config> = sync::DArc::new(config);

    config.base.setup_logging(std::io::stdout())?;

    let runtime = config.base.create_rt()?;

    let token = common::setup_cancels(&runtime, args.die_with_parent)?;

    runtime.block_on(main_loop(token, &args, config))?;

    Ok(())
}
