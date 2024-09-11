use anyhow::Result;
use clap::Parser;
use genvm::vm::VMRunResult;

#[derive(clap::Parser)]
struct CliArgs {
    #[arg(long, default_value_t = String::from("${genvmRoot}/share/genvm/default-config.json"))]
    config: String,
    #[arg(long)]
    message: String,
    #[arg(long)]
    host: String,
    #[arg(long, default_value_t = false)]
    shrink_error: bool,
}

fn main() -> Result<()> {
    let args = CliArgs::parse();

    let message: genvm::MessageData = serde_json::from_str(&args.message)?;

    let host = genvm::Host::new(&args.host)?;
    let res = genvm::run_with(message, &args.config, host)?;
    let res = match (res, args.shrink_error) {
        (VMRunResult::Error(_), true) => VMRunResult::Error("".into()),
        (res, _) => res,
    };
    println!("executed with `{res:?}`");
    Ok(())
}
