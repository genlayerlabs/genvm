pub mod actions;
pub mod cache;

mod parse;
mod ustar;
use genvm_common::*;
use itertools::Itertools;

pub use ustar::Archive;

pub use parse::parse;

pub use actions::*;

use anyhow::Context as _;
use std::{str::FromStr as _, sync::Arc};

use crate::{public_abi, rt};

pub fn append_runner_subpath(id: &str, hash: &str, path: &mut std::path::PathBuf) {
    path.push(id);
    path.push(&hash[..2]);
    path.push(&hash[2..]);
}

pub fn get_runner_of_contract(address: calldata::Address) -> symbol_table::GlobalSymbol {
    let mut contract_id = String::from("on_chain:0x");
    contract_id.push_str(&hex::encode(address.raw()));

    symbol_table::GlobalSymbol::from(contract_id)
}

pub struct ArchiveCache {
    pub(super) id: symbol_table::GlobalSymbol,
    pub(super) files: Archive,
    pub(super) actions: tokio::sync::OnceCell<Arc<InitAction>>,
}

impl ArchiveCache {
    pub fn runner_id(&self) -> symbol_table::GlobalSymbol {
        self.id
    }

    pub fn new(id: symbol_table::GlobalSymbol, files: Archive) -> Self {
        Self {
            id,
            files,
            actions: tokio::sync::OnceCell::new(),
        }
    }

    pub fn get_version(&self) -> anyhow::Result<genvm_common::version::Version> {
        let contents = match self.get_file("version") {
            Ok(contents) => contents,
            Err(e) => {
                log_warn!(error:ah = e, runner = self.id; "failed to read version file for runner, using default");
                util::SharedBytes::from(public_abi::ABSENT_VERSION.as_bytes())
            }
        };

        let contents = std::str::from_utf8(contents.as_ref())
            .with_context(|| format!("casting version to string {}", self.id))?;

        let version = version::Version::from_str(contents)?;

        log_trace!(from = contents, to = version; "version parsed");

        Ok(version)
    }

    pub async fn get_actions(&self) -> anyhow::Result<Arc<InitAction>> {
        self.actions
            .get_or_try_init(|| async {
                let contents = self.get_file("runner.json")?;

                let as_init: InitAction =
                    serde_json::from_str(std::str::from_utf8(contents.as_ref())?)?;

                Ok(Arc::new(as_init))
            })
            .await
            .map(Clone::clone)
    }

    pub fn get_file(&self, name: &str) -> anyhow::Result<util::SharedBytes> {
        let contents = self
            .files
            .data
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("no file {}", name))
            .with_context(|| format!("reading runner {}", self.id))?;
        Ok(contents.clone())
    }
}

pub fn verify_runner(runner_id: &str) -> Option<(&str, &str)> {
    let (runner_id, runner_hash) = runner_id.split(":").collect_tuple()?;

    for c in runner_id.chars() {
        if !c.is_ascii_alphanumeric() && c != '-' && c != '_' {
            log_warn!("character `{c}` is not allowed in runner id");

            return None;
        }
    }

    for c in runner_hash.chars() {
        if !c.is_ascii_alphanumeric() && c != '-' && c != '_' && c != '=' {
            log_warn!("character `{c}` is not allowed in runner hash");

            return None;
        }
    }
    Some((runner_id, runner_hash))
}
