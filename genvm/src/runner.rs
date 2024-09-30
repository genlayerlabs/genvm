use anyhow::{Context, Result};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{io::Read, sync::Arc};
use zip::ZipArchive;

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

fn empty_vec<T>() -> Vec<T> {
    return Vec::new();
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RunnerJsonFile {
    #[serde(default = "empty_vec")]
    pub pre_actions: Vec<RunnerJsonInitAction>,
    #[serde(default = "empty_vec")]
    pub depends: Vec<Arc<str>>,
    #[serde(default = "empty_vec")]
    pub actions: Vec<RunnerJsonInitAction>,
}

struct RunnerReaderCacheEntry {
    pre_actions: Vec<InitAction>,
    depends: Arc<Vec<Arc<str>>>,
    actions: Vec<InitAction>,
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
    fn transform_actions<R>(
        zip_file: &mut ZipArchive<R>,
        path_prefix: &str,
        dest: &mut Vec<InitAction>,
        from: Vec<RunnerJsonInitAction>,
    ) -> Result<()>
    where
        R: std::io::Read + std::io::Seek,
    {
        let all_files: Vec<String> = zip_file.file_names().filter(|f| !f.ends_with("/")).map(|f| f.into()).sorted().collect();
        for a in from {
            match a {
                RunnerJsonInitAction::MapFile { to, file } => {
                    let mut read_to_buf = |name: &str, to: String| -> Result<()> {
                        let mut buf = Vec::new();
                        zip_file.by_name(&name).with_context(|| format!("reading {name}"))?.read_to_end(&mut buf)?;
                        dest.push(InitAction::MapFile {
                            to,
                            contents: Arc::from(buf),
                        });
                        Ok(())
                    };
                    if file.ends_with("/") {
                        for f in all_files.iter().filter(|f| {
                            !f.ends_with("/") && f.starts_with(&file)
                        }) {
                            let mut name = to.clone();
                            if !name.ends_with("/") {
                                name.push_str("/");
                            }
                            name.push_str(&f[file.len()..]);
                            read_to_buf(&f, name)?;
                        }
                    } else {
                        read_to_buf(&file, to)?;
                    }
                }
                RunnerJsonInitAction::MapCode { to } => dest.push(InitAction::MapCode { to }),
                RunnerJsonInitAction::AddEnv { name, val } => {
                    dest.push(InitAction::AddEnv { name, val })
                }
                RunnerJsonInitAction::SetArgs { args } => dest.push(InitAction::SetArgs { args }),
                RunnerJsonInitAction::LinkWasm { file } => {
                    let mut buf = Vec::new();
                    zip_file.by_name(&file).with_context(|| format!("linking {file}"))?.read_to_end(&mut buf)?;
                    dest.push(InitAction::LinkWasm {
                        contents: Arc::from(buf),
                        debug_path: format!("{}/{}", path_prefix, file),
                    })
                }
                RunnerJsonInitAction::StartWasm { file } => {
                    let mut buf = Vec::new();
                    zip_file.by_name(&file).with_context(|| format!("starting {file}"))?.read_to_end(&mut buf)?;
                    dest.push(InitAction::StartWasm {
                        contents: Arc::from(buf),
                        debug_path: format!("{}/{}", path_prefix, file),
                    })
                }
            }
        }

        Ok(())
    }

    fn make_from_arch<R>(
        path_prefix: &str,
        zip_file: &mut ZipArchive<R>,
    ) -> Result<RunnerReaderCacheEntry>
    where
        R: std::io::Read + std::io::Seek,
    {
        let mut ret = RunnerReaderCacheEntry {
            pre_actions: Vec::new(),
            depends: Arc::new(Vec::new()),
            actions: Vec::new(),
        };

        let runner = std::io::read_to_string(zip_file.by_name("runner.json")?)?;
        let runner: RunnerJsonFile = serde_json::from_str(&runner)?;

        RunnerReaderCacheEntry::transform_actions(
            zip_file,
            path_prefix,
            &mut ret.pre_actions,
            runner.pre_actions,
        ).with_context(|| format!("pre_actions from {}", &path_prefix))?;
        ret.depends = Arc::new(runner.depends);
        RunnerReaderCacheEntry::transform_actions(
            zip_file,
            path_prefix,
            &mut ret.actions,
            runner.actions,
        ).with_context(|| format!("actions from {}", &path_prefix))?;

        Ok(ret)
    }

    fn make(runner_id: Arc<str>, mut path: std::path::PathBuf) -> Result<RunnerReaderCacheEntry> {
        let res: Vec<&str> = runner_id.split(":").collect();
        if res.len() != 2 {
            anyhow::bail!(
                "invalid runner, expected <RUNNER>:<VERSION> ; got {:?}",
                res
            );
        }

        path.push(res[0]);
        let mut fname = res[1].to_owned();
        fname.push_str(".zip");
        path.push(fname);
        let file = std::fs::File::open(&path).with_context(|| format!("reading {:?}", path))?;
        let mut zip_file = zip::ZipArchive::new(file)?;

        RunnerReaderCacheEntry::make_from_arch(&runner_id, &mut zip_file)
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

    fn post_load(
        &mut self,
        runner_id: Arc<str>,
        cache: &mut RunnerReaderCache,
        cache_entry: &RunnerReaderCacheEntry,
    ) -> Result<()> {
        let process_actions = |zelf: &mut Self, actions: &[InitAction]| {
            for ref act in actions {
                if let Some(ref was_start) = zelf.was_start {
                    anyhow::bail!(
                        "detected action after start: start from {} new action from {}",
                        was_start,
                        runner_id
                    )
                }
                match act {
                    InitAction::SetArgs { .. } => {
                        zelf.was_args = match &zelf.was_args {
                            Some(x) => {
                                anyhow::bail!("args were set twice: old {} new {}", x, runner_id)
                            }
                            None => Some(runner_id.to_owned()),
                        };
                    }
                    InitAction::StartWasm { .. } => {
                        zelf.was_start = match &zelf.was_start {
                            Some(x) => {
                                anyhow::bail!("start called twice: old {} new {}", x, runner_id)
                            }
                            None => Some(runner_id.to_owned()),
                        };
                    }
                    _ => {}
                }
                zelf.actions.push((*act).clone());
            }
            Ok(())
        };

        process_actions(self, &cache_entry.pre_actions)?;
        for c in &*cache_entry.depends.clone() {
            self.append(c.clone(), cache)?;
        }
        process_actions(self, &cache_entry.actions)?;

        Ok(())
    }

    pub fn append_archieve<R>(
        &mut self,
        path_prefix: &str,
        archieve: &mut zip::ZipArchive<R>,
        cache: &mut RunnerReaderCache,
    ) -> Result<()>
    where
        R: std::io::Read + std::io::Seek,
    {
        let cache_entry = RunnerReaderCacheEntry::make_from_arch(path_prefix, archieve)?;

        self.post_load(Arc::from(path_prefix), cache, &cache_entry)
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

        self.post_load(runner_id, cache, &cache_entry)
    }

    pub(crate) fn get(self) -> Result<Vec<InitAction>> {
        Ok(self.actions)
    }
}
