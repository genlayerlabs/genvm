use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Internal {
    pub system_message: Option<String>,
    pub user_message: String,
    pub temperature: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ExtendedOutputFormat {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "json")]
    JSON,
    #[serde(rename = "bool")]
    Bool,
}
