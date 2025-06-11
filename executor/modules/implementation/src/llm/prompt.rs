use std::sync::Arc;

use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::common::{ModuleError, ModuleResult};

#[derive(Serialize, Deserialize)]
pub struct Internal {
    pub system_message: Option<String>,
    pub user_message: String,
    pub temperature: f32,
    pub images: Vec<Arc<ImageLua>>,

    pub max_tokens: u32,
    pub use_max_completion_tokens: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum ImageType {
    PNG,
    JPG,
}

impl ImageType {
    pub fn sniff(data: &[u8]) -> Option<ImageType> {
        if data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
            Some(ImageType::PNG)
        } else if data.starts_with(&[0xFF, 0xD8, 0xFF, 0xE0]) {
            Some(ImageType::JPG)
        } else {
            None
        }
    }

    pub fn media_type(self) -> &'static str {
        match self {
            Self::JPG => "image/jpeg",
            Self::PNG => "image/png",
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ImageLua(#[serde(with = "serde_bytes")] pub Vec<u8>);

impl ImageLua {
    pub fn as_base64(&self) -> String {
        base64::prelude::BASE64_STANDARD.encode(&self.0)
    }

    pub fn kind_or_error(&self) -> ModuleResult<ImageType> {
        ImageType::sniff(&self.0).ok_or_else(|| {
            ModuleError {
                causes: vec!["INVALID_IMAGE".into()],
                fatal: true,
                ctx: std::collections::BTreeMap::new(),
            }
            .into()
        })
    }
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
