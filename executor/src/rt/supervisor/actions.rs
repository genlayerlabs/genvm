use std::collections::{BTreeMap, HashSet};

use crate::{caching, public_abi, rt, runners};

use anyhow::Context as _;
use genlayer_sdk::abi;
use genvm_common::*;
use symbol_table::GlobalSymbol;
use wiggle::error::Context as _;

pub struct Ctx<'a, 'b> {
    pub env: BTreeMap<String, String>,
    pub visited: HashSet<symbol_table::GlobalSymbol>,
    pub contract_id: symbol_table::GlobalSymbol,
    pub supervisor: &'a rt::supervisor::Supervisor,
    pub vm: &'b mut rt::vm::VMBase,
}

fn make_malformed_runner_error(extra_msg: &str) -> anyhow::Error {
    rt::errors::VMError(
        public_abi::VmError::invalid_contract().malformed_runner(),
        Some(anyhow::anyhow!("{}", extra_msg)),
    )
    .into()
}

fn maps_into_vm(to: &str) -> bool {
    to.split('/').find(|component| !component.is_empty()) == Some("vm")
}

/// How a resolved runner id should be loaded into the archive cache.
enum ResolvedKind {
    /// A packaged `name:hash` runner read from the runners directory.
    Disk {
        name: symbol_table::GlobalSymbol,
        hash: symbol_table::GlobalSymbol,
    },
    /// The current contract's runner, already preloaded into the cache.
    Preloaded,
    /// `chain:<address>:<a|f>:<slot>` — code blob read from a storage slot.
    Chain {
        address: calldata::Address,
        on: public_abi::StorageType,
        slot: crate::SlotID,
    },
    /// `custom:<hash>` — a runner registered at runtime.
    Custom { hash: symbol_table::GlobalSymbol },
}

/// A runner id resolved to its canonical cache key together with the way it
/// should be loaded.
struct Resolved {
    id: symbol_table::GlobalSymbol,
    kind: ResolvedKind,
}

/// Resolves a runner id to its canonical cache key and load strategy.
///
/// Free-standing so it can be shared between contract initialization
/// ([`Ctx`]) and runtime `gl_call`s ([`load_runner`]).
fn resolve_runner(
    supervisor: &rt::supervisor::Supervisor,
    contract_id: symbol_table::GlobalSymbol,
    id: symbol_table::GlobalSymbol,
) -> anyhow::Result<Resolved> {
    let Some(parsed) = runners::parse_runner_id(id.as_str()) else {
        return Err(make_malformed_runner_error(
            "runner id doesn't match expected format",
        ));
    };

    match parsed {
        runners::RunnerId::Contract => Ok(Resolved {
            id: contract_id,
            kind: ResolvedKind::Preloaded,
        }),
        runners::RunnerId::NameHash { name, hash } => {
            let hash = if hash.as_str() == "test" || hash.as_str() == "latest" {
                if !supervisor.shared_data.debug_mode {
                    log_warn!(":test/ :latest runner used in non-debug mode, this is not allowed");
                    return Err(make_malformed_runner_error(
                        "runner id doesn't match expected format",
                    ));
                }
                let new_latest = supervisor.runner_cache.get_latest(name);
                log_info!(runner_id = name.as_str(), new_latest:? = new_latest; "resolving :latest runner");
                let Some(new_latest) = new_latest else {
                    return Err(make_malformed_runner_error(
                        "runner id doesn't match expected format",
                    ));
                };
                new_latest
            } else {
                hash
            };

            if !supervisor.runner_cache.has_in_all(name, hash) {
                anyhow::bail!("runner {}:{} not found", name, hash);
            }

            let mut canonical = name.as_str().to_owned();
            canonical.push(':');
            canonical.push_str(hash.as_str());

            Ok(Resolved {
                id: GlobalSymbol::new(canonical),
                kind: ResolvedKind::Disk { name, hash },
            })
        }
        runners::RunnerId::Chain { address, on, slot } => Ok(Resolved {
            id: GlobalSymbol::from(runners::chain_canonical(address, on, slot)),
            kind: ResolvedKind::Chain { address, on, slot },
        }),
        runners::RunnerId::Custom { hash } => Ok(Resolved {
            id: runners::custom_runner_id(hash),
            kind: ResolvedKind::Custom { hash },
        }),
    }
}

/// Loads (and caches) the archive for an already-resolved runner.
async fn get_arch(
    supervisor: &rt::supervisor::Supervisor,
    limiter: &rt::memlimiter::Limiter,
    resolved: Resolved,
) -> anyhow::Result<(
    symbol_table::GlobalSymbol,
    sync::DArc<runners::ArchiveCache>,
)> {
    let Resolved { id, kind } = resolved;

    let cache = &supervisor.runner_cache;

    let new_arch = match kind {
        ResolvedKind::Disk { name, hash } => {
            cache
                .get_or_create(
                    id,
                    || async {
                        let mut path = cache.runners_path().to_owned();
                        runners::append_runner_subpath(name.as_str(), hash.as_str(), &mut path);
                        path.set_extension("tar");
                        if !path.exists() {
                            anyhow::bail!("runner {} not found", id);
                        }

                        let data = util::mmap_file(&path)
                            .with_context(|| format!("memory mapping runner archive for {id}"))?;
                        let data = bytes::Bytes::copy_from_slice(data.as_ref());
                        runners::Archive::from_ustar(data)
                            .with_context(|| format!("parsing ustar archive for {id}"))
                    },
                    limiter,
                )
                .await?
        }
        ResolvedKind::Preloaded => {
            cache
                .get_or_create(
                    id,
                    || async { anyhow::bail!("runner {} is not preloaded", id) },
                    limiter,
                )
                .await?
        }
        ResolvedKind::Chain { address, on, slot } => {
            cache
                .get_or_create(
                    id,
                    || async {
                        let mut storage = rt::vm::storage::Storage::new(
                            address,
                            supervisor.get_storage_limiter(),
                            crate::wasi::genlayer_sdk::StorageHostHolder(
                                supervisor.host.clone(),
                                crate::wasi::genlayer_sdk::ReadToken {
                                    account: address,
                                    mode: on,
                                },
                            ),
                        );
                        let code = storage
                            .read_code_at(slot, limiter)
                            .await
                            .with_context(|| format!("reading chain runner code for {id}"))?;
                        runners::parse(bytes::Bytes::from(code))
                            .with_context(|| format!("parsing chain runner for {id}"))
                    },
                    limiter,
                )
                .await?
        }
        ResolvedKind::Custom { hash } => {
            cache
                .get_or_create(
                    id,
                    || async {
                        supervisor
                            .get_custom_runner(hash)
                            .ok_or_else(|| anyhow::anyhow!("custom runner {} not found", id))
                    },
                    limiter,
                )
                .await?
        }
    };

    Ok((id, new_arch))
}

/// Resolves and loads a runner archive by id. Shared entry point used by both
/// contract initialization and runtime `gl_call`s.
pub(crate) async fn load_runner(
    supervisor: &rt::supervisor::Supervisor,
    contract_id: symbol_table::GlobalSymbol,
    limiter: &rt::memlimiter::Limiter,
    id: symbol_table::GlobalSymbol,
) -> anyhow::Result<(
    symbol_table::GlobalSymbol,
    sync::DArc<runners::ArchiveCache>,
)> {
    let resolved = resolve_runner(supervisor, contract_id, id)?;
    get_arch(supervisor, limiter, resolved).await
}

/// Maps a file (or, when `file` ends with `/`, a directory subtree) from a
/// runner archive into the VM filesystem at `to`. Mirrors [`InitAction::MapFile`]
/// so the runtime `MapFile` `gl_call` behaves identically.
pub(crate) fn map_archive_file(
    preview1: &mut crate::wasi::preview1::Context,
    limiter: &rt::memlimiter::Limiter,
    cancellation: &genvm_common::cancellation::Token,
    arch: &runners::ArchiveCache,
    file: &str,
    to: &str,
) -> anyhow::Result<()> {
    if file.ends_with("/") {
        let is_root = file == "/";

        let range = if is_root {
            arch.files.data.range::<str, std::ops::RangeFull>(..)
        } else {
            arch.files.data.range(String::from(file)..)
        };

        let must_start_with: &str = if is_root { "" } else { file };

        for (name, file_contents) in range {
            if cancellation.is_cancelled() {
                return Err(rt::errors::VMError(public_abi::VmError::timeout(), None).into());
            }

            if name.ends_with("/") {
                continue;
            }

            if !name.starts_with(must_start_with) {
                log_trace!(from = file, to = to, name = name; "aborting file mapping");
                break;
            }

            let mut name_in_fs = String::from(to);
            if !name_in_fs.ends_with("/") {
                name_in_fs.push('/');
            }
            name_in_fs.push_str(&name[must_start_with.len()..]);

            if maps_into_vm(&name_in_fs) {
                return Err(make_malformed_runner_error(&format!(
                    "mapping into /vm/ is forbidden: {name_in_fs}"
                )));
            }

            if !limiter
                .consume(public_abi::memory_limiter_consts::FILE_MAPPING + name_in_fs.len() as u32)
            {
                return Err(
                    rt::errors::VMError(abi::consts::VmError::oom().ram().val(), None).into(),
                );
            }

            preview1.map_file(&name_in_fs, file_contents.clone())?;
        }
    } else {
        if maps_into_vm(to) {
            return Err(make_malformed_runner_error(&format!(
                "mapping into /vm/ is forbidden: {to}"
            )));
        }

        if !limiter.consume(public_abi::memory_limiter_consts::FILE_MAPPING + to.len() as u32) {
            return Err(rt::errors::VMError(abi::consts::VmError::oom().ram().val(), None).into());
        }

        preview1.map_file(to, arch.get_file(file)?)?;
    }

    Ok(())
}

impl Ctx<'_, '_> {
    fn resolve_runner(&self, id: symbol_table::GlobalSymbol) -> anyhow::Result<Resolved> {
        resolve_runner(self.supervisor, self.contract_id, id)
    }

    async fn get_arch(
        &mut self,
        resolved: Resolved,
    ) -> anyhow::Result<(
        symbol_table::GlobalSymbol,
        sync::DArc<runners::ArchiveCache>,
    )> {
        let limiter = self.vm.store.data_mut().limits.clone();
        get_arch(self.supervisor, &limiter, resolved).await
    }

    fn load_modules(
        &mut self,
        current: symbol_table::GlobalSymbol,
        path: &std::sync::Arc<str>,
    ) -> anyhow::Result<Option<rt::DetNondet<wasmtime::Module>>> {
        let Some((id, hash)) = runners::verify_runner(current.as_str()) else {
            return Ok(None);
        };

        let special_name = caching::path_in_zip_to_hash(path);
        let Some(cache_dir) = &self.supervisor.wasm_mod_cache.cache_dir else {
            return Ok(None);
        };

        let mut cache_dir = cache_dir.to_owned();
        cache_dir.push(caching::PRECOMPILE_DIR_NAME);
        runners::append_runner_subpath(id, hash, &mut cache_dir);
        cache_dir.push(special_name);

        let det_mod = cache_dir.with_extension(caching::DET_NON_DET_PRECOMPILED_SUFFIX.det);

        if !det_mod.exists() {
            return Ok(None);
        }

        cache_dir.set_extension(caching::DET_NON_DET_PRECOMPILED_SUFFIX.non_det);
        let non_det_mod = cache_dir;

        if !det_mod.exists() {
            return Ok(None);
        }

        self.supervisor
            .shared_data
            .metrics
            .supervisor
            .precompile_hits
            .increment();

        Ok(Some(rt::DetNondet {
            det: unsafe {
                wasmtime::Module::deserialize_file(&self.supervisor.engines.det, &det_mod)
            }
            .with_context(|| format!("deserializing det module {path:?} of {current}"))?,
            non_det: unsafe {
                wasmtime::Module::deserialize_file(&self.supervisor.engines.non_det, &non_det_mod)
            }
            .with_context(|| format!("deserializing non-det module {path:?} of {current}"))?,
        }))
    }

    async fn link_wasm(
        &mut self,
        contents: bytes::Bytes,
        current: symbol_table::GlobalSymbol,
        path: &std::sync::Arc<str>,
    ) -> anyhow::Result<sync::DArc<rt::DetNondet<wasmtime::Module>>> {
        let mut wasm_key = String::from(current.as_str());
        wasm_key.push(':');
        wasm_key.push_str(path);

        let wasm_key = symbol_table::GlobalSymbol::from(wasm_key);

        let ret_mod = self
            .supervisor
            .wasm_mod_cache
            .wasm_modules_cache
            .get_or_create(wasm_key, || async {
                match self.load_modules(current, path) {
                    Ok(Some(loaded)) => return Ok(loaded),
                    Ok(None) => {}
                    Err(e) => {
                        log_error!(path:? = path, error:ah = e; "failed to load precompiled wasm module, recompiling");
                    }
                }

                self.supervisor
                    .compile_wasm(contents.as_ref(), wasm_key.as_str())
                    .await
                    .with_context(|| format!("compiling wasm {path:?} of {}", self.contract_id))
            })
            .await?;

        Ok(ret_mod)
    }

    pub async fn apply(
        &mut self,
        action: &runners::InitAction,
        current: symbol_table::GlobalSymbol,
        current_runner_arch: &runners::ArchiveCache,
    ) -> anyhow::Result<Option<wasmtime::Instance>> {
        use runners::InitAction;

        if self.supervisor.shared_data.cancellation.is_cancelled() {
            return Err(rt::errors::VMError(public_abi::VmError::timeout(), None).into());
        }

        match action {
            InitAction::MapFile { to, file } => {
                let limiter = self.vm.store.data_mut().limits.clone();
                let cancellation = self.supervisor.shared_data.cancellation.clone();
                let preview1 = &mut self.vm.store.data_mut().genlayer_ctx_mut().preview1;
                map_archive_file(
                    preview1,
                    &limiter,
                    &cancellation,
                    current_runner_arch,
                    file,
                    to,
                )?;
                Ok(None)
            }
            InitAction::AddEnv { name, val } => {
                let new_val = genvm_common::templater::patch_str(
                    &self.env,
                    val,
                    &genvm_common::templater::DOLLAR_UNFOLDER_RE,
                )?;
                self.env.insert(name.clone(), new_val);
                Ok(None)
            }
            InitAction::SetArgs(args) => {
                self.vm
                    .store
                    .data_mut()
                    .genlayer_ctx_mut()
                    .preview1
                    .set_args(&args[..])?;
                Ok(None)
            }
            InitAction::LinkWasm(path) => {
                let contents = current_runner_arch
                    .get_file(path)
                    .with_context(|| format!("getting file {path:?}"))?;

                let module = self
                    .link_wasm(contents, current, path)
                    .await
                    .with_context(|| format!("linking wasm {path:?}"))?;

                let module = module.into_gep(|x| x.get(self.vm.config_copy.is_deterministic));

                let instance = {
                    let instance = self
                        .vm
                        .linker
                        .instantiate_async(&mut self.vm.store, &module)
                        .await
                        .with_context(|| format!("instantiating {path:?}"))?;
                    let name = module
                        .name()
                        .ok_or_else(|| anyhow::anyhow!("can't link unnamed module {:?}", current))
                        .with_context(|| format!("getting module name for {path:?} of {current}"))
                        .map_err(|e| {
                            rt::errors::VMError::wrap(
                                public_abi::VmError::invalid_contract().wasm().linking(),
                                e,
                            )
                        })?;
                    self.vm
                        .linker
                        .instance(&mut self.vm.store, name, instance)
                        .with_context(|| format!("linking instance {name} for {path:?}"))?;
                    instance
                };
                match instance.get_typed_func::<(), ()>(&mut self.vm.store, "_initialize") {
                    Err(_) => {}
                    Ok(func) => {
                        log_info!(runner = current_runner_arch.runner_id().as_str(), path = path; "calling _initialize");
                        func.call_async(&mut self.vm.store, ()).await?;
                    }
                }
                Ok(None)
            }
            InitAction::StartWasm(path) => {
                let env: Vec<(String, String)> = self
                    .env
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                self.vm
                    .store
                    .data_mut()
                    .genlayer_ctx_mut()
                    .preview1
                    .set_env(&env)?;
                let contents = current_runner_arch
                    .get_file(path)
                    .with_context(|| format!("getting file {path:?}"))?;
                let module = self
                    .link_wasm(contents, current, path)
                    .await
                    .with_context(|| format!("linking wasm {path:?}"))?;

                let module = module.into_gep(|x| x.get(self.vm.config_copy.is_deterministic));

                Ok(Some(
                    self.vm
                        .linker
                        .instantiate_async(&mut self.vm.store, &module)
                        .await
                        .with_context(|| format!("instantiating {path:?}"))?,
                ))
            }
            InitAction::When { cond, action } => {
                if (*cond == runners::WasmMode::Det) != self.vm.config_copy.is_deterministic {
                    return Ok(None);
                }
                Box::pin(self.apply(action, current, current_runner_arch)).await
            }
            InitAction::Seq(vec) => {
                for act in vec {
                    if self.supervisor.shared_data.cancellation.is_cancelled() {
                        return Err(
                            rt::errors::VMError(public_abi::VmError::timeout(), None).into()
                        );
                    }

                    if let Some(x) = Box::pin(self.apply(act, current, current_runner_arch)).await?
                    {
                        return Ok(Some(x));
                    }
                }
                Ok(None)
            }
            InitAction::With {
                runner: uid,
                action,
            } => {
                let resolved = self.resolve_runner(*uid)?;
                let (uid, new_arch) = self.get_arch(resolved).await?;

                Box::pin(self.apply(action, uid, &new_arch))
                    .await
                    .with_context(|| format!("With {uid}"))
            }
            InitAction::Depends(uid) => {
                let resolved = self.resolve_runner(*uid)?;

                if !self.visited.insert(resolved.id) {
                    return Ok(None);
                }

                log_trace!(uid = resolved.id; "adding dependency");

                let (uid, new_arch) = self.get_arch(resolved).await?;

                let new_action = new_arch
                    .get_actions()
                    .await
                    .with_context(|| format!("loading {uid} runner.json"))?;

                Box::pin(self.apply(&new_action, uid, &new_arch))
                    .await
                    .with_context(|| format!("Depends {uid}"))
            }
        }
    }
}
