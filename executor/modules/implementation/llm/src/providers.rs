use anyhow::Context;
use genvm_modules_impl_common::ModuleResult;

use crate::{config, handler::OverloadedError, prompt};

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
        messages.push(serde_json::json!({
            "role": "user",
            "content": self.user_message,
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

        to.insert(
            "contents".to_owned(),
            serde_json::json!({
                "parts": [{"text": self.user_message}],
            }),
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
        let request = serde_json::json!({
            "model": model,
            "messages": prompt.to_openai_messages(),
            "max_tokens": 1000,
            "stream": false,
            "temperature": prompt.temperature,
        });

        let request = serde_json::to_vec(&request)?;
        let res = send_with_retries(|| {
            self.client
                .post(format!("{}/v1/chat/completions", self.config.host))
                .header("Content-Type", "application/json")
                .header("Authorization", &format!("Bearer {}", &self.config.key))
                .body(request.clone())
        })
        .await?;
        let res = genvm_modules_impl_common::read_response(res).await?;
        let val: serde_json::Value = serde_json::from_str(&res)?;
        let response = val
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
        let request = serde_json::json!({
            "model": model,
            "messages": prompt.to_openai_messages(),
            "max_tokens": 1000,
            "stream": false,
            "temperature": prompt.temperature,
            "response_format": {"type": "json_object"},
        });

        let request = serde_json::to_vec(&request)?;
        let res = send_with_retries(|| {
            self.client
                .post(format!("{}/v1/chat/completions", self.config.host))
                .header("Content-Type", "application/json")
                .header("Authorization", &format!("Bearer {}", &self.config.key))
                .body(request.clone())
        })
        .await?;
        let res = genvm_modules_impl_common::read_response(res).await?;
        let val: serde_json::Value = serde_json::from_str(&res)?;
        let response = val
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res))?;

        let response = serde_json::from_str(response)?;
        Ok(response)
    }
}

#[async_trait::async_trait]
impl Provider for OLlama {
    async fn exec_prompt_text(
        &self,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<String> {
        let mut request = serde_json::json!({
            "model": model,
            "prompt": prompt.user_message,
            "stream": false,
            "options": {
                "temperature": prompt.temperature,
            },
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
                .post(format!("{}/api/generate", self.config.host))
                .body(request.clone())
        })
        .await?;
        let res = genvm_modules_impl_common::read_response(res).await?;
        let val: serde_json::Value = serde_json::from_str(&res)?;
        let response = val
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
        let mut request = serde_json::json!({
            "model": model,
            "prompt": prompt.user_message,
            "stream": false,
            "format": "json",
            "options": {
                "temperature": prompt.temperature,
            },
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
                .post(format!("{}/api/generate", self.config.host))
                .body(request.clone())
        })
        .await?;
        let res = genvm_modules_impl_common::read_response(res).await?;
        let val: serde_json::Value = serde_json::from_str(&res)?;
        let response = val
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
                "maxOutputTokens": 800,
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

        let res = genvm_modules_impl_common::read_response(res).await?;

        let res: serde_json::Value = serde_json::from_str(&res)?;

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
                "maxOutputTokens": 800,
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

        let res = genvm_modules_impl_common::read_response(res).await?;
        let res: serde_json::Value = serde_json::from_str(&res)?;

        let res = res
            .pointer("/candidates/0/content/parts/0/text")
            .and_then(|x| x.as_str())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res))?;

        Ok(res.to_owned())
    }
}

#[async_trait::async_trait]
impl Provider for Anthropic {
    async fn exec_prompt_text(
        &self,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<String> {
        let mut request = serde_json::json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt.user_message}],
            "max_tokens": 1000,
            "stream": false,
            "temperature": prompt.temperature,
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

        let res = genvm_modules_impl_common::read_response(res).await?;
        let val: serde_json::Value = serde_json::from_str(&res)?;
        val.pointer("/content/0/text")
            .and_then(|x| x.as_str())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res))
            .map(String::from)
    }

    async fn exec_prompt_json(
        &self,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<serde_json::Map<String, serde_json::Value>> {
        let mut request = serde_json::json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt.user_message}],
            "max_tokens": 1000,
            "stream": false,
            "temperature": prompt.temperature,
            "tools": [{
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

        let res = genvm_modules_impl_common::read_response(res).await?;
        let val: serde_json::Value = serde_json::from_str(&res)?;

        let val = val
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
            "max_tokens": 1000,
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

        let res = genvm_modules_impl_common::read_response(res).await?;
        let val: serde_json::Value = serde_json::from_str(&res)?;

        let val = val
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
) -> anyhow::Result<reqwest::Response> {
    let req = builder();

    log::trace!(request = genvm_modules_impl_common::censor_debug(&req), cookie = genvm_modules_impl_common::get_cookie(); "sending request");

    let res = req
        .send()
        .await
        .with_context(|| "sending request to llm provider")?;

    use reqwest::StatusCode;
    match res.status() {
        StatusCode::REQUEST_TIMEOUT
        | StatusCode::SERVICE_UNAVAILABLE
        | StatusCode::TOO_MANY_REQUESTS
        | StatusCode::GATEWAY_TIMEOUT => return Err(OverloadedError.into()),
        StatusCode::OK => return Ok(res),
        x if [529].contains(&x.as_u16()) => return Err(OverloadedError.into()),
        _ => {}
    }

    let debug = format!("{:?}", &res);
    let body = res.text().await;
    log::error!(
        response = genvm_modules_impl_common::censor_str(&debug),
        body:? = body,
        cookie = genvm_modules_impl_common::get_cookie();
        "request reading failed"
    );

    anyhow::bail!("llm request failed")
}
