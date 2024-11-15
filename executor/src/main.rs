use anyhow::Result;
use clap::Parser;

mod exe;

#[derive(clap::Subcommand, Debug)]
enum Commands {
    Run(exe::run::Args),
    Precompile(exe::precompile::Args),
}

#[derive(clap::Parser)]
#[command(version = concat!(env!("CARGO_PKG_VERSION"), " ", env!("PROFILE"), " ", env!("GENVM_BUILD_ID")))]
struct CliArgs {
    #[command(subcommand)]
    command: Commands,
}

fn main() -> Result<()> {
    let args = CliArgs::parse();

    match args.command {
        Commands::Run(args) => exe::run::handle(args),
        Commands::Precompile(args) => exe::precompile::handle(args),
    }
}
