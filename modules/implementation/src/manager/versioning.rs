use anyhow::Context;
use genvm_common::sync;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::manager::Config;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ExecutorVersion {
    pub available_after: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl<'de> serde::Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(serde::de::Error::custom("Invalid version format"));
        }
        let part0 = parts[0].strip_prefix("v").unwrap_or(parts[0]);
        let major = part0
            .parse()
            .map_err(|_| serde::de::Error::custom("Invalid major version"))?;
        let minor = parts[1]
            .parse()
            .map_err(|_| serde::de::Error::custom("Invalid minor version"))?;
        let patch = parts[2]
            .parse()
            .map_err(|_| serde::de::Error::custom("Invalid patch version"))?;

        Ok(Version {
            major,
            minor,
            patch,
        })
    }
}

#[derive(serde::Deserialize, Clone)]
pub struct Manifest {
    pub executor_versions: std::collections::BTreeMap<Version, ExecutorVersion>,
}

pub struct Ctx {
    config: sync::DArc<Config>,
    manifest: tokio::sync::RwLock<Manifest>,
}

async fn load_manifest(manifest_path: &str) -> anyhow::Result<Manifest> {
    let content = tokio::fs::read_to_string(manifest_path)
        .await
        .with_context(|| format!("Failed to read manifest file: {}", manifest_path))?;
    let manifest: Manifest =
        serde_yaml::from_str(&content).with_context(|| "Failed to parse manifest YAML")?;
    Ok(manifest)
}

impl Ctx {
    pub async fn new(config: sync::DArc<Config>) -> anyhow::Result<Self> {
        let manifest = load_manifest(&config.manifest_path).await?;
        Ok(Self {
            manifest: tokio::sync::RwLock::new(manifest),
            config,
        })
    }

    pub async fn reload_manifest(&self) -> anyhow::Result<()> {
        let manifest = load_manifest(&self.config.manifest_path).await?;

        let mut lock = self.manifest.write().await;
        *lock = manifest;

        Ok(())
    }

    pub async fn get_latest_major(&self, timestamp: chrono::DateTime<chrono::Utc>) -> Option<u32> {
        let lock = self.manifest.read().await;
        lock.executor_versions
            .iter()
            .filter(|(_, ev)| ev.available_after <= timestamp)
            .map(|(ver, _)| ver.major)
            .max()
    }

    pub async fn get_version(
        &self,
        major: u32,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Option<Version> {
        let lock = self.manifest.read().await;

        let Some(mut ver) = lock
            .executor_versions
            .iter()
            .filter(|(ver, ev)| ver.major == major && ev.available_after <= timestamp)
            .map(|(ver, _)| *ver)
            .max()
        else {
            return None;
        };

        loop {
            let mut next = ver;
            next.patch += 1;
            if lock.executor_versions.contains_key(&next) {
                ver = next;
            } else {
                break;
            }
        }

        Some(ver)
    }
}

pub async fn detect_major_spec(
    full_ctx: &crate::manager::AppContext,
    data: &[u8],
    deployment_timestamp: chrono::DateTime<chrono::Utc>,
) -> anyhow::Result<u32> {
    let zelf = &full_ctx.ver_ctx;

    let Some(possible_major) = zelf.get_latest_major(deployment_timestamp).await else {
        anyhow::bail!("no_executor_version_available");
    };

    let execute_in = zelf
        .get_version(possible_major, deployment_timestamp)
        .await
        .with_context(|| "failed_to_get_executor_version")?;

    let mut genvm_path = std::path::PathBuf::from(full_ctx.run_ctx.executors_path());

    genvm_path.push(format!(
        "v{}.{}.{}",
        execute_in.major, execute_in.minor, execute_in.patch
    ));

    #[cfg(debug_assertions)]
    if zelf.config.reroute_to_test {
        genvm_path.pop();
        genvm_path.push("vTEST");
    }
    genvm_path.push("bin");
    genvm_path.push("genvm");

    let mut proc = tokio::process::Command::new(&genvm_path)
        .arg("parse-version-pattern")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .with_context(|| format!("running genvm command {:?}", genvm_path))?;

    let stdin = proc.stdin.take();
    let stdout = proc.stdout.take();

    let task = async move {
        let mut stdin = stdin.with_context(|| "failed_to_open_stdin")?;
        stdin
            .write_all(data)
            .await
            .with_context(|| "failed_to_write_to_stdin")?;
        std::mem::drop(stdin);
        let mut res = String::new();
        stdout
            .with_context(|| "failed_to_open_stdout")?
            .read_to_string(&mut res)
            .await?;

        let res = res.trim();
        let res = res.strip_prefix("v").unwrap_or(res);
        let res = &res[..res.find('.').unwrap_or(res.len())];

        let res = res.parse::<u32>().unwrap_or(possible_major);

        Ok(res)
    };

    let detected_version = task.await;

    let _ = proc.wait().await;

    detected_version.map(|v| v.min(possible_major))
}
