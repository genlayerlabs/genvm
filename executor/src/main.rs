use anyhow::Result;
use clap::{Parser, ValueEnum};

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
struct CommandPrecompileArgs {}

#[derive(clap::Args, Debug)]
struct CommandRunArgs {
    #[arg(long, default_value_t = String::from("${genvmRoot}/share/genvm/default-config.json"))]
    config: String,
    #[arg(long)]
    message: String,
    #[arg(long)]
    host: String,
    #[clap(long, default_value_t = PrintOption::None)]
    print: PrintOption,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    Run(CommandRunArgs),
}

#[derive(clap::Parser)]
#[command(version = concat!(env!("CARGO_PKG_VERSION"), " ", env!("PROFILE")))]
struct CliArgs {
    #[command(subcommand)]
    command: Commands,
}

fn handle_run(args: CommandRunArgs) -> Result<()> {
    let message: genvm::MessageData = serde_json::from_str(&args.message)?;

    let host = genvm::Host::new(&args.host)?;

    let supervisor = genvm::create_supervisor(&args.config, host)?;

    let shared_data = {
        let supervisor = supervisor.clone();
        let Ok(sup) = supervisor.lock() else { panic!() };
        sup.shared_data.clone()
    };
    eprintln!("PTR {:?}", shared_data.should_exit.as_ptr());
    let action = move || {
        eprintln!("assigning should_exit");
        shared_data
            .should_exit
            .store(1, std::sync::atomic::Ordering::SeqCst);
    };
    unsafe {
        signal_hook::low_level::register(
            signal_hook::consts::SIGTERM | signal_hook::consts::SIGINT,
            action,
        )?;
    }

    let res = genvm::run_with(message, supervisor);
    let res: Option<String> = match (res, args.print) {
        (_, PrintOption::None) => None,
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
    // FIXME exit code?
    Ok(())
}

fn main() -> Result<()> {
    let args = CliArgs::parse();

    match args.command {
        Commands::Run(command_run_args) => handle_run(command_run_args),
    }
}
