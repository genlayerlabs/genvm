use crate::{common, llm::handler::LuaErrorKind};
use anyhow::Context;
use base64::Engine;
use common::ModuleResult;

use super::{config, handler::LuaError, prompt};

#[async_trait::async_trait]
pub trait Provider {
    async fn exec_prompt_text(
        &self,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<String>;

    async fn exec_prompt_json_as_text(
        &self,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<String> {
        self.exec_prompt_text(prompt, model).await
    }

    async fn exec_prompt_json(
        &self,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<serde_json::Map<String, serde_json::Value>> {
        let res = self.exec_prompt_json_as_text(prompt, model).await?;
        let res = sanitize_json_str(&res);
        let res = serde_json::from_str(res)?;

        Ok(res)
    }

    async fn exec_prompt_bool_reason(
        &self,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<bool> {
        let res = self.exec_prompt_json(prompt, model).await?;
        let res = res
            .get("result")
            .and_then(|x| x.as_bool())
            .ok_or_else(|| anyhow::anyhow!("can't get reason from `{:?}`", res))?;
        Ok(res)
    }
}

pub struct OpenAICompatible {
    pub(crate) config: config::BackendConfig,
    pub(crate) client: reqwest::Client,
}

pub struct Gemini {
    pub(crate) config: config::BackendConfig,
    pub(crate) client: reqwest::Client,
}

pub struct OLlama {
    pub(crate) config: config::BackendConfig,
    pub(crate) client: reqwest::Client,
}

pub struct Anthropic {
    pub(crate) config: config::BackendConfig,
    pub(crate) client: reqwest::Client,
}

impl prompt::Internal {
    fn to_openai_messages(&self) -> Vec<serde_json::Value> {
        let mut messages = Vec::new();
        if let Some(sys) = &self.system_message {
            messages.push(serde_json::json!({
                "role": "system",
                "content": sys,
            }));
        }

        let mut user_content = Vec::new();

        user_content.push(serde_json::json!({
            "type": "text",
            "text": self.user_message,
        }));

        for img in &self.images {
            let mut encoded = "data:".to_owned();
            encoded.push_str(img.kind.media_type());
            encoded.push_str(";base64,");
            base64::prelude::BASE64_STANDARD.encode_string(&img.data, &mut encoded);

            user_content.push(serde_json::json!({
                "type": "image_url",
                "image_url": { "url": encoded },
            }));
        }

        messages.push(serde_json::json!({
            "role": "user",
            "content": user_content,
        }));

        messages
    }

    fn add_gemini_messages(&self, to: &mut serde_json::Map<String, serde_json::Value>) {
        if let Some(sys) = &self.system_message {
            to.insert(
                "system_instruction".to_owned(),
                serde_json::json!({
                    "parts": [{"text": sys}],
                }),
            );
        }

        let mut parts = Vec::new();
        for img in &self.images {
            parts.push(serde_json::json!({
                "inline_data": {
                    "mime_type": img.kind.media_type(),
                    "data": img.as_base64(),
                }
            }));
        }
        parts.push(serde_json::json!({"text": self.user_message}));

        to.insert(
            "contents".to_owned(),
            serde_json::json!([{
                "parts": parts,
            }]),
        );
    }
}

#[async_trait::async_trait]
impl Provider for OpenAICompatible {
    async fn exec_prompt_text(
        &self,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<String> {
        let mut request = serde_json::json!({
            "model": model,
            "messages": prompt.to_openai_messages(),
            "stream": false,
            "temperature": prompt.temperature,
        });

        if prompt.use_max_completion_tokens {
            request
                .as_object_mut()
                .unwrap()
                .insert("max_completion_tokens".to_owned(), prompt.max_tokens.into());
        } else {
            request
                .as_object_mut()
                .unwrap()
                .insert("max_tokens".to_owned(), prompt.max_tokens.into());
        }

        let request = serde_json::to_vec(&request)?;
        let res = send_with_retries(|| {
            self.client
                .post(format!("{}/v1/chat/completions", self.config.host))
                .header("Content-Type", "application/json")
                .header("Authorization", &format!("Bearer {}", &self.config.key))
                .body(request.clone())
        })
        .await?;

        let response = res
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res))?;

        Ok(response.to_owned())
    }

    async fn exec_prompt_json(
        &self,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<serde_json::Map<String, serde_json::Value>> {
        let mut request = serde_json::json!({
            "model": model,
            "messages": prompt.to_openai_messages(),
            "stream": false,
            "temperature": prompt.temperature,
            "response_format": {"type": "json_object"},
        });

        if prompt.use_max_completion_tokens {
            request
                .as_object_mut()
                .unwrap()
                .insert("max_completion_tokens".to_owned(), prompt.max_tokens.into());
        } else {
            request
                .as_object_mut()
                .unwrap()
                .insert("max_tokens".to_owned(), prompt.max_tokens.into());
        }

        let request = serde_json::to_vec(&request)?;
        let res = send_with_retries(|| {
            self.client
                .post(format!("{}/v1/chat/completions", self.config.host))
                .header("Content-Type", "application/json")
                .header("Authorization", &format!("Bearer {}", &self.config.key))
                .body(request.clone())
        })
        .await?;

        let response = res
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res))?;

        let response = serde_json::from_str(response)?;
        Ok(response)
    }
}

impl prompt::Internal {
    fn to_ollama_no_format(&self, model: &str) -> serde_json::Value {
        let mut request = serde_json::json!({
            "model": model,
            "prompt": self.user_message,
            "stream": false,
            "options": {
                "temperature": self.temperature,
                "num_predict": self.max_tokens,
            },
        });

        let mut images = Vec::new();
        for img in &self.images {
            images.push(serde_json::Value::String(img.as_base64()));
        }
        request
            .as_object_mut()
            .unwrap()
            .insert("images".into(), serde_json::Value::Array(images));

        if let Some(sys) = &self.system_message {
            request
                .as_object_mut()
                .unwrap()
                .insert("system".into(), sys.to_owned().into());
        }

        request
    }
}

#[async_trait::async_trait]
impl Provider for OLlama {
    async fn exec_prompt_text(
        &self,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<String> {
        let request = prompt.to_ollama_no_format(model);

        let request = serde_json::to_vec(&request)?;
        let res = send_with_retries(|| {
            self.client
                .post(format!("{}/api/generate", self.config.host))
                .body(request.clone())
        })
        .await?;

        let response = res
            .as_object()
            .and_then(|v| v.get("response"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res))?;
        Ok(response.to_owned())
    }

    async fn exec_prompt_json_as_text(
        &self,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<String> {
        let mut request = prompt.to_ollama_no_format(model);

        request
            .as_object_mut()
            .unwrap()
            .insert("format".into(), "json".into());

        let mut images = Vec::new();
        for img in &prompt.images {
            images.push(serde_json::Value::String(img.as_base64()));
        }

        if !images.is_empty() {
            request
                .as_object_mut()
                .unwrap()
                .insert("images".into(), serde_json::Value::Array(images));
        }

        if let Some(sys) = &prompt.system_message {
            request
                .as_object_mut()
                .unwrap()
                .insert("system".into(), sys.to_owned().into());
        }

        let request = serde_json::to_vec(&request)?;
        let res = send_with_retries(|| {
            self.client
                .post(format!("{}/api/generate", self.config.host))
                .body(request.clone())
        })
        .await?;

        let response = res
            .as_object()
            .and_then(|v| v.get("response"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res))?;
        Ok(response.to_owned())
    }
}

#[async_trait::async_trait]
impl Provider for Gemini {
    async fn exec_prompt_text(
        &self,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<String> {
        let mut request = serde_json::json!({
            "generationConfig": {
                "responseMimeType": "text/plain",
                "temperature": prompt.temperature,
                "maxOutputTokens": prompt.max_tokens,
            }
        });

        prompt.add_gemini_messages(request.as_object_mut().unwrap());

        let request = serde_json::to_vec(&request)?;
        let res = send_with_retries(|| {
            self.client
                .post(format!(
                    "{}/v1beta/models/{}:generateContent?key={}",
                    self.config.host, model, self.config.key
                ))
                .header("Content-Type", "application/json")
                .body(request.clone())
        })
        .await?;

        let res = res
            .pointer("/candidates/0/content/parts/0/text")
            .and_then(|x| x.as_str())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res))?;
        Ok(res.into())
    }

    async fn exec_prompt_json_as_text(
        &self,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<String> {
        let mut request = serde_json::json!({
            "generationConfig": {
                "responseMimeType": "application/json",
                "temperature": prompt.temperature,
                "maxOutputTokens": prompt.max_tokens,
            }
        });

        prompt.add_gemini_messages(request.as_object_mut().unwrap());

        let request = serde_json::to_vec(&request)?;
        let res = send_with_retries(|| {
            self.client
                .post(format!(
                    "{}/v1beta/models/{}:generateContent?key={}",
                    self.config.host, model, self.config.key
                ))
                .header("Content-Type", "application/json")
                .body(request.clone())
        })
        .await?;

        let res = res
            .pointer("/candidates/0/content/parts/0/text")
            .and_then(|x| x.as_str())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res))?;

        Ok(res.to_owned())
    }
}

impl prompt::Internal {
    fn to_anthropic_no_format(&self, model: &str) -> serde_json::Value {
        let mut user_content = Vec::new();

        for img in &self.images {
            user_content.push(serde_json::json!({"type": "image", "source": {
                "type": "base64",
                "media_type": img.kind.media_type(),
                "data": img.as_base64(),
            }}));
        }

        user_content.push(serde_json::json!({"type": "text", "text": self.user_message}));

        let mut request = serde_json::json!({
            "model": model,
            "messages": [{"role": "user", "content": user_content}],
            "max_tokens": self.max_tokens,
            "stream": false,
            "temperature": self.temperature,
        });

        if let Some(sys) = &self.system_message {
            request
                .as_object_mut()
                .unwrap()
                .insert("system".into(), sys.to_owned().into());
        }

        request
    }
}

#[async_trait::async_trait]
impl Provider for Anthropic {
    async fn exec_prompt_text(
        &self,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<String> {
        let request = prompt.to_anthropic_no_format(model);

        let request = serde_json::to_vec(&request)?;
        let res = send_with_retries(|| {
            self.client
                .post(format!("{}/v1/messages", self.config.host))
                .header("Content-Type", "application/json")
                .header("x-api-key", &self.config.key)
                .header("anthropic-version", "2023-06-01")
                .body(request.clone())
        })
        .await?;

        res.pointer("/content/0/text")
            .and_then(|x| x.as_str())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res))
            .map(String::from)
    }

    async fn exec_prompt_json(
        &self,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<serde_json::Map<String, serde_json::Value>> {
        let mut request = prompt.to_anthropic_no_format(model);

        request.as_object_mut().unwrap().insert(
            "tools".to_owned(),
            serde_json::json!(
                [{
                    "name": "json_out",
                    "description": "Output a valid json object",
                    "input_schema": {
                        "type": "object",
                        "patternProperties": {
                            "": {
                                "type": ["object", "null", "array", "number", "string"],
                            }
                        },
                    }
                }]
            ),
        );
        request.as_object_mut().unwrap().insert(
            "tool_choice".to_owned(),
            serde_json::json!({
                "type": "tool",
                "name": "json_out"
            }),
        );

        if let Some(sys) = &prompt.system_message {
            request
                .as_object_mut()
                .unwrap()
                .insert("system".into(), sys.to_owned().into());
        }

        let request = serde_json::to_vec(&request)?;
        let res = send_with_retries(|| {
            self.client
                .post(format!("{}/v1/messages", self.config.host))
                .header("Content-Type", "application/json")
                .header("x-api-key", &self.config.key)
                .header("anthropic-version", "2023-06-01")
                .body(request.clone())
        })
        .await?;

        let val = res
            .pointer("/content/0/input")
            .and_then(|x| x.as_object())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res))?;

        Ok(val.clone())
    }

    async fn exec_prompt_bool_reason(
        &self,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<bool> {
        let mut request = serde_json::json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt.user_message}],
            "max_tokens": 200,
            "stream": false,
            "temperature": prompt.temperature,
            "tools": [{
                "name": "json_out",
                "description": "Output a valid json object",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "result": { "type": "boolean" },
                        "reason": { "type": "string" },
                    },
                    "required": ["result"],
                }
            }],
            "tool_choice": {
                "type": "tool",
                "name": "json_out"
            }
        });

        if let Some(sys) = &prompt.system_message {
            request
                .as_object_mut()
                .unwrap()
                .insert("system".into(), sys.to_owned().into());
        }

        let request = serde_json::to_vec(&request)?;
        let res = send_with_retries(|| {
            self.client
                .post(format!("{}/v1/messages", self.config.host))
                .header("Content-Type", "application/json")
                .header("x-api-key", &self.config.key)
                .header("anthropic-version", "2023-06-01")
                .body(request.clone())
        })
        .await?;

        let val = res
            .pointer("/content/0/input/result")
            .and_then(|x| x.as_bool())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res))?;

        Ok(val)
    }
}

fn sanitize_json_str(s: &str) -> &str {
    let s = s.trim();
    let s = s
        .strip_prefix("```json")
        .or(s.strip_prefix("```"))
        .unwrap_or(s);
    let s = s.strip_suffix("```").unwrap_or(s);
    s.trim()
}

async fn send_with_retries(
    builder: impl (FnOnce() -> reqwest::RequestBuilder) + Send,
) -> anyhow::Result<serde_json::Value> {
    let req = builder();

    log::trace!(request = common::censor_debug(&req), cookie = common::get_cookie(); "sending request");

    let res = req
        .send()
        .await
        .with_context(|| "sending request to llm provider")
        .map_err(|e| {
            log::warn!(cookie = common::get_cookie(), error = genvm_common::log_error(&e); "sending request failed");

            LuaError {
                kind: crate::llm::handler::LuaErrorKind::RequestFailed,
                context: serde_json::Value::Null,
            }
        })?;

    let mut context = serde_json::Map::new();

    let status_code = res.status();

    context.insert("status_code".into(), status_code.as_u16().into());

    let body = res.text().await.with_context(|| "reading_body")
        .map_err(|e| {
            log::warn!(cookie = common::get_cookie(), error = genvm_common::log_error(&e), status = status_code.as_u16(); "reading body failed");

            LuaError {
                kind: crate::llm::handler::LuaErrorKind::BodyReadingFailed,
                context: context.clone().into(),
            }
        })?;

    let body: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        log::warn!(cookie = common::get_cookie(), error:err = e, status = status_code.as_u16(), body = common::censor_str(&body); "parsing body failed");

        context.insert("body_raw".into(), serde_json::Value::String(body));

        LuaError {
            kind: crate::llm::handler::LuaErrorKind::BodyReadingFailed,
            context: context.clone().into(),
        }
    })?;

    use reqwest::StatusCode;
    let err_kind = match status_code {
        StatusCode::OK => return Ok(body),
        StatusCode::REQUEST_TIMEOUT
        | StatusCode::SERVICE_UNAVAILABLE
        | StatusCode::TOO_MANY_REQUESTS
        | StatusCode::GATEWAY_TIMEOUT => LuaErrorKind::Overloaded,
        x if [529].contains(&x.as_u16()) => LuaErrorKind::Overloaded,
        _ => LuaErrorKind::StatusNotOk,
    };

    context.insert("body".into(), body);

    Err(LuaError {
        kind: err_kind,
        context: context.into(),
    }
    .into())
}
