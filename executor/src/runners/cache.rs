use std::collections::BTreeMap;

use super::*;
use genlayer_sdk::abi;
use genvm_common::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct StrSymbol(symbol_table::GlobalSymbol);

impl StrSymbol {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl std::borrow::Borrow<str> for StrSymbol {
    fn borrow(&self) -> &str {
        self.0.as_str()
    }
}

impl Ord for StrSymbol {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl PartialOrd for StrSymbol {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl serde::Serialize for StrSymbol {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.as_str().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for StrSymbol {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self(symbol_table::GlobalSymbol::from(s)))
    }
}

pub struct Reader {
    cache: sync::CacheMap<ArchiveCache>,
    runners_data_path: std::path::PathBuf,

    all: BTreeMap<StrSymbol, Vec<StrSymbol>>,
    latest: BTreeMap<StrSymbol, StrSymbol>,
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

        let mut all: BTreeMap<_, Vec<_>> = serde_json::from_reader(
            std::fs::File::open(registry_path.join("all.json"))
                .with_context(|| format!("opening {registry_path:?}/all.json"))?,
        )?;
        for b in all.values_mut() {
            b.sort();
        }

        let latest = if debug_mode {
            serde_json::from_reader(
                std::fs::File::open(registry_path.join("latest.json"))
                    .with_context(|| format!("opening {registry_path:?}/latest.json"))?,
            )?
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

    pub fn get_latest(&self, id: &str) -> Option<&str> {
        self.latest.get(id).map(|s| s.as_str())
    }

    pub fn has_in_all(&self, id: &str, hash: &str) -> bool {
        match self.all.get(id) {
            Some(hashes) => hashes.iter().any(|h| h.as_str() == hash),
            None => false,
        }
    }

    pub fn runners_path(&self) -> &std::path::Path {
        &self.runners_data_path
    }

    pub fn put(&self, id: symbol_table::GlobalSymbol, archive: Archive) {
        self.cache.insert(id, ArchiveCache::new(id, archive));
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
                    return Err(anyhow::Error::from(rt::errors::VMError(
                        abi::consts::VmError::oom().ram().val(),
                        None,
                    )));
                }
                Ok(ArchiveCache::new(name, arch))
            })
            .await?;

        if !called.load(std::sync::atomic::Ordering::SeqCst)
            && !limiter.consume(res.files.total_size)
        {
            return Err(anyhow::Error::from(rt::errors::VMError(
                abi::consts::VmError::oom().ram().val(),
                None,
            )));
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
