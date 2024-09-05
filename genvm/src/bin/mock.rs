use std::{collections::HashMap, sync::LazyLock};

use anyhow::{Context, Result};
use clap::Parser;
use genvm::vm::VMRunResult;

mod test_node_iface_impl {
    use genvm::plugin_loader::nondet_functions_api::Loader;
    use genvm_modules_common::interfaces::nondet_functions_api;
    use serde_with::{serde_as, base64::Base64};

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

    #[serde_as]
    #[derive(Serialize, Deserialize, Clone)]
    pub struct Config {
        accounts: HashMap<String, FakeAccount>,
        runners: HashMap<String, Vec<FakeInitAction>>,
        message: node_iface::MessageData,
        #[serde_as(as = "Base64")]
        calldata: Vec<u8>,
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
        nondet_meths: Box<dyn nondet_functions_api::Trait>
    }

    impl TestApi {
        pub fn new(conf: Config) -> Result<Self> {
            let dflt_path = genvm::plugin_loader::default_plugin_path()?;
            let nondet_meths = nondet_functions_api::Methods::load_from_lib(&dflt_path, "nondet-funcs")?;
            Ok(Self {
                conf,
                nondet_meths,
            })
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

        fn get_calldata(&mut self) -> Result<Vec<u8>> {
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

    impl nondet_functions_api::Trait for TestApi {
        fn get_webpage(&mut self,gas: &mut u64,config: *const u8,url: *const u8) -> genvm_modules_common::interfaces::CStrResult {
            return self.nondet_meths.get_webpage(gas, config, url);
        }

        fn call_llm(&mut self,gas: &mut u64,config: *const u8,data: *const u8) -> genvm_modules_common::interfaces::CStrResult {
            return self.nondet_meths.get_webpage(gas, config, data);
        }
    }
}

impl genvm::RequiredApis for test_node_iface_impl::TestApi {}

#[derive(clap::Parser)]
struct CliArgs {
    #[arg(long)]
    config: std::path::PathBuf,
    #[arg(long, default_value_t = false)]
    shrink_error: bool,
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
    let conf: serde_json::Value = serde_json::from_str(&conf).with_context(|| "parsing config to raw json")?;

    let json_dir: String = std::path::Path::new(&args.config).parent().ok_or(anyhow::anyhow!("no parent"))?.to_str().ok_or(anyhow::anyhow!("to str"))?.into();
    let artifacts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../build/out").to_str().ok_or(anyhow::anyhow!("to str"))?.into();
    let mut unfolder = JsonUnfolder {
        vars: HashMap::from([
            ("jsonDir".into(), json_dir),
            ("artifacts".into(), artifacts)
        ]),
    };
     conf.get("vars").and_then(|x| x.as_object()).map(|x| -> Result<()> {
        for (k, v) in x{
            unfolder.vars.insert(k.clone(), String::from(v.as_str().ok_or(anyhow::anyhow!("invalid var value for {}", k))?));
        }
        Ok(())
    }).unwrap_or(Ok(()))?;
    let conf = unfolder.run(conf)?;
    let conf = serde_json::from_value(conf).with_context(|| "parsing config")?;

    let node_api = Box::new(test_node_iface_impl::TestApi::new(conf)?);
    let res = genvm::run_with_api(node_api)?;
    let res = match (res, args.shrink_error) {
        (VMRunResult::Error(_), true) => VMRunResult::Error("".into()),
        (res, _) => res,
    };
    println!("executed with `{res:?}`");
    Ok(())
}
