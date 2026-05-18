use serde_derive::Deserialize;

#[derive(Deserialize)]
pub struct Module {
    pub address: String,
}

#[derive(Deserialize)]
pub struct Modules {
    pub llm: Module,
    pub web: Module,
}

fn default_fee_expr_zero() -> String {
    "0".to_owned()
}

#[derive(Clone, Deserialize, Debug)]
pub struct FeesBucketConfig {
    pub bucket_no: u8,
    /// Cost charged once, up-front, when the bucket is created
    /// (the fixed part of `start + sum of per-change`).
    #[serde(default = "default_fee_expr_zero")]
    pub subtract_on_start_expr: String,
    /// Cost charged per change, evaluated with the `units` variable.
    pub delta_expr: String,
}

#[derive(Clone, Deserialize, Debug)]
pub struct FeesConfig {
    pub expr_prelude: String,
    pub storage: FeesBucketConfig,
    pub message_receipt: FeesBucketConfig,
    pub nondet_output: FeesBucketConfig,
    pub message_fee: FeesBucketConfig,
}

#[derive(Deserialize)]
pub struct Config {
    pub modules: Modules,
    pub fees: FeesConfig,
    pub cache_dir: String,
    pub runners_dir: String,
    pub registry_dir: String,

    #[serde(flatten)]
    pub base: genvm_common::BaseConfig,
}
