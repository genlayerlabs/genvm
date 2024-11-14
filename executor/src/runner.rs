use anyhow::{Context, Result};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read, sync::Arc};
use zip::ZipArchive;

#[derive(Serialize, Deserialize, Clone)]
pub enum InitActionTrivial {
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
    SetArgs(Vec<String>),
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
pub enum InitAction {
    Trivial(InitActionTrivial),
    When {
        cond: WasmMode,
        act: Box<InitAction>,
    },
    Seq(Vec<InitAction>),
    Once(Arc<str>, Box<InitAction>),
}

#[derive(Serialize, Deserialize, Clone)]
enum InitActionDependable {
    Trivial(InitActionTrivial),
    When {
        cond: WasmMode,
        act: Box<InitActionDependable>,
    },
    Seq(Vec<InitActionDependable>),
    Depends(Arc<str>),
}

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
    MapCode {
        to: String,
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

struct RunnerReaderCacheEntry {
    action: Arc<InitActionDependable>,
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
    fn transform_action_impl<R>(
        zip_file: &mut ZipArchive<R>,
        path_prefix: &str,
        all_files: &Vec<String>,
        action: RunnerJsonInitAction,
        runners_path: &std::path::PathBuf,
    ) -> Result<InitActionDependable>
    where
        R: std::io::Read + std::io::Seek,
    {
        let mut rec = |action: RunnerJsonInitAction| -> Result<InitActionDependable> {
            RunnerReaderCacheEntry::transform_action_impl(
                zip_file,
                path_prefix,
                all_files,
                action,
                runners_path,
            )
        };
        match action {
            RunnerJsonInitAction::MapFile { to, file } => {
                let mut read_to_buf = |name: &str, to: String| -> Result<InitActionDependable> {
                    let mut buf = Vec::new();
                    zip_file
                        .by_name(&name)
                        .with_context(|| format!("reading {name}"))?
                        .read_to_end(&mut buf)?;
                    Ok(InitActionDependable::Trivial(InitActionTrivial::MapFile {
                        to,
                        contents: Arc::from(buf),
                    }))
                };
                if file.ends_with("/") {
                    let mut ret = Vec::new();
                    for f in all_files
                        .iter()
                        .filter(|f| !f.ends_with("/") && f.starts_with(&file))
                    {
                        let mut name = to.clone();
                        if !name.ends_with("/") {
                            name.push_str("/");
                        }
                        name.push_str(&f[file.len()..]);
                        ret.push(read_to_buf(&f, name)?);
                    }
                    if ret.len() == 1 {
                        for x in ret.into_iter() {
                            return Ok(x);
                        }
                        unreachable!()
                    }
                    Ok(InitActionDependable::Seq(ret))
                } else {
                    read_to_buf(&file, to)
                }
            }
            RunnerJsonInitAction::MapCode { to } => {
                Ok(InitActionDependable::Trivial(InitActionTrivial::MapCode {
                    to,
                }))
            }
            RunnerJsonInitAction::AddEnv { name, val } => {
                Ok(InitActionDependable::Trivial(InitActionTrivial::AddEnv {
                    name,
                    val,
                }))
            }
            RunnerJsonInitAction::SetArgs(args) => Ok(InitActionDependable::Trivial(
                InitActionTrivial::SetArgs(args),
            )),
            RunnerJsonInitAction::LinkWasm(file) => {
                let mut buf = Vec::new();
                zip_file
                    .by_name(&file)
                    .with_context(|| format!("linking {file}"))?
                    .read_to_end(&mut buf)?;
                Ok(InitActionDependable::Trivial(InitActionTrivial::LinkWasm {
                    contents: Arc::from(buf),
                    debug_path: format!("{}/{}", path_prefix, file),
                }))
            }
            RunnerJsonInitAction::StartWasm(file) => {
                let mut buf = Vec::new();
                zip_file
                    .by_name(&file)
                    .with_context(|| format!("starting {file}"))?
                    .read_to_end(&mut buf)?;
                Ok(InitActionDependable::Trivial(
                    InitActionTrivial::StartWasm {
                        contents: Arc::from(buf),
                        debug_path: format!("{}/{}", path_prefix, file),
                    },
                ))
            }
            RunnerJsonInitAction::Depends(dep) => Ok(InitActionDependable::Depends(Arc::from(dep))),
            RunnerJsonInitAction::When { cond, action: act } => Ok(InitActionDependable::When {
                cond,
                act: Box::new(rec(*act)?),
            }),
            RunnerJsonInitAction::Seq(vec) => {
                let r: Result<Vec<InitActionDependable>> =
                    vec.into_iter().map(|x| rec(x)).collect();
                Ok(InitActionDependable::Seq(r?))
            }
            RunnerJsonInitAction::With { runner, action } => {
                let runner_id = Arc::from(runner);
                let mut arch = Self::get_arch_for(&runner_id, runners_path.clone())?;
                Self::transform_action(&mut arch, &runner_id, *action, runners_path)
            }
        }
    }

    fn transform_action<R>(
        zip_file: &mut ZipArchive<R>,
        path_prefix: &str,
        action: RunnerJsonInitAction,
        runners_path: &std::path::PathBuf,
    ) -> Result<InitActionDependable>
    where
        R: std::io::Read + std::io::Seek,
    {
        let all_files: Vec<String> = zip_file
            .file_names()
            .filter(|f| !f.ends_with("/"))
            .map(|f| f.into())
            .sorted()
            .collect();
        RunnerReaderCacheEntry::transform_action_impl(
            zip_file,
            path_prefix,
            &all_files,
            action,
            runners_path,
        )
    }

    fn make_from_arch<R>(
        path_prefix: &str,
        zip_file: &mut ZipArchive<R>,
        runners_path: &std::path::PathBuf,
    ) -> Result<RunnerReaderCacheEntry>
    where
        R: std::io::Read + std::io::Seek,
    {
        let runner = std::io::read_to_string(zip_file.by_name("runner.json")?)?;
        let runner: RunnerJsonInitAction =
            serde_json::from_str(&runner).with_context(|| format!("json: {runner}"))?;

        let ret =
            RunnerReaderCacheEntry::transform_action(zip_file, path_prefix, runner, runners_path)
                .with_context(|| format!("pre_actions from {}", &path_prefix))?;

        Ok(RunnerReaderCacheEntry {
            action: Arc::new(ret),
        })
    }

    fn get_arch_for(
        runner_id: &Arc<str>,
        mut path: std::path::PathBuf,
    ) -> Result<ZipArchive<File>> {
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
        Ok(zip::ZipArchive::new(file)?)
    }

    fn make(runner_id: &Arc<str>, path: &std::path::PathBuf) -> Result<RunnerReaderCacheEntry> {
        let mut zip_file = Self::get_arch_for(runner_id, path.clone())?;

        RunnerReaderCacheEntry::make_from_arch(&runner_id, &mut zip_file, &path)
    }
}

pub struct RunnerReader {
    runners_path: std::path::PathBuf,
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
        Ok(RunnerReader { runners_path })
    }

    fn unfold(
        &mut self,
        path_prefix: &str,
        cache: &mut RunnerReaderCache,
        action: &InitActionDependable,
    ) -> Result<InitAction> {
        match action {
            InitActionDependable::Trivial(init_action_trivial) => {
                Ok(InitAction::Trivial(init_action_trivial.clone()))
            }
            InitActionDependable::When { cond, act } => Ok(InitAction::When {
                cond: *cond,
                act: Box::new(self.unfold(path_prefix, cache, act)?),
            }),
            InitActionDependable::Seq(vec) => {
                let r: Result<Vec<InitAction>> = vec
                    .iter()
                    .map(|x| self.unfold(path_prefix, cache, x))
                    .collect();
                Ok(InitAction::Seq(r?))
            }
            InitActionDependable::Depends(dep) => {
                let cache_entry = cache.cache.entry(dep.clone());
                let cache_entry: Arc<RunnerReaderCacheEntry> = match cache_entry {
                    std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
                    std::collections::hash_map::Entry::Vacant(e) => e.insert(Arc::new(
                        RunnerReaderCacheEntry::make(&dep, &self.runners_path)?,
                    )),
                }
                .clone();

                let res = self.unfold(&dep, cache, &*cache_entry.action)?;
                Ok(InitAction::Once(dep.clone(), Box::new(res)))
            }
        }
    }

    pub fn get_for_archive<R>(
        &mut self,
        path_prefix: &str,
        archive: &mut zip::ZipArchive<R>,
        cache: &mut RunnerReaderCache,
    ) -> Result<InitAction>
    where
        R: std::io::Read + std::io::Seek,
    {
        let cache_entry =
            RunnerReaderCacheEntry::make_from_arch(path_prefix, archive, &self.runners_path)?;
        self.unfold(path_prefix, cache, &*cache_entry.action)
    }
}
