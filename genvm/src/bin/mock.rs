use anyhow::{Context, Result};
use clap::Parser;
use genvm::vm::VMRunResult;

mod test_node_iface_impl {
    use serde_with::{base64::Base64, serde_as};

    use genvm::*;
    use std::{
        collections::HashMap,
        io::{stderr, Write},
        sync::Arc,
    };

    use anyhow::Result;
    use node_iface::{self};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Clone)]
    pub struct FakeAccount {
        code: Option<String>,
    }

    #[serde_as]
    #[derive(Serialize, Deserialize, Clone)]
    pub struct Config {
        storage_file_path: String,
        accounts: HashMap<String, FakeAccount>,
        message: node_iface::MessageData,
        #[serde_as(as = "Base64")]
        calldata: Vec<u8>,
    }

    pub struct TestApi {
        conf: Config,
        fake_storage: FakeStorage,
    }

    #[derive(Serialize, Deserialize)]
    pub struct FakeStorage {
        slots: HashMap<node_iface::Address, HashMap<node_iface::Address, Vec<u8>>>,
    }

    macro_rules! get_slot {
        ($to: ident, $stor:expr, $slot:ident) => {
            let mut ent0 = ($stor).slots.entry($slot.account.clone());
            let by_acc = match ent0 {
                std::collections::hash_map::Entry::Occupied(ref mut v) => v.get_mut(),
                std::collections::hash_map::Entry::Vacant(v) => v.insert(HashMap::new()),
            };
            let mut ent2 = by_acc.entry($slot.slot.clone());
            let $to = match ent2 {
                std::collections::hash_map::Entry::Occupied(ref mut v) => v.get_mut(),
                std::collections::hash_map::Entry::Vacant(v) => v.insert(Vec::new()),
            };
        };
    }

    impl node_iface::StorageApi for TestApi {
        fn storage_read(
            &mut self,
            _remaing_gas: &mut node_iface::Gas,
            slot: node_iface::StorageSlot,
            index: u32,
            buf: &mut [u8],
        ) -> Result<()> {
            let index = index as usize;
            get_slot!(slot, self.fake_storage, slot);
            if index + buf.len() > slot.len() {
                slot.resize(index + buf.len(), 0);
            }
            buf.copy_from_slice(&slot[index..index + buf.len()]);
            Ok(())
        }

        fn storage_write(
            &mut self,
            _remaing_gas: &mut node_iface::Gas,
            slot: node_iface::StorageSlot,
            index: u32,
            buf: &[u8],
        ) -> Result<()> {
            let index = index as usize;
            get_slot!(slot, self.fake_storage, slot);
            if index + buf.len() > slot.len() {
                slot.resize(index + buf.len(), 0);
            }
            slot[index..index + buf.len()].copy_from_slice(buf);
            Ok(())
        }
    }

    impl Drop for TestApi {
        fn drop(&mut self) {
            let path = std::path::Path::new(&self.conf.storage_file_path);
            let res = serde_json::to_string(&self.fake_storage)
                .map_err(|e| anyhow::Error::from(e))
                .and_then(|x| std::fs::write(path, x).map_err(Into::into));
            match res {
                Err(e) => {
                    let _ = stderr()
                        .lock()
                        .write_fmt(format_args!("Writing storage to {:#?} failed {}", path, e));
                }
                _ => {}
            }
        }
    }

    impl TestApi {
        pub fn new(conf: Config) -> Result<Self> {
            let path = std::path::Path::new(&conf.storage_file_path);
            let fake_storage = if path.exists() {
                let storage_str = String::from_utf8(std::fs::read(path)?)?;
                serde_json::from_str(&storage_str)?
            } else {
                FakeStorage {
                    slots: HashMap::new(),
                }
            };
            Ok(Self { conf, fake_storage })
        }
    }

    #[allow(unused_variables)]
    impl node_iface::InitApi for TestApi {
        fn get_initial_data(&mut self, calldata: &mut Vec<u8>) -> Result<node_iface::MessageData> {
            calldata.extend_from_slice(&self.conf.calldata);
            Ok(self.conf.message.clone())
        }

        fn get_code(&mut self, account: &node_iface::Address) -> Result<Arc<[u8]>> {
            let mut acc: String = serde_json::to_string(account)?;
            // remove ""
            acc.pop();
            acc.remove(0);

            let acc = self
                .conf
                .accounts
                .get(&acc)
                .ok_or(anyhow::anyhow!("no account"))?;
            let Some(ref code) = acc.code else {
                return Err(anyhow::anyhow!("no account"));
            };
            let code = std::fs::read(code)?;
            Ok(Arc::from(code))
        }
    }
}

impl genvm::RequiredApis for test_node_iface_impl::TestApi {}

#[derive(clap::Parser)]
struct CliArgs {
    #[arg(long, default_value_t = String::from("${genvmRoot}/share/genvm/default-config.json"))]
    config: String,
    #[arg(long)]
    mock_config: std::path::PathBuf,
    #[arg(long, default_value_t = false)]
    shrink_error: bool,
}

fn main() -> Result<()> {
    let args = CliArgs::parse();
    let mock_conf = std::fs::read_to_string(&args.mock_config)?;
    let mock_conf = serde_json::from_str(&mock_conf).with_context(|| "parsing mock config")?;

    let api = Box::new(test_node_iface_impl::TestApi::new(mock_conf)?);
    let res = genvm::run_with_api(api, &args.config)?;
    let res = match (res, args.shrink_error) {
        (VMRunResult::Error(_), true) => VMRunResult::Error("".into()),
        (res, _) => res,
    };
    println!("executed with `{res:?}`");
    Ok(())
}
