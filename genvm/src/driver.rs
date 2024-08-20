
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct DriverData {
    pub entrypoint: String,
    pub args: Vec<String>,
    //pub env: Vec<(String, String)>,
}