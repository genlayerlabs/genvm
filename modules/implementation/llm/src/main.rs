use anyhow::Result;
use clap::Parser;

#[cfg(not(debug_assertions))]
fn default_log_level() -> log::LevelFilter {
    log::LevelFilter::Info
}

#[cfg(debug_assertions)]
fn default_log_level() -> log::LevelFilter {
    log::LevelFilter::Trace
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

#[derive(clap::Parser)]
#[command(version = concat!(env!("CARGO_PKG_VERSION"), " ", env!("PROFILE"), " ", env!("GENVM_BUILD_ID")))]
#[clap(rename_all = "kebab_case")]
struct CliArgs {
    #[arg(long, default_value_t = default_log_level())]
    log_level: log::LevelFilter,

    #[arg(long, default_value = "tracing*,polling*")]
    log_disable: String,

    #[arg(long, default_value_t = String::from("${genvmRoot}/etc/genvm-module-llm.yaml"))]
    config: String,
}

fn main() -> Result<()> {
    let args = CliArgs::parse();

    structured_logger::Builder::with_level(args.log_level.as_str())
        .with_default_writer(structured_logger::json::new_writer(std::io::stdout()))
        .with_target_writer(&args.log_disable, Box::new(NullWiriter))
        .init();

    Ok(())
}
