use std::{
    io::{Read, Write},
    os::fd::FromRawFd,
};

use genvm_common::*;

use anyhow::{Context, Result};
use genvm::{
    config,
    rt::{self},
};

const EXECUTION_DATA_HELP: &str = "path to file containing encoded execution data (use '-' for stdin, 'fd://N' for file descriptor N)";

#[derive(clap::Args, Debug)]
pub struct Args {
    #[arg(
        long,
        help = "whenever to allow `:latest` and `:test` as runners version, tracing, etc."
    )]
    debug_mode: bool,

    #[arg(long, default_value = "-", help = EXECUTION_DATA_HELP)]
    execution_data: String,
    #[arg(
        long,
        help = "host connections in format id=uri (e.g. 0=unix:///path or 1=fd://N)"
    )]
    host: Vec<String>,
    #[arg(long, help = "id to pass to modules, useful for aggregating logs")]
    genvm_id: Option<u64>,
    #[clap(long, default_value_t = false)]
    sync: bool,
    #[clap(
        long,
        default_value = "rwscn",
        help = "r?w?s?c?n?, read/write/send messages/call contracts/spawn nondet"
    )]
    permissions: String,
    #[clap(long, help = "override LLM module address from config")]
    module_llm: Option<String>,
    #[clap(long, help = "override web module address from config")]
    module_web: Option<String>,
}

pub fn handle(args: Args, mut config: config::Config) -> Result<()> {
    // Apply CLI overrides for module addresses
    if let Some(llm_addr) = &args.module_llm {
        config.modules.llm.address = llm_addr.clone();
    }
    if let Some(web_addr) = &args.module_web {
        config.modules.web.address = web_addr.clone();
    }

    // Read execution data from file path, stdin, or file descriptor
    let execution_data_bytes = if args.execution_data == "-" {
        let mut buffer = Vec::new();
        std::io::stdin()
            .read_to_end(&mut buffer)
            .with_context(|| "reading execution data from stdin")?;
        buffer
    } else if let Some(fd_str) = args.execution_data.strip_prefix("fd://") {
        let fd: i32 = fd_str
            .parse()
            .with_context(|| format!("parsing file descriptor number from '{fd_str}'"))?;
        let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .with_context(|| format!("reading execution data from fd {fd}"))?;
        std::mem::drop(file);
        buffer
    } else {
        std::fs::read(&args.execution_data)
            .with_context(|| format!("reading execution data from {}", args.execution_data))?
    };

    let execution_data = calldata::decode_obj::<domain::ExecutionData>(&execution_data_bytes)
        .with_context(|| "decoding execution data")?;
    let message = &execution_data.message;
    let host_data = rt::parse_host_data(&execution_data)?;

    let runtime = config
        .base
        .create_rt()
        .with_context(|| "creating tokio runtime")?;

    let (token, canceller) = genvm_common::cancellation::make();

    let handle_sigterm = move || {
        log_warn!("sigterm received");
        canceller();
    };
    unsafe {
        signal_hook::low_level::register(signal_hook::consts::SIGTERM, handle_sigterm.clone())?;
        signal_hook::low_level::register(signal_hook::consts::SIGINT, handle_sigterm)?;
    }

    let genvm_id = match &args.genvm_id {
        None => {
            let mut random_bytes = [0; 8];
            let _ = getrandom::fill(&mut random_bytes);
            u64::from_le_bytes(random_bytes)
        }
        Some(v) => *v,
    };

    if args.host.is_empty() {
        anyhow::bail!("at least one --host must be specified");
    }

    let mut metrics = genvm::Metrics::default();
    let metrics_for_each_host = args
        .host
        .iter()
        .map(|_| genvm::host::Metrics::default())
        .collect::<Vec<_>>();
    metrics.hosts = Box::from(metrics_for_each_host);

    let bucket_totals = execution_data
        .bucket_totals
        .iter()
        .map(|bi| {
            let (sign, bytes) = bi.to_bytes_be();
            anyhow::ensure!(
                sign != num_bigint::Sign::Minus,
                "bucket total must be non-negative"
            );
            anyhow::ensure!(bytes.len() <= 32, "bucket total exceeds U256 range");
            let mut buf = [0u8; 32];
            let start = 32usize.saturating_sub(bytes.len());
            buf[start..].copy_from_slice(&bytes);
            Ok(primitive_types::U256::from_big_endian(&buf))
        })
        .collect::<Result<Vec<_>>>()?;

    let max_bucket_no = [
        config.fees.storage.bucket_no,
        config.fees.message_receipt.bucket_no,
        config.fees.nondet_output.bucket_no,
        config.fees.message_fee.bucket_no,
    ]
    .into_iter()
    .max()
    .unwrap_or(0);

    anyhow::ensure!(
        (max_bucket_no as usize) < bucket_totals.len(),
        "fees config references bucket {max_bucket_no} but only {} bucket(s) provided",
        bucket_totals.len(),
    );

    let shared_data = sync::DArc::new(genvm::rt::SharedData {
        cancellation: token,
        is_sync: args.sync,
        genvm_id: genvm_modules_interfaces::GenVMId(genvm_id),
        debug_mode: args.debug_mode,
        metrics,
        data_fees_limit: genvm::rt::fees::DataLimit::new(
            bucket_totals,
            config.fees.clone(),
            execution_data.gas_data.clone(),
        )?,
    });

    let hosts: Vec<genvm::Host> = args
        .host
        .iter()
        .enumerate()
        .map(|(id, uri)| {
            let metrics = shared_data.gep(|x| &x.metrics.hosts[id]);
            genvm::Host::connect(uri, metrics)
                .with_context(|| format!("connecting host {id} to {uri}"))
        })
        .collect::<Result<_>>()?;

    let method_hosts = execution_data.method_hosts.clone();

    let mut perm_size = 0;
    for perm in ["r", "w", "s", "c", "n"] {
        if args.permissions.contains(perm) {
            perm_size += 1;
        }
    }

    if perm_size != args.permissions.len() {
        anyhow::bail!("Invalid permissions {}", &args.permissions)
    }

    log_info!(genvm_id = genvm_id; "genvm id");

    let rt = runtime.enter();

    let supervisor = genvm::create_supervisor(
        &config,
        hosts,
        method_hosts,
        host_data,
        shared_data,
        message,
        execution_data.leader_nondet_results.clone(),
    )
    .with_context(|| format!("creating supervisor for genvm_id {genvm_id}"))?;

    std::mem::drop(rt);

    let res = runtime
        .block_on(genvm::run_with(
            execution_data,
            supervisor.clone(),
            &args.permissions,
        ))
        .with_context(|| "running genvm");

    if let Err(err) = &res {
        log_error!(error:ah = err; "error running genvm");
    }

    runtime.block_on(async {
        supervisor.modules.llm.close().await;
        supervisor.modules.web.close().await;
    });

    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    runtime.shutdown_timeout(std::time::Duration::from_millis(30));

    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    Ok(())
}
