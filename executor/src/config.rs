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

fn deserialize_bucket_nos<'de, D>(d: D) -> Result<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de;

    struct Visitor;
    impl<'de> de::Visitor<'de> for Visitor {
        type Value = Vec<u8>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("an integer or array of integers")
        }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Vec<u8>, E> {
            u8::try_from(v)
                .map(|b| vec![b])
                .map_err(|_| E::custom(format!("bucket_no {v} exceeds u8 range")))
        }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Vec<u8>, E> {
            u8::try_from(v)
                .map(|b| vec![b])
                .map_err(|_| E::custom(format!("bucket_no {v} out of u8 range")))
        }
        fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<u8>, A::Error> {
            let mut v = Vec::new();
            while let Some(n) = seq.next_element::<u8>()? {
                v.push(n);
            }
            Ok(v)
        }
    }
    d.deserialize_any(Visitor)
}

#[derive(Clone, Deserialize, Debug)]
pub struct FeesBucketConfig {
    #[serde(deserialize_with = "deserialize_bucket_nos")]
    pub bucket_no: Vec<u8>,
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
    pub event: FeesBucketConfig,
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
