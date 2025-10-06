use std::{collections::HashMap, os::fd::FromRawFd};

use anyhow::{Context, Result};
use clap::Parser;
use genvm::config;
use genvm_common::logger;

mod exe;

#[derive(clap::Subcommand, Debug)]
enum Commands {
    Run(exe::run::Args),
    Precompile(exe::precompile::Args),
    ParseVersionPattern(exe::parse_version::Args),
}

#[derive(clap::Parser)]
#[command(version = genvm_common::VERSION)]
#[clap(rename_all = "kebab_case")]
struct CliArgs {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, default_value_t = String::from("${exeDir}/../config/genvm.yaml"))]
    config: String,

    #[arg(long, default_value = "2")]
    log_fd: std::os::fd::RawFd,

    #[arg(long)]
    log_level: Option<logger::Level>,
}

fn main_impl() -> Result<()> {
    let args = CliArgs::parse();

    let log_file: Box<dyn std::io::Write + Sync + Send> = match args.log_fd {
        1 => Box::new(std::io::stdout()),
        2 => Box::new(std::io::stderr()),
        fd => {
            let log_fd = unsafe { std::os::fd::OwnedFd::from_raw_fd(fd) };
            Box::new(std::fs::File::from(log_fd))
        }
    };

    let config = genvm_common::load_config(HashMap::new(), &args.config)
        .with_context(|| "loading config")?;
    let mut config: config::Config = serde_yaml::from_value(config)?;

    if let Some(log_level) = args.log_level {
        config.base.log_level = log_level;
    }

    config.base.setup_logging(log_file)?;

    match args.command {
        Commands::Run(args) => exe::run::handle(args, config),
        Commands::Precompile(args) => exe::precompile::handle(args, config),
        Commands::ParseVersionPattern(args) => exe::parse_version::handle(args, config),
    }
}

fn main() -> Result<()> {
    use std::io::Write;

    let res = main_impl();

    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    res
}
