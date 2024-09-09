use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{io::Read, sync::Arc};

#[derive(Serialize, Deserialize, Clone)]
pub(crate) enum InitAction {
    MapFile {
        to: String,
        contents: Arc<[u8]>,
    },
    MapCode {
        to: String,
    },
    AddEnv {
        name: String,
        val: String,
    },
    SetArgs {
        args: Vec<String>,
    },
    LinkWasm {
        contents: Arc<[u8]>,
        debug_path: String,
    },
    StartWasm {
        contents: Arc<[u8]>,
        debug_path: String,
    },
}

#[derive(Serialize, Deserialize, Clone)]
pub enum RunnerJsonInitAction {
    MapFile { to: String, file: String },
    MapCode { to: String },
    AddEnv { name: String, val: String },
    SetArgs { args: Vec<String> },
    LinkWasm { file: String },
    StartWasm { file: String },
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RunnerJsonFile {
    pub actions: Vec<RunnerJsonInitAction>,
    pub depends: Vec<Arc<str>>,
}

struct RunnerReaderCacheEntry {
    actions: Vec<InitAction>,
    depends: Arc<Vec<Arc<str>>>,
}

pub struct RunnerReaderCache {
    cache: std::collections::HashMap<Arc<str>, Arc<RunnerReaderCacheEntry>>,
}

impl RunnerReaderCache {
    pub fn new() -> Self {
        return Self {
            cache: std::collections::HashMap::new(),
        };
    }
}

impl RunnerReaderCacheEntry {
    fn make(runner_id: Arc<str>, mut path: std::path::PathBuf) -> Result<RunnerReaderCacheEntry> {
        let res: Vec<&str> = runner_id.split(":").collect();
        if res.len() != 2 {
            anyhow::bail!(
                "invalid runner, expected <RUNNER>:<VERSION> ; got {:?}",
                res
            );
        }
        let mut ret = RunnerReaderCacheEntry {
            actions: Vec::new(),
            depends: Arc::new(Vec::new()),
        };

        path.push(res[0]);
        let mut fname = res[1].to_owned();
        fname.push_str(".zip");
        path.push(fname);
        let file = std::fs::File::open(path)?;
        let mut zip_file = zip::ZipArchive::new(file)?;

        let runner = std::io::read_to_string(zip_file.by_name("runner.json")?)?;
        let runner: RunnerJsonFile = serde_json::from_str(&runner)?;

        ret.depends = Arc::new(runner.depends);

        for a in runner.actions {
            match a {
                RunnerJsonInitAction::MapFile { to, file } => {
                    let mut buf = Vec::new();
                    zip_file.by_name(&file)?.read_to_end(&mut buf)?;
                    ret.actions.push(InitAction::MapFile {
                        to,
                        contents: Arc::from(buf),
                    })
                }
                RunnerJsonInitAction::MapCode { to } => {
                    ret.actions.push(InitAction::MapCode { to })
                }
                RunnerJsonInitAction::AddEnv { name, val } => {
                    ret.actions.push(InitAction::AddEnv { name, val })
                }
                RunnerJsonInitAction::SetArgs { args } => {
                    ret.actions.push(InitAction::SetArgs { args })
                }
                RunnerJsonInitAction::LinkWasm { file } => {
                    let mut buf = Vec::new();
                    zip_file.by_name(&file)?.read_to_end(&mut buf)?;
                    ret.actions.push(InitAction::LinkWasm {
                        contents: Arc::from(buf),
                        debug_path: format!("{}/{}", runner_id, file),
                    })
                }
                RunnerJsonInitAction::StartWasm { file } => {
                    let mut buf = Vec::new();
                    zip_file.by_name(&file)?.read_to_end(&mut buf)?;
                    ret.actions.push(InitAction::StartWasm {
                        contents: Arc::from(buf),
                        debug_path: format!("{}/{}", runner_id, file),
                    })
                }
            }
        }
        Ok(ret)
    }
}

pub struct RunnerReader {
    runners_path: std::path::PathBuf,
    visited: std::collections::HashSet<Arc<str>>,
    actions: Vec<InitAction>,
    was_args: Option<Arc<str>>,
    was_start: Option<Arc<str>>,
}

impl RunnerReader {
    pub fn new() -> Result<RunnerReader> {
        let mut runners_path = std::env::current_exe()?;
        runners_path.pop();
        runners_path.pop();
        runners_path.push("share");
        runners_path.push("genvm");
        runners_path.push("runners");
        if !runners_path.exists() {
            anyhow::bail!("path {:#?} doesn't exist", &runners_path);
        }
        Ok(RunnerReader {
            runners_path,
            visited: std::collections::HashSet::new(),
            actions: Vec::new(),
            was_args: None,
            was_start: None,
        })
    }

    pub fn append(&mut self, runner_id: Arc<str>, cache: &mut RunnerReaderCache) -> Result<()> {
        if self.visited.contains(&runner_id) {
            return Ok(());
        }

        self.visited.insert(runner_id.clone());

        let cache_entry = cache.cache.entry(runner_id.clone());
        let cache_entry: Arc<RunnerReaderCacheEntry> = match cache_entry {
            std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::hash_map::Entry::Vacant(e) => e.insert(Arc::new(
                RunnerReaderCacheEntry::make(runner_id.clone(), self.runners_path.clone())?,
            )),
        }
        .clone();

        for c in &*cache_entry.depends.clone() {
            self.append(c.clone(), cache)?;
        }

        for act in &*cache_entry.actions {
            match act {
                InitAction::SetArgs { .. } => {
                    self.was_args = match &self.was_args {
                        Some(x) => {
                            anyhow::bail!("args were set twice: old {} new {}", x, runner_id)
                        }
                        None => Some(runner_id.to_owned()),
                    };
                }
                InitAction::StartWasm { .. } => {
                    self.was_start = match &self.was_start {
                        Some(x) => anyhow::bail!("start called twice: old {} new {}", x, runner_id),
                        None => Some(runner_id.to_owned()),
                    };
                }
                _ => {}
            }
            self.actions.push(act.clone());
        }

        Ok(())
    }

    pub fn get(self) -> Result<Vec<InitAction>> {
        Ok(self.actions)
    }
}
