use anyhow::{Context, Result};
use core::str;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{io::Read, sync::Arc};
use zip::ZipArchive;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum WasmMode {
    Det,
    Nondet,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum RunnerJsonInitAction {
    MapFile {
        to: String,
        file: String,
    },
    AddEnv {
        name: String,
        val: String,
    },
    SetArgs(Vec<String>),
    Depends(String),
    LinkWasm(String),
    StartWasm(String),

    When {
        cond: WasmMode,
        action: Box<RunnerJsonInitAction>,
    },
    Seq(Vec<RunnerJsonInitAction>),

    With {
        runner: String,
        action: Box<RunnerJsonInitAction>,
    },
}

#[derive(Clone, Debug)]
pub enum InitAction {
    MapFile {
        to: String,
        file: symbol_table::GlobalSymbol,
    },
    AddEnv {
        name: String,
        val: String,
    },
    SetArgs(Vec<String>),
    Depends(symbol_table::GlobalSymbol),
    LinkWasm(symbol_table::GlobalSymbol),
    StartWasm(symbol_table::GlobalSymbol),

    When {
        cond: WasmMode,
        action: Box<InitAction>,
    },
    Seq(Vec<InitAction>),

    With {
        runner: symbol_table::GlobalSymbol,
        action: Box<InitAction>,
    },
}

pub struct ZipCache {
    zip: ZipArchive<std::io::Cursor<Arc<[u8]>>>,
    id: symbol_table::GlobalSymbol,

    files: std::collections::HashMap<symbol_table::GlobalSymbol, Arc<[u8]>>,
    all_files: Option<Arc<[symbol_table::GlobalSymbol]>>,
    extracted_actions: Option<Arc<InitAction>>,
}

impl ZipCache {
    pub fn runner_id(&self) -> symbol_table::GlobalSymbol {
        self.id
    }

    pub fn new(
        zip: ZipArchive<std::io::Cursor<Arc<[u8]>>>,
        id: symbol_table::GlobalSymbol,
    ) -> Self {
        Self {
            zip,
            id,
            files: std::collections::HashMap::new(),
            all_files: None,
            extracted_actions: None,
        }
    }

    pub fn get_actions(&mut self) -> Result<Arc<InitAction>> {
        if self.extracted_actions.is_none() {
            use symbol_table::GlobalSymbol;
            let contents = self.get_file(symbol_table::static_symbol!("runner.json"))?;
            let as_init: RunnerJsonInitAction = serde_json::from_str(&str::from_utf8(&contents)?)?;

            self.extracted_actions = Some(Arc::new(transform(as_init)));
        }

        match &self.extracted_actions {
            Some(v) => return Ok(v.clone()),
            _ => unreachable!(),
        }
    }

    pub fn get_file(&mut self, name: symbol_table::GlobalSymbol) -> Result<Arc<[u8]>> {
        match self.files.entry(name) {
            std::collections::hash_map::Entry::Occupied(occupied_entry) => {
                Ok(occupied_entry.get().clone())
            }
            std::collections::hash_map::Entry::Vacant(vacant_entry) => {
                let mut file = self
                    .zip
                    .by_name(name.as_str())
                    .with_context(|| format!("fetching {name}"))?;
                let mut buf = Vec::new();
                file.read_to_end(&mut buf)?;
                let buf: Arc<[u8]> = Arc::from(buf);
                vacant_entry.insert(buf.clone());
                Ok(buf)
            }
        }
    }

    pub fn get_all_names(&mut self) -> Result<Arc<[symbol_table::GlobalSymbol]>> {
        if self.all_files.is_none() {
            let v: Vec<symbol_table::GlobalSymbol> = self
                .zip
                .file_names()
                .filter(|f| !f.ends_with("/"))
                .map(|f| symbol_table::GlobalSymbol::from(f))
                .sorted()
                .collect();
            self.all_files = Some(Arc::from(v));
        }

        match &self.all_files {
            Some(v) => Ok(v.clone()),
            None => unreachable!(),
        }
    }
}

pub struct RunnerReaderCache {
    cache: std::collections::HashMap<symbol_table::GlobalSymbol, ZipCache>,
    path: std::path::PathBuf,
}

impl RunnerReaderCache {
    pub fn new() -> Result<Self> {
        let runners_path = path()?;
        if !runners_path.exists() {
            anyhow::bail!("path {:#?} doesn't exist", &runners_path);
        }

        Ok(Self {
            cache: std::collections::HashMap::new(),
            path: runners_path,
        })
    }

    pub fn path(&self) -> &std::path::Path {
        std::path::Path::new(&self.path)
    }

    pub fn get_or_create<'a>(
        &'a mut self,
        name: symbol_table::GlobalSymbol,
        arch_provider: impl FnOnce() -> Result<ZipArchive<std::io::Cursor<Arc<[u8]>>>>,
    ) -> Result<&'a mut ZipCache> {
        match self.cache.entry(name) {
            std::collections::hash_map::Entry::Occupied(occupied_entry) => {
                Ok(occupied_entry.into_mut())
            }
            std::collections::hash_map::Entry::Vacant(vacant_entry) => {
                let to_insert = ZipCache::new(arch_provider()?, name);
                Ok(vacant_entry.insert(to_insert))
            }
        }
    }

    pub fn get_unsafe<'a>(&'a mut self, key: symbol_table::GlobalSymbol) -> &'a mut ZipCache {
        self.cache.get_mut(&key).unwrap()
    }
}

pub fn verify_runner<'a>(runner_id: &'a str) -> Result<(&'a str, &'a str)> {
    let (runner_id, runner_hash) = runner_id
        .split(":")
        .collect_tuple()
        .ok_or(anyhow::anyhow!("expected <name>:<hash>"))?;

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

pub fn transform(from: RunnerJsonInitAction) -> InitAction {
    match from {
        RunnerJsonInitAction::MapFile { to, file } => InitAction::MapFile {
            to,
            file: symbol_table::GlobalSymbol::from(&file),
        },
        RunnerJsonInitAction::AddEnv { name, val } => InitAction::AddEnv { name, val },
        RunnerJsonInitAction::SetArgs(vec) => InitAction::SetArgs(vec),
        RunnerJsonInitAction::Depends(dep) => {
            InitAction::Depends(symbol_table::GlobalSymbol::from(dep))
        }
        RunnerJsonInitAction::LinkWasm(fpath) => {
            InitAction::LinkWasm(symbol_table::GlobalSymbol::from(fpath))
        }
        RunnerJsonInitAction::StartWasm(fpath) => {
            InitAction::StartWasm(symbol_table::GlobalSymbol::from(fpath))
        }
        RunnerJsonInitAction::When { cond, action } => InitAction::When {
            cond,
            action: Box::new(transform(*action)),
        },
        RunnerJsonInitAction::Seq(vec) => InitAction::Seq(vec.into_iter().map(transform).collect()),
        RunnerJsonInitAction::With { runner, action } => InitAction::With {
            runner: symbol_table::GlobalSymbol::from(runner),
            action: Box::new(transform(*action)),
        },
    }
}

pub fn path() -> Result<std::path::PathBuf> {
    let mut runners_path = std::env::current_exe()?;
    runners_path.pop();
    runners_path.pop();
    runners_path.push("share");
    runners_path.push("genvm");
    runners_path.push("runners");
    Ok(runners_path)
}
