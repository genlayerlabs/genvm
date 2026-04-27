use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use genvm_common::*;

use crate::{common, scripting};

type WebSubContext = crate::manager::execution_context::WebSubContext;

pub(crate) mod config;
mod ctx;
mod domains;
mod handler;

#[derive(serde::Serialize, Debug, Default)]
pub(crate) struct Metrics {
    pub scripting: scripting::Metrics,
}

impl<W: calldata::Writer> calldata::codec::Encode<W> for Metrics {
    type Error = W::Error;

    fn encode(&self, enc: &mut calldata::Encoder<W>) -> std::result::Result<(), Self::Error> {
        enc.start_map(1)?;
        enc.push_map_k("scripting")?;
        calldata::codec::Encode::encode(&self.scripting, enc)?;
        Ok(())
    }
}

#[derive(clap::Args, Debug)]
pub struct CliArgs {
    #[arg(long, default_value_t = String::from("${exeDir}/../config/genvm-module-web.yaml"))]
    config: String,

    #[arg(long, default_value_t = false)]
    die_with_parent: bool,
}

/// Creates the Web module and returns the stream handler.
/// The returned future runs the bind loop if bind_address is Some.
pub async fn create_web_module(
    cancel: Arc<cancellation::Token>,
    config: config::Config,
) -> Result<(
    crate::manager::modules::StreamHandler,
    impl std::future::Future<Output = Result<()>>,
    sync::DArc<config::Config>,
)> {
    let _webdriver_host = config.webdriver_host.clone();

    let config = sync::DArc::new(config);

    let moved_config = config.clone();

    let vm_pool = scripting::pool::new(config.mod_base.vm_count, move || {
        let moved_config = moved_config.clone();
        async move {
            let moved_config_for_data = moved_config.clone();
            let user_vm = crate::scripting::UserVM::create(
                &moved_config.mod_base,
                move |vm: mlua::Lua| async move {
                    // set web-related globals
                    vm.globals()
                        .set("__web", ctx::create_global(&vm, &moved_config_for_data)?)?;

                    // load script
                    scripting::load_script(&vm, &moved_config_for_data.mod_base.lua_script_path)
                        .await?;

                    // get functions populated by script
                    let render: mlua::Function = vm.globals().get("Render")?;
                    let request: mlua::Function = vm.globals().get("Request")?;

                    Ok(ctx::VMData { render, request })
                },
                Box::new(
                    move |vm: &mlua::Lua,
                          table: &mlua::Table,
                          sub_ctx: &sync::DArc<WebSubContext>| {
                        let scripting = sub_ctx.gep(|x| &x.scripting);
                        scripting::setup_lua_default_ctx(scripting, vm, table)?;

                        let ctx = Arc::new(ctx::CtxPart {});

                        table.set("__ctx_web", vm.create_userdata(ctx.clone())?)?;

                        Ok(ctx)
                    },
                ),
            )
            .await?;

            Ok(user_vm)
        }
    })
    .await?;

    let handler_provider = Arc::new(handler::HandlerProvider {
        vm_pool,
        config: config.clone(),
    });

    // Create the type-erased stream handler
    let stream_handler: crate::manager::modules::StreamHandler = {
        let hp = handler_provider.clone();
        Arc::new(move |stream: Box<dyn genvm_common::io::Stream>, exec_ctx| {
            let hp = hp.clone();
            Box::pin(async move {
                let sub_ctx = exec_ctx.map(|ctx| ctx.gep(|x| x.web.as_ref().unwrap()));
                crate::common::handle_stream(hp, stream, "relay", sub_ctx).await;
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        })
    };

    let bind_future = crate::common::run_loop(
        config.mod_base.bind_address.clone(),
        cancel,
        handler_provider,
    );

    Ok((stream_handler, bind_future, config))
}

pub async fn run_web_module(
    cancel: Arc<cancellation::Token>,
    config: config::Config,
) -> Result<()> {
    let (_handler, bind_future, _config) = create_web_module(cancel, config).await?;
    bind_future.await
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
