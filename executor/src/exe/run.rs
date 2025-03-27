use std::io::Write;

use anyhow::{Context, Result};
use clap::ValueEnum;
use genvm::vm::RunOk;

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
    #[arg(long, default_value_t = String::from("${genvmRoot}/etc/genvm-config.json"))]
    config: String,
    #[arg(long, default_value_t = 4)]
    threads: usize,
    #[arg(long)]
    message: String,
    #[arg(long)]
    host: String,
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
}

pub fn handle(args: Args) -> Result<()> {
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

    let (token, canceller) = genvm_modules_interfaces::make_cancellation();

    let supervisor = genvm::create_supervisor(&args.config, host, args.sync, token)
        .with_context(|| "creating supervisor")?;

    let handle_sigterm = move || {
        log::warn!(target = "rt"; "sigterm received");
        canceller();
    };
    unsafe {
        signal_hook::low_level::register(signal_hook::consts::SIGTERM, handle_sigterm.clone())?;
        signal_hook::low_level::register(signal_hook::consts::SIGINT, handle_sigterm)?;
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .worker_threads(args.threads)
        .max_blocking_threads(args.threads)
        .build()?;

    let res = runtime
        .block_on(genvm::run_with(message, supervisor, &args.permissions))
        .with_context(|| "running genvm");
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

    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    runtime.shutdown_timeout(std::time::Duration::from_millis(30));

    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    Ok(())
}
