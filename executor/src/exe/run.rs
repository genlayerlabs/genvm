use std::io::Write;

use anyhow::{Context, Result};
use clap::ValueEnum;
use genvm::{config, vm::RunOk, PublicArgs};

#[derive(Debug, Clone, ValueEnum, PartialEq)]
#[clap(rename_all = "kebab_case")]
enum PrintOption {
    Shrink,
    All,
    None,
}

impl std::fmt::Display for PrintOption {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&format!("{:?}", self).to_ascii_lowercase())
    }
}

#[derive(clap::Args, Debug)]
pub struct Args {
    #[arg(long)]
    allow_latest: bool,

    #[arg(long)]
    message: String,
    #[arg(long)]
    host: String,
    #[arg(long)]
    cookie: Option<String>,
    #[clap(long, default_value_t = PrintOption::None)]
    print: PrintOption,
    #[clap(long, default_value_t = false)]
    sync: bool,
    #[clap(
        long,
        default_value = "rwscn",
        help = "r?w?s?c?n?, read/write/send messages/call contracts/spawn nondet"
    )]
    permissions: String,

    #[clap(long, default_value = "{}")]
    host_data: String,
}

pub fn handle(args: Args, config: config::Config) -> Result<()> {
    let message: genvm::MessageData = serde_json::from_str(&args.message)?;

    let host = genvm::Host::new(&args.host)?;

    let mut perm_size = 0;
    for perm in ["r", "w", "s", "c", "n"] {
        if args.permissions.contains(perm) {
            perm_size += 1;
        }
    }

    if perm_size != args.permissions.len() {
        anyhow::bail!("Invalid permissions {}", &args.permissions)
    }

    let runtime = config.base.create_rt()?;

    let (token, canceller) = genvm_common::cancellation::make();

    let handle_sigterm = move || {
        log::warn!("sigterm received");
        canceller();
    };
    unsafe {
        signal_hook::low_level::register(signal_hook::consts::SIGTERM, handle_sigterm.clone())?;
        signal_hook::low_level::register(signal_hook::consts::SIGINT, handle_sigterm)?;
    }

    let host_data = serde_json::from_str(&args.host_data)?;

    let cookie = match &args.cookie {
        None => {
            let mut cookie = [0; 8];
            let _ = getrandom::fill(&mut cookie);

            let mut cookie_str = String::new();
            for c in cookie {
                cookie_str.push_str(&format!("{:x}", c));
            }
            cookie_str
        }
        Some(v) => v.clone(),
    };

    log::info!(cookie = cookie; "genvm cookie");

    let supervisor = genvm::create_supervisor(
        &config,
        host,
        token,
        host_data,
        PublicArgs {
            cookie,
            is_sync: args.sync,
            allow_latest: args.allow_latest,
        },
    )
    .with_context(|| "creating supervisor")?;

    let res = runtime
        .block_on(genvm::run_with(
            message,
            supervisor.clone(),
            &args.permissions,
        ))
        .with_context(|| "running genvm");

    if let Err(err) = &res {
        log::error!(error = genvm_common::log_error(err); "error running genvm");
    }

    let res: Option<String> = match (res, args.print) {
        (_, PrintOption::None) => None,
        (Ok(RunOk::ContractError(e, cause)), PrintOption::Shrink) => {
            eprintln!("shrunk contract error {:?}", cause);
            Some(format!("ContractError(\"{e}\")"))
        }
        (Err(e), PrintOption::Shrink) => {
            eprintln!("shrunk error {:?}", e);
            match e.downcast_ref::<wasmtime::Trap>() {
                None => Some("Error(\"\")".into()),
                Some(e) => Some(format!("Error(\"{e:?}\")")),
            }
        }
        (Err(e), _) => Some(format!("Error({})", e)),
        (Ok(res), _) => Some(format!("{:?}", &res)),
    };
    match res {
        None => {}
        Some(res) => println!("executed with `{res}`"),
    }

    runtime.block_on(async {
        let supervisor = supervisor.lock().await;
        supervisor.shared_data.modules.llm.close().await;
        supervisor.shared_data.modules.web.close().await;
    });

    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    runtime.shutdown_timeout(std::time::Duration::from_millis(30));

    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    Ok(())
}
