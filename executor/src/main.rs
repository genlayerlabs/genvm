use std::os::fd::FromRawFd;

use anyhow::Result;
use clap::Parser;

mod exe;

#[derive(clap::Subcommand, Debug)]
enum Commands {
    Run(exe::run::Args),
    Precompile(exe::precompile::Args),
}

#[cfg(not(debug_assertions))]
fn default_log_level() -> log::LevelFilter {
    log::LevelFilter::Info
}

#[cfg(debug_assertions)]
fn default_log_level() -> log::LevelFilter {
    log::LevelFilter::Trace
}

#[derive(clap::Parser)]
#[command(version = concat!(env!("CARGO_PKG_VERSION"), " ", env!("PROFILE"), " ", env!("GENVM_BUILD_ID")))]
#[clap(rename_all = "kebab_case")]
struct CliArgs {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, default_value_t = default_log_level())]
    log_level: log::LevelFilter,

    #[arg(long, default_value = "wasmtime*,cranelift*")]
    log_disable: String,

    #[arg(long, default_value = "2")]
    log_fd: std::os::fd::RawFd,
}

struct NullWiriter;

impl structured_logger::Writer for NullWiriter {
    fn write_log(
        &self,
        _value: &std::collections::BTreeMap<log::kv::Key, log::kv::Value>,
    ) -> std::result::Result<(), std::io::Error> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = CliArgs::parse();

    let log_file: Box<dyn std::io::Write + Sync + Send> = match args.log_fd {
        1 => Box::new(std::io::stdout()),
        2 => Box::new(std::io::stderr()),
        fd => {
            let log_fd = unsafe { std::os::fd::OwnedFd::from_raw_fd(fd) };
            Box::new(std::fs::File::from(log_fd))
        }
    };

    structured_logger::Builder::with_level(args.log_level.as_str())
        .with_default_writer(structured_logger::json::new_writer(log_file))
        .with_target_writer(&args.log_disable, Box::new(NullWiriter))
        .init();

    log::info!(target: "vm", method = "start", version = env!("GENVM_BUILD_ID"); "");

    match args.command {
        Commands::Run(args) => exe::run::handle(args).await,
        Commands::Precompile(args) => exe::precompile::handle(args),
    }
}
