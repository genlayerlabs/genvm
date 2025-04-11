use std::{collections::HashMap, os::fd::FromRawFd};

use anyhow::{Context, Result};
use clap::Parser;
use genvm::config;

mod exe;

#[derive(clap::Subcommand, Debug)]
enum Commands {
    Run(exe::run::Args),
    Precompile(exe::precompile::Args),
}

#[derive(clap::Parser)]
#[command(version = genvm_common::VERSION)]
#[clap(rename_all = "kebab_case")]
struct CliArgs {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, default_value_t = String::from("${genvmRoot}/etc/genvm.yaml"))]
    config: String,

    #[arg(long, default_value = "2")]
    log_fd: std::os::fd::RawFd,
}

fn main() -> Result<()> {
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
    let config: config::Config = serde_yaml::from_value(config)?;

    config.base.setup_logging(log_file)?;

    match args.command {
        Commands::Run(args) => exe::run::handle(args, config),
        Commands::Precompile(args) => exe::precompile::handle(args, config),
    }
}
