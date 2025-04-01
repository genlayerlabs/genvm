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

    #[arg(long, default_value_t = String::from("${genvmRoot}/etc/genvm-module-web.yaml"))]
    config: String,
}

pub struct Config {
    bind: String,

    extra_tld: Vec<Box<str>>,
    always_allow_hosts: Vec<Box<str>>,
}

fn main() -> Result<()> {
    let args = CliArgs::parse();

    structured_logger::Builder::with_level(args.log_level.as_str())
        .with_default_writer(structured_logger::json::new_writer(std::io::stdout()))
        .with_target_writer(&args.log_disable, Box::new(NullWiriter))
        .init();

    let mut root_path = std::env::current_exe().with_context(|| "getting current exe")?;
    root_path.pop();
    root_path.pop();
    let root_path = root_path
        .into_os_string()
        .into_string()
        .map_err(|e| anyhow::anyhow!("can't convert path to string `{e:?}`"))?;

    let vars: HashMap<String, String> = HashMap::from([("genvmRoot".into(), root_path)]);

    let config = genvm_common::load_config(&vars, args.config).with_context(|| "loading config")?;
    let config: ConfigSchema = serde_yaml::from_value(config)?;

    Ok(())
}
