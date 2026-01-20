use serde_derive::{Deserialize, Serialize};
use anyhow::{Result, bail};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Module {
    pub address: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Modules {
    pub llm: Module,
    pub web: Module,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub modules: Modules,
    pub cache_dir: String,
    pub runners_dir: String,
    pub registry_dir: String,

    #[serde(flatten)]
    pub base: genvm_common::BaseConfig,
}

impl Config {
    /// Validates the configuration to ensure all paths and module addresses are present.
    pub fn validate(&self) -> Result<()> {
        // Ensure module addresses are not empty
        if self.modules.llm.address.is_empty() {
            bail!("Config Error: LLM module address cannot be empty");
        }
        if self.modules.web.address.is_empty() {
            bail!("Config Error: Web module address cannot be empty");
        }

        // Ensure critical directories are specified
        if self.cache_dir.is_empty() || self.runners_dir.is_empty() {
            bail!("Config Error: Critical directory paths (cache/runners) cannot be empty");
        }

        log_info!("Configuration validated successfully");
        Ok(())
    }
}
