use crate::{common::ModuleResult, scripting};
use anyhow::Context as _;
use base64::Engine;
use genvm_common::*;

use super::{config, prompt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokensSanityError {
    ZeroTotal,
    TotalLessThanParts { total: u32, input: u32, output: u32 },
}

impl std::fmt::Display for TokensSanityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokensSanityError::ZeroTotal => write!(f, "total tokens is zero"),
            TokensSanityError::TotalLessThanParts {
                total,
                input,
                output,
            } => {
                write!(f, "total ({total}) < input ({input}) + output ({output})")
            }
        }
    }
}

impl std::error::Error for TokensSanityError {}

#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub input: Option<u32>,
    pub output: Option<u32>,
    pub total: Option<u32>,
    pub cache_read_tokens: Option<u32>,
    pub cache_write_tokens: Option<u32>,
    pub image_units: Option<u32>,
    pub raw_usage: serde_json::Value,
}

#[allow(dead_code)]
impl TokenUsage {
    pub fn new(input: Option<u32>, output: Option<u32>, total: Option<u32>) -> Self {
        Self {
            input,
            output,
            total,
            ..Default::default()
        }
    }

    pub fn from_input_output(input: u32, output: u32) -> Self {
        Self {
            input: Some(input),
            output: Some(output),
            total: Some(input + output),
            ..Default::default()
        }
    }

    pub fn from_total(total: u32) -> Self {
        Self {
            total: Some(total),
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub fn sanity_check(&self) -> Result<(), TokensSanityError> {
        let total = self.total.unwrap_or_default();
        if total == 0 {
            return Err(TokensSanityError::ZeroTotal);
        }
        let input = self.input.unwrap_or_default();
        let output = self.output.unwrap_or_default();
        if total < input + output {
            return Err(TokensSanityError::TotalLessThanParts {
                total,
                input,
                output,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ProviderResponse<T> {
    pub result: T,
    pub tokens: TokenUsage,
}

impl<T> ProviderResponse<T> {
    pub fn new(result: T, tokens: TokenUsage) -> Self {
        Self { result, tokens }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> ProviderResponse<U> {
        ProviderResponse {
            result: f(self.result),
            tokens: self.tokens,
        }
    }
}

use super::merge;

#[async_trait::async_trait]
pub trait Provider {
    async fn exec_prompt_text(
        &self,
        ctx: &scripting::CtxPart,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<ProviderResponse<String>>;

    async fn exec_prompt_json_as_text(
        &self,
        ctx: &scripting::CtxPart,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<ProviderResponse<String>> {
        let res = self.exec_prompt_json(ctx, prompt, model).await?;
        let serialized = serde_json::to_string(&res.result)?;
        Ok(ProviderResponse::new(serialized, res.tokens))
    }

    async fn exec_prompt_json(
        &self,
        ctx: &scripting::CtxPart,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<ProviderResponse<serde_json::Map<String, serde_json::Value>>> {
        let res = self.exec_prompt_json_as_text(ctx, prompt, model).await?;
        let json_str = sanitize_json_str(&res.result);
        let parsed =
            serde_json::from_str(&json_str).with_context(|| format!("parsing {json_str:?}"))?;

        Ok(ProviderResponse::new(parsed, res.tokens))
    }

    async fn exec_prompt_bool_reason(
        &self,
        ctx: &scripting::CtxPart,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<ProviderResponse<bool>> {
        let res = self.exec_prompt_json(ctx, prompt, model).await?;
        let result_val = res.result.get("result").and_then(|x| x.as_bool());

        if let Some(val) = result_val {
            Ok(ProviderResponse::new(val, res.tokens))
        } else {
            log_error!(result:? = res.result; "no result in reason, returning false");

            Ok(ProviderResponse::new(false, res.tokens))
        }
    }
}

pub struct OpenAICompatible {
    pub(crate) config: config::BackendConfig,
}

pub struct Gemini {
    pub(crate) config: config::BackendConfig,
}

pub struct OLlama {
    pub(crate) config: config::BackendConfig,
}

pub struct Anthropic {
    pub(crate) config: config::BackendConfig,
}

impl prompt::Internal {
    fn to_openai_messages(&self) -> ModuleResult<Vec<serde_json::Value>> {
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
            let kind = img.kind_or_error()?;
            encoded.push_str(kind.media_type());
            encoded.push_str(";base64,");
            base64::prelude::BASE64_STANDARD.encode_string(&img.0, &mut encoded);

            user_content.push(serde_json::json!({
                "type": "image_url",
                "image_url": { "url": encoded },
            }));
        }

        messages.push(serde_json::json!({
            "role": "user",
            "content": user_content,
        }));

        Ok(messages)
    }

    fn add_gemini_messages(
        &self,
        to: &mut serde_json::Map<String, serde_json::Value>,
    ) -> ModuleResult<()> {
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
            let kind = img.kind_or_error()?;
            parts.push(serde_json::json!({
                "inline_data": {
                    "mime_type": kind.media_type(),
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

        Ok(())
    }
}

fn extract_openai_tokens(body: &serde_json::Value) -> TokenUsage {
    let usage = body.pointer("/usage");
    let input = usage
        .and_then(|v| v.get("prompt_tokens"))
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);
    let output = usage
        .and_then(|v| v.get("completion_tokens"))
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);
    let total = usage
        .and_then(|v| v.get("total_tokens"))
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);
    let cache_read_tokens = usage
        .and_then(|v| v.pointer("/prompt_tokens_details/cached_tokens"))
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);
    let cache_write_tokens = usage
        .and_then(|v| v.pointer("/prompt_tokens_details/cache_write_tokens"))
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);
    TokenUsage {
        input,
        output,
        total,
        cache_read_tokens,
        cache_write_tokens,
        raw_usage: usage.cloned().unwrap_or_default(),
        ..Default::default()
    }
}

#[async_trait::async_trait]
impl Provider for OpenAICompatible {
    async fn exec_prompt_text(
        &self,
        ctx: &scripting::CtxPart,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<ProviderResponse<String>> {
        let mut request = serde_json::json!({
            "model": model,
            "messages": prompt.to_openai_messages()?,
            "stream": false,
            "temperature": prompt.temperature,
        });

        if let Some(seed) = prompt.seed {
            request
                .as_object_mut()
                .unwrap()
                .insert("seed".to_owned(), seed.into());
        }

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

        merge::merge_extra(
            &mut request,
            serde_json::Value::Object(prompt.extra.clone()),
            prompt.extra_merge_strategy.clone(),
        )?;

        let url = format!("{}/v1/chat/completions", self.config.host);
        log_trace!(request:serde = request, url = url; "final request body after merging extra");

        let request = serde_json::to_vec(&request)?;
        let request = ctx
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", &format!("Bearer {}", &self.config.key))
            .body(request.clone());
        let res = scripting::send_request_get_lua_compatible_response_json(
            &ctx.metrics,
            &url,
            prompt.apply_timeout(request),
            true,
            usize::MAX,
        )
        .await?;

        let tokens = extract_openai_tokens(&res.body);

        let response = res
            .body
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res.body))?;

        Ok(ProviderResponse::new(response.to_owned(), tokens))
    }

    async fn exec_prompt_json_as_text(
        &self,
        ctx: &scripting::CtxPart,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<ProviderResponse<String>> {
        let mut request = serde_json::json!({
            "model": model,
            "messages": prompt.to_openai_messages()?,
            "stream": false,
            "temperature": prompt.temperature,
            "response_format": {"type": "json_object"},
        });

        if let Some(seed) = prompt.seed {
            request
                .as_object_mut()
                .unwrap()
                .insert("seed".to_owned(), seed.into());
        }

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

        merge::merge_extra(
            &mut request,
            serde_json::Value::Object(prompt.extra.clone()),
            prompt.extra_merge_strategy.clone(),
        )?;

        let url = format!("{}/v1/chat/completions", self.config.host);
        log_trace!(request:serde = request, url = url; "final request body after merging extra");

        let request = serde_json::to_vec(&request)?;
        let request = ctx
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", &format!("Bearer {}", &self.config.key))
            .body(request.clone());
        let res = scripting::send_request_get_lua_compatible_response_json(
            &ctx.metrics,
            &url,
            prompt.apply_timeout(request),
            true,
            usize::MAX,
        )
        .await?;

        let tokens = extract_openai_tokens(&res.body);

        let response = res
            .body
            .pointer("/choices/0/message/content")
            .and_then(|v| if v.is_null() { Some("") } else { v.as_str() })
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res.body))?;

        Ok(ProviderResponse::new(sanitize_json_str(response), tokens))
    }
}

impl prompt::Internal {
    fn to_ollama_no_format(&self, model: &str) -> serde_json::Value {
        let mut options = serde_json::json!({
            "temperature": self.temperature,
            "num_predict": self.max_tokens,
        });

        if let Some(seed) = self.seed {
            options
                .as_object_mut()
                .unwrap()
                .insert("seed".to_owned(), seed.into());
        }

        let mut request = serde_json::json!({
            "model": model,
            "prompt": self.user_message,
            "stream": false,
            "options": options,
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

fn extract_ollama_tokens(body: &serde_json::Value) -> TokenUsage {
    let input = body
        .get("prompt_eval_count")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);
    let output = body
        .get("eval_count")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);
    let total = match (input, output) {
        (Some(i), Some(o)) => Some(i + o),
        _ => None,
    };
    let mut raw = serde_json::Map::new();
    if let Some(v) = body.get("prompt_eval_count") {
        raw.insert("prompt_eval_count".into(), v.clone());
    }
    if let Some(v) = body.get("eval_count") {
        raw.insert("eval_count".into(), v.clone());
    }
    TokenUsage {
        input,
        output,
        total,
        raw_usage: serde_json::Value::Object(raw),
        ..Default::default()
    }
}

#[async_trait::async_trait]
impl Provider for OLlama {
    async fn exec_prompt_text(
        &self,
        ctx: &scripting::CtxPart,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<ProviderResponse<String>> {
        if !prompt.extra.is_empty() {
            log_warn!(extra:serde = prompt.extra; "ollama provider ignores extra body fields");
        }
        let request = prompt.to_ollama_no_format(model);
        let url = format!("{}/api/ generate", self.config.host);
        log_trace!(request:serde = request, url = url; "final request body after merging extra");
        let request = serde_json::to_vec(&request)?;
        let request = ctx.client.post(&url).body(request.clone());
        let res = scripting::send_request_get_lua_compatible_response_json(
            &ctx.metrics,
            &url,
            prompt.apply_timeout(request),
            true,
            usize::MAX,
        )
        .await?;

        let tokens = extract_ollama_tokens(&res.body);

        let response = res
            .body
            .as_object()
            .and_then(|v| v.get("response"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res.body))?;
        Ok(ProviderResponse::new(response.to_owned(), tokens))
    }

    async fn exec_prompt_json_as_text(
        &self,
        ctx: &scripting::CtxPart,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<ProviderResponse<String>> {
        if !prompt.extra.is_empty() {
            log_warn!(extra:serde = prompt.extra; "ollama provider ignores extra body fields");
        }
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

        let url = format!("{}/api/generate", self.config.host);
        log_trace!(request:serde = request, url = url; "final request body after merging extra");
        let request = serde_json::to_vec(&request)?;
        let request = ctx.client.post(&url).body(request.clone());
        let res = scripting::send_request_get_lua_compatible_response_json(
            &ctx.metrics,
            &url,
            prompt.apply_timeout(request),
            true,
            usize::MAX,
        )
        .await?;

        let tokens = extract_ollama_tokens(&res.body);

        let response = res
            .body
            .as_object()
            .and_then(|v| v.get("response"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res.body))?;
        Ok(ProviderResponse::new(sanitize_json_str(response), tokens))
    }
}

fn extract_gemini_tokens(body: &serde_json::Value) -> TokenUsage {
    let usage = body.pointer("/usageMetadata");
    let input = usage
        .and_then(|v| v.get("promptTokenCount"))
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);
    let output = usage
        .and_then(|v| v.get("candidatesTokenCount"))
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);
    let total = usage
        .and_then(|v| v.get("totalTokenCount"))
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);
    let cache_read_tokens = usage
        .and_then(|v| v.get("cachedContentTokenCount"))
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);
    TokenUsage {
        input,
        output,
        total,
        cache_read_tokens,
        raw_usage: usage.cloned().unwrap_or_default(),
        ..Default::default()
    }
}

#[async_trait::async_trait]
impl Provider for Gemini {
    async fn exec_prompt_text(
        &self,
        ctx: &scripting::CtxPart,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<ProviderResponse<String>> {
        if !prompt.extra.is_empty() {
            log_warn!(extra:serde = prompt.extra; "gemini provider ignores extra body fields");
        }
        let mut request = serde_json::json!({
            "generationConfig": {
                "responseMimeType": "text/plain",
                "temperature": prompt.temperature,
                "maxOutputTokens": prompt.max_tokens,
            }
        });

        prompt.add_gemini_messages(request.as_object_mut().unwrap())?;

        let request = serde_json::to_vec(&request)?;
        let url = format!(
            "{}/v1beta/models/{}:generateContent",
            self.config.host, model,
        );
        log_trace!(request:serde = request, url = url; "final request body");

        let request = ctx
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-goog-api-key", &self.config.key)
            .body(request.clone());
        let res_json = scripting::send_request_get_lua_compatible_response_json(
            &ctx.metrics,
            &url,
            prompt.apply_timeout(request),
            true,
            usize::MAX,
        )
        .await?;

        let tokens = extract_gemini_tokens(&res_json.body);

        let res = res_json
            .body
            .pointer("/candidates/0/content/parts/0/text")
            .and_then(|x| x.as_str());

        if res.is_none()
            && res_json
                .body
                .pointer("/candidates/0/finishReason")
                .and_then(|x| x.as_str())
                == Some("MAX_TOKENS")
        {
            return Ok(ProviderResponse::new("".into(), tokens));
        }

        let res =
            res.ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res_json.body))?;
        Ok(ProviderResponse::new(res.into(), tokens))
    }

    async fn exec_prompt_json_as_text(
        &self,
        ctx: &scripting::CtxPart,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<ProviderResponse<String>> {
        if !prompt.extra.is_empty() {
            log_warn!(extra:serde = prompt.extra; "gemini provider ignores extra body fields");
        }
        let mut request = serde_json::json!({
            "generationConfig": {
                "responseMimeType": "application/json",
                "temperature": prompt.temperature,
                "maxOutputTokens": prompt.max_tokens,
            }
        });

        prompt.add_gemini_messages(request.as_object_mut().unwrap())?;

        let url = format!(
            "{}/v1beta/models/{}:generateContent",
            self.config.host, model,
        );
        log_trace!(request:serde = request, url = url; "final request body after merging extra");
        let request = serde_json::to_vec(&request)?;
        let request = ctx
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-goog-api-key", &self.config.key)
            .body(request.clone());
        let res_json = scripting::send_request_get_lua_compatible_response_json(
            &ctx.metrics,
            &url,
            prompt.apply_timeout(request),
            true,
            usize::MAX,
        )
        .await?;

        let tokens = extract_gemini_tokens(&res_json.body);

        let res = res_json
            .body
            .pointer("/candidates/0/content/parts/0/text")
            .and_then(|x| x.as_str());

        if !res.map(|x| x.starts_with("{")).unwrap_or(false)
            && res_json
                .body
                .pointer("/candidates/0/finishReason")
                .and_then(|x| x.as_str())
                == Some("MAX_TOKENS")
        {
            return Ok(ProviderResponse::new("{}".to_owned(), tokens));
        }

        let res =
            res.ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res_json.body))?;

        Ok(ProviderResponse::new(sanitize_json_str(res), tokens))
    }
}

impl prompt::Internal {
    fn to_anthropic_no_format(&self, model: &str) -> ModuleResult<serde_json::Value> {
        let mut user_content = Vec::new();

        for img in &self.images {
            let kind = img.kind_or_error()?;
            user_content.push(serde_json::json!({"type": "image", "source": {
                "type": "base64",
                "media_type": kind.media_type(),
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

        Ok(request)
    }
}

fn extract_anthropic_tokens(body: &serde_json::Value) -> TokenUsage {
    let usage = body.pointer("/usage");
    let input = usage
        .and_then(|v| v.get("input_tokens"))
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);
    let output = usage
        .and_then(|v| v.get("output_tokens"))
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);
    let total = match (input, output) {
        (Some(i), Some(o)) => Some(i + o),
        _ => None,
    };
    let cache_read_tokens = usage
        .and_then(|v| v.get("cache_read_input_tokens"))
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);
    let cache_write_tokens = usage
        .and_then(|v| v.get("cache_creation_input_tokens"))
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);
    TokenUsage {
        input,
        output,
        total,
        cache_read_tokens,
        cache_write_tokens,
        raw_usage: usage.cloned().unwrap_or_default(),
        ..Default::default()
    }
}

#[async_trait::async_trait]
impl Provider for Anthropic {
    async fn exec_prompt_text(
        &self,
        ctx: &scripting::CtxPart,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<ProviderResponse<String>> {
        if !prompt.extra.is_empty() {
            log_warn!(extra:serde = prompt.extra; "anthropic provider ignores extra body fields");
        }
        let request = prompt.to_anthropic_no_format(model)?;
        let url = format!("{}/v1/messages", self.config.host);
        log_trace!(request:serde = request, url = url; "final request body");
        let request = serde_json::to_vec(&request)?;
        let request = ctx
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.config.key)
            .header("anthropic-version", "2023-06-01")
            .body(request.clone());
        let res = scripting::send_request_get_lua_compatible_response_json(
            &ctx.metrics,
            &url,
            prompt.apply_timeout(request),
            true,
            usize::MAX,
        )
        .await?;

        let tokens = extract_anthropic_tokens(&res.body);

        let text = res
            .body
            .pointer("/content/0/text")
            .and_then(|x| x.as_str())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res.body))?;

        Ok(ProviderResponse::new(String::from(text), tokens))
    }

    async fn exec_prompt_json(
        &self,
        ctx: &scripting::CtxPart,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<ProviderResponse<serde_json::Map<String, serde_json::Value>>> {
        if !prompt.extra.is_empty() {
            log_warn!(extra:serde = prompt.extra; "anthropic provider ignores extra body fields");
        }
        let mut request = prompt.to_anthropic_no_format(model)?;

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
        let url = format!("{}/v1/messages", self.config.host);
        log_trace!(request:serde = request, url = url; "final request body");
        let request = ctx
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.config.key)
            .header("anthropic-version", "2023-06-01")
            .body(request.clone());
        let res = scripting::send_request_get_lua_compatible_response_json(
            &ctx.metrics,
            &url,
            prompt.apply_timeout(request),
            true,
            usize::MAX,
        )
        .await?;

        let tokens = extract_anthropic_tokens(&res.body);

        let val = res
            .body
            .pointer("/content/0/input")
            .and_then(|x| x.as_object())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res.body))?;

        Ok(ProviderResponse::new(val.clone(), tokens))
    }

    async fn exec_prompt_bool_reason(
        &self,
        ctx: &scripting::CtxPart,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<ProviderResponse<bool>> {
        if !prompt.extra.is_empty() {
            log_warn!(extra:serde = prompt.extra; "anthropic provider ignores extra body fields");
        }
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

        let url = format!("{}/v1/messages", self.config.host);
        log_trace!(request:serde = request, url = url; "final request body after merging extra");
        let request = serde_json::to_vec(&request)?;
        let request = ctx
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.config.key)
            .header("anthropic-version", "2023-06-01")
            .body(request.clone());
        let res = scripting::send_request_get_lua_compatible_response_json(
            &ctx.metrics,
            &url,
            prompt.apply_timeout(request),
            true,
            usize::MAX,
        )
        .await?;

        let tokens = extract_anthropic_tokens(&res.body);

        let val = res
            .body
            .pointer("/content/0/input/result")
            .and_then(|x| x.as_bool())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res.body))?;

        Ok(ProviderResponse::new(val, tokens))
    }
}

fn sanitize_json_str(s: &str) -> String {
    let s = s.trim();
    let s = s
        .strip_prefix("```json")
        .or(s.strip_prefix("```"))
        .unwrap_or(s);
    let s = s.strip_suffix("```").unwrap_or(s);
    let s = s.trim();

    crate::complete_json(s)
}
