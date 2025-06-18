pub(crate) mod common;
pub(crate) mod scripting;

mod llm;
mod web;

use anyhow::Result;
use clap::Parser;
use genvm_common::log_error;

#[derive(clap::Subcommand, Debug)]
enum Commands {
    Web(web::CliArgs),
    Llm(llm::CliArgsRun),
    LlmCheck(llm::CliArgsCheck),
}

#[derive(clap::Parser)]
#[command(version = genvm_common::VERSION)]
#[clap(rename_all = "kebab_case")]
struct CliArgs {
    #[command(subcommand)]
    command: Commands,
}

fn main() -> Result<()> {
    let args = CliArgs::parse();

    match args.command {
        Commands::Web(a) => web::entrypoint(a),
        Commands::Llm(a) => llm::entrypoint_run(a),
        Commands::LlmCheck(a) => llm::entrypoint_check(a),
    }
    .inspect_err(|e| {
        log_error!(
            error:ah = e;
            "error in main"
        );
    })
}
