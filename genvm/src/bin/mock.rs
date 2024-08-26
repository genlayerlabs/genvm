use std::{collections::HashMap, sync::LazyLock};

use anyhow::Result;
use clap::Parser;

mod test_node_iface_impl {
    use std::{collections::HashMap, sync::Arc};
    use genvm::*;

    use node_iface::{self};
    use anyhow::Result;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Clone)]
    pub enum FakeInitAction {
        MapFile { to: String, file: String },
        MapCode { to: String },
        AddEnv { name: String, val: String },
        SetArgs { args: Vec<String> },
        LinkWasm { file: String },
        StartWasm { file: String },
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct FakeAccount {
        code: Option<String>,
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct Config {
        accounts: HashMap<String, FakeAccount>,
        runners: HashMap<String, Vec<FakeInitAction>>,
        message: node_iface::MessageData,
        calldata: String
    }

    impl TryFrom<FakeInitAction> for node_iface::InitAction {
        type Error = anyhow::Error;

        fn try_from(value: FakeInitAction) -> Result<Self> {
            Ok(match value {
                FakeInitAction::MapFile { to, file } => node_iface::InitAction::MapFile { to, contents: Arc::new(std::fs::read(file)?) },
                FakeInitAction::MapCode { to } => node_iface::InitAction::MapCode { to },
                FakeInitAction::AddEnv { name, val } => node_iface::InitAction::AddEnv { name, val },
                FakeInitAction::SetArgs { args } => node_iface::InitAction::SetArgs { args },
                FakeInitAction::LinkWasm { file } => node_iface::InitAction::LinkWasm { contents: Arc::new(std::fs::read(&file)?), debug_path: Some(file), },
                FakeInitAction::StartWasm { file } => node_iface::InitAction::StartWasm { contents: Arc::new(std::fs::read(&file)?), debug_path: Some(file), },
            })
        }
    }
    pub struct TestApi {
        conf: Config,
    }

    impl TestApi {
        pub fn new(conf: Config) -> Self {
            Self {
                conf,
            }
        }
    }

    impl node_iface::RunnerApi for TestApi {
        fn get_runner(&mut self, desc: node_iface::RunnerDescription) -> anyhow::Result<Vec<node_iface::InitAction>> {
            let run = self.conf.runners.get(&desc.lang).ok_or(anyhow::anyhow!("no runner"))?;
            run.iter().map(|f| f.clone().try_into() as Result<node_iface::InitAction, _>).collect()
        }
    }

    #[allow(unused_variables)]
    impl node_iface::InitApi for TestApi {
        fn get_initial_data(&mut self) -> Result<node_iface::MessageData> {
            Ok(self.conf.message.clone())
        }

        fn get_calldata(&mut self) -> Result<String> {
            Ok(self.conf.calldata.clone())
        }

        fn get_code(&mut self, account: &node_iface::Address) -> Result<Arc<Vec<u8>>> {
            let mut acc: String = serde_json::to_string(account)?;
            // remove ""
            acc.pop();
            acc.remove(0);

            let acc = self.conf.accounts.get(&acc).ok_or(anyhow::anyhow!("no account"))?;
            let Some(ref code) = acc.code else { return Err(anyhow::anyhow!("no account")) };
            let code = std::fs::read(code)?;
            Ok(Arc::new(code))
        }
    }
}

impl genvm::RequiredApis for test_node_iface_impl::TestApi {}

#[derive(clap::Parser)]
struct CliArgs {
    #[arg(long)]
    config: std::path::PathBuf,
}

struct JsonUnfolder {
    vars: HashMap<String, String>,
}

fn replace_all<E>(
    re: &regex::Regex,
    haystack: &str,
    replacement: impl Fn(&regex::Captures) -> Result<String, E>,
) -> Result<String, E> {
    let mut new = String::with_capacity(haystack.len());
    let mut last_match = 0;
    for caps in re.captures_iter(haystack) {
        let m = caps.get(0).unwrap();
        new.push_str(&haystack[last_match..m.start()]);
        new.push_str(&replacement(&caps)?);
        last_match = m.end();
    }
    new.push_str(&haystack[last_match..]);
    Ok(new)
}

static JSON_UNFOLDER_RE: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r#"\$\{([a-zA-Z0-9_]*)\}"#).unwrap() );

impl JsonUnfolder {
    fn patch(&self, s: String) -> Result<String> {
        replace_all(&JSON_UNFOLDER_RE, &s, |r: &regex::Captures| {
            let r: &str = &r[1];
            self.vars.get(r).ok_or(anyhow::anyhow!("error")).map(|x| x.clone())
        })
    }
    fn run(&self, v: serde_json::Value) -> Result<serde_json::Value> {
        Ok(match v {
            serde_json::Value::String(s) => serde_json::Value::String(self.patch(s)?),
            serde_json::Value::Array(a) => {
                let res: Result<Vec<serde_json::Value>, _> = a.into_iter().map(|a| self.run(a)).collect();
                serde_json::Value::Array(res?)
            },
            serde_json::Value::Object(ob) => {
                let res: Result<Vec<(String, serde_json::Value)>, _> =
                    ob
                        .into_iter()
                            .map(|(k, v)| -> Result<(String, serde_json::Value)> {
                                Ok((k, self.run(v)?))
                            })
                            .collect();
                serde_json::Value::Object(serde_json::Map::from_iter(res?.into_iter()))
            },
            x => x,
        })
    }
}

fn main() -> Result<()> {
    let args = CliArgs::parse();
    let conf = std::fs::read(&args.config)?;
    let conf = String::from_utf8(conf)?;
    let conf: serde_json::Value = serde_json::from_str(&conf)?;

    let json_dir: String = std::path::Path::new(&args.config).parent().ok_or(anyhow::anyhow!("no parent"))?.to_str().ok_or(anyhow::anyhow!("to str"))?.into();
    let unfolder = JsonUnfolder {
        vars: HashMap::from([
            ("jsonDir".into(), json_dir)
        ]),
    };
    let conf = unfolder.run(conf)?;
    let conf = serde_json::from_value(conf)?;

    let node_api = Box::new(test_node_iface_impl::TestApi::new(conf));
    let res = genvm::run_with_api(node_api)?;
    println!("executed with {res:?}");
    match res {
        genvm::vm::VMRunResult::Return(r) => {
            println!("\tas utf8: {}", String::from_utf8_lossy(&r[..]));
        }
        _ => {},
    }
    Ok(())
}
