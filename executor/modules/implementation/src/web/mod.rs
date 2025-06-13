use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use genvm_common::*;

use crate::{
    common,
    scripting::{self, RSContext},
};

mod config;
mod ctx;
mod domains;
mod handler;

#[derive(clap::Args, Debug)]
pub struct CliArgs {
    #[arg(long, default_value_t = String::from("${genvmRoot}/config/genvm-module-web.yaml"))]
    config: String,

    #[arg(long, default_value_t = false)]
    die_with_parent: bool,
}

async fn check_status(webdriver_host: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let status_res = client
        .get(format!("{}/status", webdriver_host))
        .header("Content-Type", "application/json; charset=utf-8")
        .send()
        .await
        .with_context(|| "creating sessions request")?;

    let body = crate::common::read_response(status_res)
        .await
        .with_context(|| "reading response")?;

    let val: serde_json::Value = serde_json::from_str(&body)?;

    if val.pointer("/value/ready").and_then(|v| v.as_bool()) != Some(true) {
        anyhow::bail!("not ready {}", val)
    }

    Ok(())
}

pub fn entrypoint(args: CliArgs) -> Result<()> {
    let config = genvm_common::load_config(HashMap::new(), &args.config)
        .with_context(|| "loading config")?;
    let config: Arc<config::Config> = Arc::new(serde_yaml::from_value(config)?);

    config.base.setup_logging(std::io::stdout())?;

    let runtime = config.base.create_rt()?;

    let token = common::setup_cancels(&runtime, args.die_with_parent)?;

    let webdriver_host = config.webdriver_host.clone();

    let moved_config = config.clone();

    let vm_pool = runtime.block_on(scripting::pool::new(config.vm_count, move || {
        let moved_config = moved_config.clone();
        async {
            let mut user_vm =
                crate::scripting::UserVM::create("", move |vm: mlua::Lua| async move {
                    // set llm-related globals
                    vm.globals()
                        .set("__web", ctx::create_global(&vm, &moved_config)?)?;

                    // load script
                    scripting::load_script(&vm, &moved_config.lua_script_path).await?;

                    // get functions populated by script
                    let render: mlua::Function = vm.globals().get("Render")?;
                    let request: mlua::Function = vm.globals().get("Request")?;

                    Ok(ctx::VMData { render, request })
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
        Arc::new(handler::HandlerProvider { vm_pool, config }),
    );

    runtime
        .block_on(check_status(&webdriver_host))
        .with_context(|| "initial health check")?;

    log_info!("health is OK");

    runtime.block_on(loop_future)?;

    std::mem::drop(runtime);

    Ok(())
}
