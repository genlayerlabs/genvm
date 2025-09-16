use std::collections::BTreeMap;

use super::*;
use genvm_common::*;

pub struct Reader {
    cache: sync::CacheMap<ArchiveCache>,
    runners_data_path: std::path::PathBuf,

    all: BTreeMap<symbol_table::GlobalSymbol, Vec<symbol_table::GlobalSymbol>>,
    latest: BTreeMap<symbol_table::GlobalSymbol, symbol_table::GlobalSymbol>,
}

impl Reader {
    pub fn new(
        path: &std::path::Path,
        registry_path: &std::path::Path,
        debug_mode: bool,
    ) -> anyhow::Result<Self> {
        let runners_path = std::path::PathBuf::from(path);
        if !runners_path.exists() {
            anyhow::bail!("path {:#?} doesn't exist", &runners_path);
        }

        let mut all: BTreeMap<_, Vec<_>> =
            serde_json::from_reader(std::fs::File::open(registry_path.join("all.json"))?)?;
        for b in all.values_mut() {
            b.sort();
        }

        let latest = if debug_mode {
            serde_json::from_reader(std::fs::File::open(registry_path.join("latest.json"))?)?
        } else {
            BTreeMap::new()
        };

        Ok(Self {
            cache: sync::CacheMap::new(),
            runners_data_path: runners_path.clone(),
            all,
            latest,
        })
    }

    pub fn get_latest(&self, id: symbol_table::GlobalSymbol) -> Option<symbol_table::GlobalSymbol> {
        self.latest.get(&id).cloned()
    }

    pub fn has_in_all(
        &self,
        id: symbol_table::GlobalSymbol,
        hash: symbol_table::GlobalSymbol,
    ) -> bool {
        match self.all.get(&id) {
            Some(hashes) => hashes.binary_search(&hash).is_ok(),
            None => false,
        }
    }

    pub fn runners_path(&self) -> &std::path::Path {
        &self.runners_data_path
    }

    pub async fn get_or_create<F>(
        &self,
        name: symbol_table::GlobalSymbol,
        arch_provider: impl FnOnce() -> F,
        limiter: &rt::memlimiter::Limiter,
    ) -> anyhow::Result<sync::DArc<ArchiveCache>>
    where
        F: std::future::Future<Output = anyhow::Result<Archive>>,
    {
        let called = std::sync::atomic::AtomicBool::new(false);

        let res = self
            .cache
            .get_or_create(name, || async {
                called.store(true, std::sync::atomic::Ordering::SeqCst);
                let arch = arch_provider().await?;
                if !limiter.consume(arch.total_size) {
                    return Err(anyhow::Error::from(rt::errors::VMError::oom(None)));
                }
                Ok(ArchiveCache::new(name, arch))
            })
            .await?;

        if !called.load(std::sync::atomic::Ordering::SeqCst)
            && !limiter.consume(res.files.total_size)
        {
            return Err(anyhow::Error::from(rt::errors::VMError::oom(None)));
        }

        Ok(res)
    }
}

pub fn get_cache_dir(base_path: &str) -> anyhow::Result<std::path::PathBuf> {
    let base_path = std::path::Path::new(base_path);

    std::fs::create_dir_all(base_path).with_context(|| "creating cache dir")?;

    let test_path = base_path.join(".test");
    std::fs::write(test_path, "").with_context(|| "creating test file")?;
    Ok(base_path.to_owned())
}
