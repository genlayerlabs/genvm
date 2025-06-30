use std::io::Write;

use genvm_common::*;

use anyhow::{Context, Result};
use clap::ValueEnum;
use genvm::{config, vm::RunOk, PublicArgs};

#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
#[clap(rename_all = "kebab_case")]
enum PrintOption {
    Result,
    Fingerprint,
    StderrFull,
}

impl std::fmt::Display for PrintOption {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&format!("{self:?}").to_ascii_lowercase())
    }
}

macro_rules! combine {
    ($A:expr, $B:expr) => {{
        const LEN: usize = $A.len() + $B.len();
        const fn combine(a: &'static str, b: &'static str) -> [u8; LEN] {
            let mut out = [0u8; LEN];
            out = copy_slice(a.as_bytes(), out, 0);
            out = copy_slice(b.as_bytes(), out, a.len());
            out
        }
        const fn copy_slice(input: &[u8], mut output: [u8; LEN], offset: usize) -> [u8; LEN] {
            let mut index = 0;
            loop {
                output[offset + index] = input[index];
                index += 1;
                if index == input.len() {
                    break;
                }
            }
            output
        }
        const COMBINED_TO_ARRAY: [u8; LEN] = combine($A, $B);
        unsafe { std::str::from_utf8_unchecked(&COMBINED_TO_ARRAY as &[u8]) }
    }};
}

const MESSAGE_SCHEMA: &str = include_str!("../../../doc/schemas/message.json");
const MESSAGE_SCHEMA_HELP: &str = combine!("message, follows schema:\n", MESSAGE_SCHEMA);

#[derive(clap::Args, Debug)]
pub struct Args {
    #[arg(
        long,
        help = "whenever to allow `:latest` and `:test` as runners version"
    )]
    allow_latest: bool,

    #[arg(long, help = MESSAGE_SCHEMA_HELP)]
    message: String,
    #[arg(long, help = "host uri, preferably unix://")]
    host: String,
    #[arg(long, help = "id to pass to modules, useful for aggregating logs")]
    cookie: Option<String>,
    #[clap(long, help = "what to output to stdout/stderr")]
    print: Vec<PrintOption>,
    #[clap(long, default_value_t = false)]
    sync: bool,
    #[clap(
        long,
        default_value = "rwscn",
        help = "r?w?s?c?n?, read/write/send messages/call contracts/spawn nondet"
    )]
    permissions: String,

    #[clap(long, default_value = "{}", help = "value to pass to modules")]
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
        log_warn!("sigterm received");
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
                cookie_str.push_str(&format!("{c:x}"));
            }
            cookie_str
        }
        Some(v) => v.clone(),
    };

    log_info!(cookie = cookie; "genvm cookie");

    let supervisor = genvm::create_supervisor(
        &config,
        host,
        token,
        host_data,
        PublicArgs {
            cookie,
            is_sync: args.sync,
            allow_latest: args.allow_latest,
            message: &message,
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
        log_error!(error:ah = err; "error running genvm");
    }

    if args.print.contains(&PrintOption::StderrFull) {
        eprintln!("{res:?}");
    }

    if args.print.contains(&PrintOption::Result) {
        match &res {
            Ok((RunOk::VMError(e, cause), _)) => {
                println!("executed with `VMError(\"{e}\")`");
                if let Some(cause) = cause {
                    eprintln!("{cause:?}");
                }
            }
            Ok((res, _)) => {
                println!("executed with `{res:?}`")
            }
            Err(err) => {
                println!("executed with `InternalError(\"\")`");
                eprintln!("{err:?}");
            }
        }
    }

    if args.print.contains(&PrintOption::Fingerprint) {
        if let Ok((_, fp)) = &res {
            println!("Fingerprint: {fp:?}");
        }
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
