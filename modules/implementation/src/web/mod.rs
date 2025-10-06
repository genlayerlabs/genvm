use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use genvm_common::*;

use crate::{common, scripting};

mod config;
mod ctx;
mod domains;
mod handler;

#[derive(serde::Serialize, Debug, Default)]
struct Metrics {
    pub scripting: scripting::Metrics,
}

#[derive(clap::Args, Debug)]
pub struct CliArgs {
    #[arg(long, default_value_t = String::from("${exeDir}/../config/genvm-module-web.yaml"))]
    config: String,

    #[arg(long, default_value_t = false)]
    die_with_parent: bool,
}

pub async fn run_web_module(
    cancel: Arc<cancellation::Token>,
    config: config::Config,
) -> Result<()> {
    let _webdriver_host = config.webdriver_host.clone();

    let config = sync::DArc::new(config);

    let moved_config = config.clone();

    let vm_pool = scripting::pool::new(config.mod_base.vm_count, move || {
        let moved_config_1 = moved_config.clone();
        let moved_config_2 = moved_config.clone();
        let moved_config = moved_config.clone();
        async move {
            let user_vm = crate::scripting::UserVM::create(
                &moved_config_1.mod_base,
                move |vm: mlua::Lua| async move {
                    // set web-related globals
                    vm.globals()
                        .set("__web", ctx::create_global(&vm, &moved_config)?)?;

                    // load script
                    scripting::load_script(&vm, &moved_config.mod_base.lua_script_path).await?;

                    // get functions populated by script
                    let render: mlua::Function = vm.globals().get("Render")?;
                    let request: mlua::Function = vm.globals().get("Request")?;

                    Ok(ctx::VMData { render, request })
                },
                Box::new(move |vm: &mlua::Lua, table: &mlua::Table, hello| {
                    let metrics = sync::DArc::new(Metrics::default());

                    let _dflt_ctx = scripting::create_default_ctx(
                        hello,
                        moved_config_2.gep(|x| &x.mod_base),
                        metrics.gep(|x| &x.scripting),
                        vm,
                        table,
                    )?;

                    let ctx = Arc::new(ctx::CtxPart {});

                    table.set("__ctx_web", vm.create_userdata(ctx.clone())?)?;

                    Ok(ctx)
                }),
            )
            .await?;

            Ok(user_vm)
        }
    })
    .await?;

    crate::common::run_loop(
        config.mod_base.bind_address.clone(),
        cancel,
        Arc::new(handler::HandlerProvider { vm_pool }),
    )
    .await
}

pub fn entrypoint(args: CliArgs) -> Result<()> {
    let config = genvm_common::load_config(HashMap::new(), &args.config)
        .with_context(|| "loading config")?;
    let config: config::Config = serde_yaml::from_value(config)?;

    config.base.setup_logging(std::io::stdout())?;

    let runtime = config.base.create_rt()?;

    let token = common::setup_cancels(&runtime, args.die_with_parent)?;

    runtime.block_on(run_web_module(token, config))?;

    std::mem::drop(runtime);

    Ok(())
}
