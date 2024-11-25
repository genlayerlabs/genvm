use anyhow::{Context, Result};
use clap::ValueEnum;

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
pub struct Args {
    #[arg(long, default_value_t = String::from("${genvmRoot}/share/genvm/default-config.json"))]
    config: String,
    #[arg(long)]
    message: String,
    #[arg(long)]
    host: String,
    #[clap(long, default_value_t = PrintOption::None)]
    print: PrintOption,
}

pub fn handle(args: Args, log_fd: std::os::fd::RawFd) -> Result<()> {
    let message: genvm::MessageData = serde_json::from_str(&args.message)?;

    let host = genvm::Host::new(&args.host)?;

    let supervisor = genvm::create_supervisor(&args.config, host, log_fd)
        .with_context(|| "creating supervisor")?;

    let shared_data = {
        let supervisor = supervisor.clone();
        let Ok(sup) = supervisor.lock() else { panic!() };
        sup.shared_data.clone()
    };
    let action = move || {
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

    let res = genvm::run_with(message, supervisor).with_context(|| "running genvm");
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
