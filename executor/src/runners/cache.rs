use super::*;
use genvm_common::*;

pub struct Reader {
    cache: sync::CacheMap<ArchiveCache>,
    runners_data_path: std::path::PathBuf,
    registry_path: std::path::PathBuf,
}

impl Reader {
    pub fn new() -> anyhow::Result<Self> {
        let runners_path = std::path::PathBuf::from(&path()?);
        if !runners_path.exists() {
            anyhow::bail!("path {:#?} doesn't exist", &runners_path);
        }

        Ok(Self {
            cache: sync::CacheMap::new(),
            runners_data_path: runners_path.clone(),
            registry_path: runners_path,
        })
    }

    pub fn runners_path(&self) -> &std::path::Path {
        &self.runners_data_path
    }

    pub fn registry_path(&self) -> &std::path::Path {
        &self.registry_path
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
