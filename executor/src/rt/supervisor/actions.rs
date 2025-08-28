use std::collections::{BTreeMap, HashSet};

use crate::{caching, public_abi, rt, runners};

use anyhow::Context;
use genvm_common::*;

pub struct Ctx<'a, 'b> {
    pub env: BTreeMap<String, String>,
    pub visited: HashSet<symbol_table::GlobalSymbol>,
    pub contract_id: symbol_table::GlobalSymbol,
    pub supervisor: &'a rt::supervisor::Supervisor,
    pub vm: &'b mut rt::vm::VMBase,
}

fn try_get_latest(runner_id: &str, registry_path: &std::path::Path) -> Option<String> {
    let mut path = std::path::PathBuf::from(registry_path);
    path.push("latest.json");

    let latest_registry = std::fs::read_to_string(&path).ok()?;
    let mut latest_registry: BTreeMap<String, String> =
        serde_json::from_str(&latest_registry).ok()?;

    latest_registry.remove(runner_id)
}

impl Ctx<'_, '_> {
    async fn get_arch(
        &mut self,
        uid: symbol_table::GlobalSymbol,
    ) -> anyhow::Result<(
        symbol_table::GlobalSymbol,
        sync::DArc<runners::ArchiveCache>,
    )> {
        let uid = self.unfold_test_id_if_any(uid, self.supervisor.runner_cache.registry_path());

        let Some((runner_id, runner_hash)) = runners::verify_runner(uid.as_str()) else {
            anyhow::bail!("invalid runner id: {}", uid);
        };

        let limiter = &self.vm.store.data_mut().limits;

        let new_arch = self
            .supervisor
            .runner_cache
            .get_or_create(
                uid,
                || async {
                    let mut path = self.supervisor.runner_cache.runners_path().to_owned();
                    runners::append_runner_subpath(runner_id, runner_hash, &mut path);
                    path.set_extension("tar");
                    if !path.exists() {
                        anyhow::bail!("runner {} not found", uid);
                    }

                    let data = util::mmap_file(&path)
                        .with_context(|| format!("creating new archive for {uid}"))?;
                    let data = util::SharedBytes::new(data);
                    runners::Archive::from_ustar(data)
                },
                limiter,
            )
            .await?;

        Ok((uid, new_arch))
    }

    fn unfold_test_id_if_any(
        &mut self,
        id: symbol_table::GlobalSymbol,
        registry_path: &std::path::Path,
    ) -> symbol_table::GlobalSymbol {
        if id.as_str() == "<contract>" {
            return self.contract_id;
        }
        let Some((runner_id, runner_hash)) = runners::verify_runner(id.as_str()) else {
            return id;
        };

        if runner_hash != "test" && runner_hash != "latest" {
            return id;
        }

        if !self.supervisor.shared_data.debug_mode {
            log_warn!(":test/ :latest runner used in non-debug mode, this is not allowed");

            return id;
        }

        let Some(borrowed) = try_get_latest(runner_id, registry_path) else {
            return id;
        };
        let mut new_id = runner_id.to_owned();
        new_id.push(':');
        new_id.push_str(&borrowed);

        symbol_table::GlobalSymbol::new(new_id)
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
            }?,
            non_det: unsafe {
                wasmtime::Module::deserialize_file(&self.supervisor.engines.non_det, &non_det_mod)
            }?,
        }))
    }

    async fn link_wasm(
        &mut self,
        contents: util::SharedBytes,
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
                if let Some(loaded) = self.load_modules(current, path)? {
                    return Ok(loaded);
                }

                self.supervisor
                    .compile_wasm(contents.as_ref(), wasm_key.as_str())
                    .await
                    .with_context(|| format!("compiling wasm for {}", self.contract_id))
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
            return Err(
                rt::errors::VMError(public_abi::VmError::Timeout.value().to_owned(), None).into(),
            );
        }

        match action {
            InitAction::MapFile { to, file } => {
                if file.ends_with("/") {
                    let is_root = file.as_ref() == "/";

                    let file_name_str = String::from(&file[..]);

                    let range = if is_root {
                        current_runner_arch
                            .files
                            .data
                            .range::<str, std::ops::RangeFull>(..)
                    } else {
                        current_runner_arch.files.data.range(file_name_str..)
                    };

                    let must_start_with: &str = if is_root { "" } else { file.as_ref() };

                    for (name, file_contents) in range {
                        if self.supervisor.shared_data.cancellation.is_cancelled() {
                            return Err(rt::errors::VMError(
                                public_abi::VmError::Timeout.value().to_owned(),
                                None,
                            )
                            .into());
                        }

                        if name.ends_with("/") {
                            continue;
                        }

                        if !name.starts_with(must_start_with) {
                            log_trace!(from = file, to = to, name = name; "aborting file mapping");

                            break;
                        }

                        let mut name_in_fs = String::from(&to[..]);
                        if !name_in_fs.ends_with("/") {
                            name_in_fs.push('/');
                        }
                        name_in_fs.push_str(&name[must_start_with.len()..]);

                        let limiter = &self.vm.store.data_mut().limits;

                        if !limiter.consume(
                            public_abi::MemoryLimiterConsts::FileMapping.value()
                                + name_in_fs.len() as u32,
                        ) {
                            return Err(rt::errors::VMError::oom(None).into());
                        }

                        self.vm
                            .store
                            .data_mut()
                            .genlayer_ctx_mut()
                            .preview1
                            .map_file(&name_in_fs, file_contents.clone())?;
                    }
                } else {
                    let limiter = &self.vm.store.data_mut().limits;

                    if !limiter.consume(
                        public_abi::MemoryLimiterConsts::FileMapping.value() + to.len() as u32,
                    ) {
                        return Err(rt::errors::VMError::oom(None).into());
                    }

                    self.vm
                        .store
                        .data_mut()
                        .genlayer_ctx_mut()
                        .preview1
                        .map_file(to, current_runner_arch.get_file(file)?)?;
                }
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
                let contents = current_runner_arch.get_file(path)?;

                let module = self.link_wasm(contents, current, path).await?;

                let module = module.into_gep(|x| x.get(self.vm.config_copy.is_deterministic));

                let instance = {
                    let instance = self
                        .vm
                        .linker
                        .instantiate_async(&mut self.vm.store, &module)
                        .await?;
                    let name = module
                        .name()
                        .ok_or_else(|| anyhow::anyhow!("can't link unnamed module {:?}", current))
                        .map_err(|e| {
                            rt::errors::VMError::wrap(
                                format!("{} wasm", public_abi::VmError::InvalidContract.value()),
                                e,
                            )
                        })?;
                    self.vm
                        .linker
                        .instance(&mut self.vm.store, name, instance)?;
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
                let contents = current_runner_arch.get_file(path)?;
                let module = self.link_wasm(contents, current, path).await?;

                let module = module.into_gep(|x| x.get(self.vm.config_copy.is_deterministic));

                Ok(Some(
                    self.vm
                        .linker
                        .instantiate_async(&mut self.vm.store, &module)
                        .await?,
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
                        return Err(rt::errors::VMError(
                            public_abi::VmError::Timeout.value().to_owned(),
                            None,
                        )
                        .into());
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
                let (uid, new_arch) = self.get_arch(*uid).await?;

                Box::pin(self.apply(action, uid, &new_arch))
                    .await
                    .with_context(|| format!("With {uid}"))
            }
            InitAction::Depends(uid) => {
                let uid =
                    self.unfold_test_id_if_any(*uid, self.supervisor.runner_cache.registry_path());

                if !self.visited.insert(uid) {
                    return Ok(None);
                }

                log_trace!(uid = uid; "adding dependency");

                let (uid, new_arch) = self.get_arch(uid).await?;

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
