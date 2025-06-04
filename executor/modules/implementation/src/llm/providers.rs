use crate::{common::ModuleResult, scripting};
use anyhow::Context;
use base64::Engine;

use super::{config, prompt};

#[async_trait::async_trait]
pub trait Provider {
    async fn exec_prompt_text(
        &self,
        client: &reqwest::Client,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<String>;

    async fn exec_prompt_json_as_text(
        &self,
        client: &reqwest::Client,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<String> {
        self.exec_prompt_text(client, prompt, model).await
    }

    async fn exec_prompt_json(
        &self,
        client: &reqwest::Client,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<serde_json::Map<String, serde_json::Value>> {
        let res = self.exec_prompt_json_as_text(client, prompt, model).await?;
        let res = sanitize_json_str(&res);
        let res = serde_json::from_str(res).with_context(|| format!("parsing {res:?}"))?;

        Ok(res)
    }

    async fn exec_prompt_bool_reason(
        &self,
        client: &reqwest::Client,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<bool> {
        let res = self.exec_prompt_json(client, prompt, model).await?;
        let res = res
            .get("result")
            .and_then(|x| x.as_bool())
            .ok_or_else(|| anyhow::anyhow!("can't get reason from `{:?}`", res))?;
        Ok(res)
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

#[async_trait::async_trait]
impl Provider for OpenAICompatible {
    async fn exec_prompt_text(
        &self,
        client: &reqwest::Client,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<String> {
        let mut request = serde_json::json!({
            "model": model,
            "messages": prompt.to_openai_messages()?,
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
        let url = format!("{}/v1/chat/completions", self.config.host);
        let request = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", &format!("Bearer {}", &self.config.key))
            .body(request.clone());
        let res =
            scripting::send_request_get_lua_compatible_response_json(&url, request, true).await?;

        let response = res
            .body
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res.body))?;

        Ok(response.to_owned())
    }

    async fn exec_prompt_json(
        &self,
        client: &reqwest::Client,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<serde_json::Map<String, serde_json::Value>> {
        let mut request = serde_json::json!({
            "model": model,
            "messages": prompt.to_openai_messages()?,
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
        let url = format!("{}/v1/chat/completions", self.config.host);
        let request = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", &format!("Bearer {}", &self.config.key))
            .body(request.clone());
        let res =
            scripting::send_request_get_lua_compatible_response_json(&url, request, true).await?;

        let response = res
            .body
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res.body))?;

        let response = sanitize_json_str(response);
        let response =
            serde_json::from_str(response).with_context(|| format!("parsing {response:?}"))?;

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
        client: &reqwest::Client,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<String> {
        let request = prompt.to_ollama_no_format(model);

        let request = serde_json::to_vec(&request)?;
        let url = format!("{}/api/generate", self.config.host);
        let request = client.post(&url).body(request.clone());
        let res =
            scripting::send_request_get_lua_compatible_response_json(&url, request, true).await?;

        let response = res
            .body
            .as_object()
            .and_then(|v| v.get("response"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res.body))?;
        Ok(response.to_owned())
    }

    async fn exec_prompt_json_as_text(
        &self,
        client: &reqwest::Client,
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
        let url = format!("{}/api/generate", self.config.host);
        let request = client.post(&url).body(request.clone());
        let res =
            scripting::send_request_get_lua_compatible_response_json(&url, request, true).await?;

        let response = res
            .body
            .as_object()
            .and_then(|v| v.get("response"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res.body))?;
        Ok(response.to_owned())
    }
}

#[async_trait::async_trait]
impl Provider for Gemini {
    async fn exec_prompt_text(
        &self,
        client: &reqwest::Client,
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

        prompt.add_gemini_messages(request.as_object_mut().unwrap())?;

        let request = serde_json::to_vec(&request)?;
        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            self.config.host, model, self.config.key
        );
        let request = client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(request.clone());
        let res =
            scripting::send_request_get_lua_compatible_response_json(&url, request, true).await?;

        let res = res
            .body
            .pointer("/candidates/0/content/parts/0/text")
            .and_then(|x| x.as_str())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res.body))?;
        Ok(res.into())
    }

    async fn exec_prompt_json_as_text(
        &self,
        client: &reqwest::Client,
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

        prompt.add_gemini_messages(request.as_object_mut().unwrap())?;

        let request = serde_json::to_vec(&request)?;
        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            self.config.host, model, self.config.key
        );
        let request = client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(request.clone());
        let res =
            scripting::send_request_get_lua_compatible_response_json(&url, request, true).await?;

        let res = res
            .body
            .pointer("/candidates/0/content/parts/0/text")
            .and_then(|x| x.as_str())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res.body))?;

        Ok(res.to_owned())
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

#[async_trait::async_trait]
impl Provider for Anthropic {
    async fn exec_prompt_text(
        &self,
        client: &reqwest::Client,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<String> {
        let request = prompt.to_anthropic_no_format(model)?;

        let request = serde_json::to_vec(&request)?;
        let url = format!("{}/v1/messages", self.config.host);
        let request = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.config.key)
            .header("anthropic-version", "2023-06-01")
            .body(request.clone());
        let res =
            scripting::send_request_get_lua_compatible_response_json(&url, request, true).await?;

        res.body
            .pointer("/content/0/text")
            .and_then(|x| x.as_str())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res.body))
            .map(String::from)
    }

    async fn exec_prompt_json(
        &self,
        client: &reqwest::Client,
        prompt: &prompt::Internal,
        model: &str,
    ) -> ModuleResult<serde_json::Map<String, serde_json::Value>> {
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
        let request = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.config.key)
            .header("anthropic-version", "2023-06-01")
            .body(request.clone());
        let res =
            scripting::send_request_get_lua_compatible_response_json(&url, request, true).await?;

        let val = res
            .body
            .pointer("/content/0/input")
            .and_then(|x| x.as_object())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res.body))?;

        Ok(val.clone())
    }

    async fn exec_prompt_bool_reason(
        &self,
        client: &reqwest::Client,
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
        let url = format!("{}/v1/messages", self.config.host);
        let request = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.config.key)
            .header("anthropic-version", "2023-06-01")
            .body(request.clone());
        let res =
            scripting::send_request_get_lua_compatible_response_json(&url, request, true).await?;

        let val = res
            .body
            .pointer("/content/0/input/result")
            .and_then(|x| x.as_bool())
            .ok_or_else(|| anyhow::anyhow!("can't get response field {}", &res.body))?;

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

#[cfg(test)]
#[allow(non_upper_case_globals, dead_code)]
mod tests {
    use std::collections::HashMap;

    use crate::common;

    use super::super::{config, prompt};
    use genvm_common::templater;

    mod conf {
        pub const openai: &str = r#"{
            "host": "https://api.openai.com",
            "provider": "openai-compatible",
            "models": {
                "gpt-4o-mini": { "supports_json": true }
            },
            "key": "${ENV[OPENAIKEY]}"
        }"#;

        pub const heurist: &str = r#"{
            "host": "https://llm-gateway.heurist.xyz",
            "provider": "openai-compatible",
            "models": {
                "meta-llama/llama-3.3-70b-instruct": { "supports_json": true }
            },
            "key": "${ENV[HEURISTKEY]}"
        }"#;

        pub const heurist_deepseek: &str = r#"{
            "host": "https://llm-gateway.heurist.xyz",
            "provider": "openai-compatible",
            "models": {
                "deepseek/deepseek-v3": { "supports_json": true }
            },
            "key": "${ENV[HEURISTKEY]}"
        }"#;

        pub const anthropic: &str = r#"{
            "host": "https://api.anthropic.com",
            "provider": "anthropic",
            "models": { "claude-3-5-sonnet-20241022" : {} },
            "key": "${ENV[ANTHROPICKEY]}"
        }"#;

        pub const xai: &str = r#"{
            "host": "https://api.x.ai",
            "provider": "openai-compatible",
            "models": { "grok-2-1212" : { "supports_json": true } },
            "key": "${ENV[XAIKEY]}"
        }"#;

        pub const google: &str = r#"{
            "host": "https://generativelanguage.googleapis.com",
            "provider": "google",
            "models": { "gemini-1.5-flash": { "supports_json": true } },
            "key": "${ENV[GEMINIKEY]}"
        }"#;

        pub const atoma: &str = r#"{
            "host": "https://api.atoma.network",
            "provider": "openai-compatible",
            "models": { "meta-llama/Llama-3.3-70B-Instruct": {} },
            "key": "${ENV[ATOMAKEY]}"
        }"#;
    }

    async fn do_test_text(conf: &str) {
        common::tests::setup();

        let backend: serde_json::Value = serde_json::from_str(conf).unwrap();
        let mut vars = HashMap::new();
        for (mut name, value) in std::env::vars() {
            name.insert_str(0, "ENV[");
            name.push(']');

            vars.insert(name, value);
        }
        let backend =
            genvm_common::templater::patch_json(&vars, backend, &templater::DOLLAR_UNFOLDER_RE)
                .unwrap();
        let backend: config::BackendConfig = serde_json::from_value(backend).unwrap();
        let provider = backend.to_provider();

        let client = common::create_client().unwrap();

        let res = provider
            .exec_prompt_text(
                &client,
                &prompt::Internal {
                    system_message: None,
                    temperature: 0.7,
                    user_message: "Respond with a single word \"yes\" (without quotes) and only this word, lowercase".to_owned(),
                    images: Vec::new(),
                    max_tokens: 100,
                    use_max_completion_tokens: true,
                },
                backend.script_config.models.first_key_value().unwrap().0,
            )
            .await
            .unwrap();

        let res = res.trim().to_lowercase();

        assert_eq!(res, "yes");
    }

    async fn do_test_json(conf: &str) {
        common::tests::setup();

        let backend: serde_json::Value = serde_json::from_str(conf).unwrap();
        let mut vars = HashMap::new();
        for (mut name, value) in std::env::vars() {
            name.insert_str(0, "ENV[");
            name.push(']');

            vars.insert(name, value);
        }
        let backend =
            genvm_common::templater::patch_json(&vars, backend, &templater::DOLLAR_UNFOLDER_RE)
                .unwrap();
        let backend: config::BackendConfig = serde_json::from_value(backend).unwrap();

        if !backend
            .script_config
            .models
            .first_key_value()
            .unwrap()
            .1
            .supports_json
        {
            return;
        }

        let provider = backend.to_provider();

        let client = common::create_client().unwrap();

        const PROMPT: &str = r#"respond with json object containing single key "result" and associated value being a random integer from 0 to 100 (inclusive), it must be number, not wrapped in quotes. This object must not be wrapped into other objects. Example: {"result": 10}"#;
        let res = provider
            .exec_prompt_json(
                &client,
                &prompt::Internal {
                    system_message: Some("respond with json".to_owned()),
                    temperature: 0.7,
                    user_message: PROMPT.to_owned(),
                    images: Vec::new(),
                    max_tokens: 100,
                    use_max_completion_tokens: true,
                },
                backend.script_config.models.first_key_value().unwrap().0,
            )
            .await;
        eprintln!("{res:?}");
        let res = res.unwrap();

        let as_val = serde_json::Value::Object(res);

        // all this because of anthropic
        for potential in [
            as_val.pointer("/result").and_then(|x| x.as_i64()),
            as_val.pointer("/root/result").and_then(|x| x.as_i64()),
            as_val.pointer("/json/result").and_then(|x| x.as_i64()),
            as_val.pointer("/type/result").and_then(|x| x.as_i64()),
            as_val.pointer("/object/result").and_then(|x| x.as_i64()),
            as_val.pointer("/value/result").and_then(|x| x.as_i64()),
            as_val.pointer("/data/result").and_then(|x| x.as_i64()),
            as_val.pointer("/response/result").and_then(|x| x.as_i64()),
            as_val.pointer("/answer/result").and_then(|x| x.as_i64()),
        ] {
            if let Some(v) = potential {
                assert!((0..=100).contains(&v));
                return;
            }
        }
        unreachable!("no result found in {as_val:?}");
    }

    macro_rules! make_test {
        ($conf:ident) => {
            mod $conf {
                use crate::common;

                #[tokio::test]
                async fn text() {
                    let conf = super::conf::$conf;
                    common::test_with_cookie(conf, async { super::do_test_text(conf).await }).await;
                }
                #[tokio::test]
                async fn json() {
                    let conf = super::conf::$conf;
                    common::test_with_cookie(conf, async { super::do_test_json(conf).await }).await;
                }
            }
        };
    }

    make_test!(openai);
    make_test!(anthropic);
    make_test!(google);
    make_test!(xai);

    make_test!(heurist);
    make_test!(heurist_deepseek);
    //make_test!(atoma);
}
