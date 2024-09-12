use anyhow::Result;
use clap::{Parser, ValueEnum};
use genvm::vm::RunResult;

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

#[derive(clap::Parser)]
struct CliArgs {
    #[arg(long, default_value_t = String::from("${genvmRoot}/share/genvm/default-config.json"))]
    config: String,
    #[arg(long)]
    message: String,
    #[arg(long)]
    host: String,
    #[clap(long, default_value_t = PrintOption::None)]
    print: PrintOption,
}

fn main() -> Result<()> {
    let args = CliArgs::parse();

    let message: genvm::MessageData = serde_json::from_str(&args.message)?;

    let host = genvm::Host::new(&args.host)?;
    let supervisor = genvm::create_supervisor(&args.config, host)?;
    let res = genvm::run_with(message, supervisor)?;
    let res = match (res, args.print) {
        (_, PrintOption::None) => None,
        (RunResult::Error(_), PrintOption::Shrink) => Some(RunResult::Error("".into())),
        (res, _) => Some(res),
    };
    match res {
        None => {}
        Some(res) => println!("executed with `{res:?}`"),
    }
    Ok(())
}
