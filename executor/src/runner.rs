use anyhow::{Context, Result};
use core::str;
use genvm_common::{log_trace, log_warn};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::Arc};
use symbol_table::GlobalSymbol;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum WasmMode {
    Det,
    Nondet,
}

pub fn get_id_of_contract(address: calldata::Address) -> GlobalSymbol {
    let mut contract_id = String::from("on_chain:0x");
    contract_id.push_str(&hex::encode(address.raw()));

    GlobalSymbol::from(contract_id)
}

struct GlobalSymbolDeserializeVisitor;

impl serde::de::Visitor<'_> for GlobalSymbolDeserializeVisitor {
    type Value = symbol_table::GlobalSymbol;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("expected string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(symbol_table::GlobalSymbol::from(value))
    }
}

fn global_symbol_deserialize<'de, D>(d: D) -> Result<symbol_table::GlobalSymbol, D::Error>
where
    D: serde::Deserializer<'de>,
{
    d.deserialize_str(GlobalSymbolDeserializeVisitor)
}

#[derive(Clone, Debug, Deserialize)]
pub enum InitAction {
    MapFile {
        to: Arc<str>,
        file: Arc<str>,
    },
    AddEnv {
        name: String,
        val: String,
    },
    SetArgs(Vec<String>),
    Depends(#[serde(deserialize_with = "global_symbol_deserialize")] symbol_table::GlobalSymbol),
    LinkWasm(Arc<str>),
    StartWasm(Arc<str>),

    When {
        cond: WasmMode,
        action: Box<InitAction>,
    },
    Seq(Vec<InitAction>),

    With {
        #[serde(deserialize_with = "global_symbol_deserialize")]
        runner: symbol_table::GlobalSymbol,
        action: Box<InitAction>,
    },
}

use crate::{calldata, errors::ContractError, memlimiter, public_abi, ustar::*};

pub struct ZipCache {
    id: symbol_table::GlobalSymbol,

    actions: Option<Arc<InitAction>>,

    pub files: Archive,
}

impl ZipCache {
    pub fn runner_id(&self) -> symbol_table::GlobalSymbol {
        self.id
    }

    pub fn new(id: symbol_table::GlobalSymbol, files: Archive) -> Self {
        Self {
            id,
            files,
            actions: None,
        }
    }

    pub fn get_version(&self) -> Result<genvm_common::version::Version> {
        let contents = match self.get_file("version") {
            Ok(contents) => contents,
            Err(e) => {
                log_warn!(error:ah = e, runner = self.id; "failed to read version file for runner, using default");
                SharedBytes::from(public_abi::ABSENT_VERSION.as_bytes())
            }
        };

        let contents = str::from_utf8(contents.as_ref())
            .with_context(|| format!("casting version to string {}", self.id))?;

        let version = genvm_common::version::Version::from_str(contents)?;

        log_trace!(from = contents, to = version; "version parsed");

        Ok(version)
    }

    pub fn get_actions(&mut self) -> Result<Arc<InitAction>> {
        if self.actions.is_none() {
            let contents = self.get_file("runner.json")?;

            let as_init: InitAction = serde_json::from_str(str::from_utf8(contents.as_ref())?)?;

            self.actions = Some(Arc::new(as_init));
        }

        match &self.actions {
            Some(v) => Ok(v.clone()),
            _ => unreachable!(),
        }
    }

    pub fn get_file(&self, name: &str) -> Result<SharedBytes> {
        let contents = self
            .files
            .data
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("no file {}", name))
            .with_context(|| format!("reading runner {}", self.id))?;
        Ok(contents.clone())
    }
}

pub struct RunnerReaderCache {
    cache: std::collections::HashMap<symbol_table::GlobalSymbol, ZipCache>,
    path: Arc<std::path::Path>,
}

impl RunnerReaderCache {
    pub fn new() -> Result<Self> {
        let runners_path: Arc<std::path::Path> = Arc::from(std::path::Path::new(&path()?));
        if !runners_path.exists() {
            anyhow::bail!("path {:#?} doesn't exist", &runners_path);
        }

        Ok(Self {
            cache: std::collections::HashMap::new(),
            path: runners_path,
        })
    }

    pub fn path(&self) -> &Arc<std::path::Path> {
        &self.path
    }

    pub fn get_or_create(
        &mut self,
        name: symbol_table::GlobalSymbol,
        arch_provider: impl FnOnce() -> Result<Archive>,
        limiter: &memlimiter::Limiter,
    ) -> Result<&mut ZipCache> {
        match self.cache.entry(name) {
            std::collections::hash_map::Entry::Occupied(occupied_entry) => {
                if !limiter.consume(occupied_entry.get().files.total_size) {
                    return Err(ContractError::oom(None).into());
                }
                Ok(occupied_entry.into_mut())
            }
            std::collections::hash_map::Entry::Vacant(vacant_entry) => {
                let to_insert = ZipCache::new(name, arch_provider()?);
                Ok(vacant_entry.insert(to_insert))
            }
        }
    }

    pub fn get_unsafe(&mut self, key: symbol_table::GlobalSymbol) -> &mut ZipCache {
        self.cache.get_mut(&key).unwrap()
    }
}

pub fn verify_runner(runner_id: &str) -> Result<(&str, &str)> {
    let (runner_id, runner_hash) = runner_id
        .split(":")
        .collect_tuple()
        .ok_or_else(|| anyhow::anyhow!("expected <name>:<hash>"))?;

    for c in runner_id.chars() {
        if !c.is_ascii_alphanumeric() && c != '-' && c != '_' {
            anyhow::bail!("character `{c}` is not allowed in runner id");
        }
    }

    for c in runner_hash.chars() {
        if !c.is_ascii_alphanumeric() && c != '-' && c != '_' && c != '=' {
            anyhow::bail!("character `{c}` is not allowed in runner hash");
        }
    }
    Ok((runner_id, runner_hash))
}

pub fn path() -> Result<std::path::PathBuf> {
    let mut runners_path = std::env::current_exe()?;
    runners_path.pop();
    runners_path.pop();
    runners_path.push("share");
    runners_path.push("lib");
    runners_path.push("genvm");
    runners_path.push("runners");
    Ok(runners_path)
}
