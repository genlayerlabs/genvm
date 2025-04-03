use anyhow::Context;
use genvm_modules_impl_common::{MessageHandler, MessageHandlerProvider, ModuleResult};
use std::{collections::BTreeMap, sync::Arc};

use crate::config;
use genvm_modules_interfaces::llm as llm_iface;

struct Handler {
    config: Arc<config::Config>,
    client: reqwest::Client,
}

async fn send_with_retries(
    builder: impl (Fn() -> reqwest::RequestBuilder) + Send,
) -> anyhow::Result<reqwest::Response> {
    for i in 0..3 {
        let res = builder()
            .send()
            .await
            .with_context(|| "sending request to llm provider")?;
        if ![
            reqwest::StatusCode::REQUEST_TIMEOUT,
            reqwest::StatusCode::SERVICE_UNAVAILABLE,
            reqwest::StatusCode::GATEWAY_TIMEOUT,
        ]
        .contains(&res.status())
        {
            return Ok(res);
        }

        let debug = format!("{:?}", &res);
        let body = res.text().await;
        log::error!(response = genvm_modules_impl_common::CENSOR_RESPONSE.replace_all(&debug, "\"<censored>\": \"<censored>\""), body:? = body, retry = i; "llm request failed");

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }

    Err(anyhow::anyhow!("llm retries exceeded"))
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

pub struct HandlerProvider {
    pub config: Arc<config::Config>,
}

impl
    MessageHandlerProvider<
        genvm_modules_interfaces::llm::Message,
        genvm_modules_interfaces::llm::PromptAnswer,
    > for HandlerProvider
{
    async fn new_handler(
        &self,
    ) -> anyhow::Result<
        impl MessageHandler<
            genvm_modules_interfaces::llm::Message,
            genvm_modules_interfaces::llm::PromptAnswer,
        >,
    > {
        let client = reqwest::Client::new();

        Ok(Handler {
            config: self.config.clone(),
            client,
        })
    }
}

impl genvm_modules_impl_common::MessageHandler<llm_iface::Message, llm_iface::PromptAnswer>
    for Handler
{
    async fn handle(
        &self,
        message: llm_iface::Message,
    ) -> genvm_modules_impl_common::ModuleResult<llm_iface::PromptAnswer> {
        match message {
            llm_iface::Message::Prompt(payload) => self.exec_prompt(payload).await,
            llm_iface::Message::PromptTemplate(payload) => self.exec_prompt_template(payload).await,
        }
    }

    async fn cleanup(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

impl Handler {
    async fn exec_prompt_impl_anthropic(
        &self,
        prompt: &str,
        response_format: llm_iface::OutputFormat,
        provider: &config::BackendConfig,
    ) -> ModuleResult<String> {
        let mut request = serde_json::json!({
            "model": &provider.model,
            "messages": [{
                "role": "user",
                "content": prompt,
            }],
            "max_tokens": 1000,
            "stream": false,
            "temperature": 0.7,
        });
        match response_format {
            llm_iface::OutputFormat::Text => {}
            llm_iface::OutputFormat::JSON => {
                request.as_object_mut().unwrap().insert(
                    "tools".into(),
                    serde_json::json!([{
                        "name": "json_out",
                        "description": "Output a valid json object",
                        "input_schema": {
                            "type": "object"
                        }
                    }]),
                );
                request.as_object_mut().unwrap().insert(
                    "tool_choice".into(),
                    serde_json::json!({
                        "type": "tool",
                        "name": "json_out"
                    }),
                );
            }
        }

        let request = serde_json::to_vec(&request)?;
        let res = send_with_retries(|| {
            self.client
                .post(format!("{}/v1/messages", provider.host))
                .header("Content-Type", "application/json")
                .header("x-api-key", &provider.key)
                .header("anthropic-version", "2023-06-01")
                .body(request.clone())
        })
        .await?;

        let res = genvm_modules_impl_common::read_response(res).await?;
        let val: serde_json::Value = serde_json::from_str(&res)?;
        match response_format {
            llm_iface::OutputFormat::Text => val
                .pointer("/content/0/text")
                .and_then(|x| x.as_str())
                .ok_or(anyhow::anyhow!("can't get response field {}", &res))
                .map(String::from)
                .map(Ok),
            llm_iface::OutputFormat::JSON => val
                .pointer("/content/0/input/type")
                .ok_or(anyhow::anyhow!("can't get response field {}", &res))
                .and_then(|x| serde_json::to_string(x).map_err(Into::into))
                .map(Ok),
        }
    }

    async fn exec_prompt_impl_openai(
        &self,
        prompt: &str,
        response_format: llm_iface::OutputFormat,
        provider: &config::BackendConfig,
    ) -> ModuleResult<String> {
        let mut request = serde_json::json!({
            "model": &provider.model,
            "messages": [{
                "role": "user",
                "content": prompt,
            }],
            "max_tokens": 1000,
            "stream": false,
            "temperature": 0.7,
        });
        match response_format {
            llm_iface::OutputFormat::Text => {}
            llm_iface::OutputFormat::JSON => {
                request.as_object_mut().unwrap().insert(
                    "response_format".into(),
                    serde_json::json!({"type": "json_object"}),
                );
            }
        }
        let request = serde_json::to_vec(&request)?;
        let res = send_with_retries(|| {
            self.client
                .post(format!("{}/v1/chat/completions", provider.host))
                .header("Content-Type", "application/json")
                .header("Authorization", &format!("Bearer {}", &provider.key))
                .body(request.clone())
        })
        .await?;
        let res = genvm_modules_impl_common::read_response(res).await?;
        let val: serde_json::Value = serde_json::from_str(&res)?;
        let response = val
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("can't get response field {}", &res))?;

        // let total_tokens = val
        //     .pointer("/usage/total_tokens")
        //     .and_then(|v| v.as_u64())
        //     .ok_or(anyhow::anyhow!("can't get eval_duration field {}", &res))?;

        Ok(Ok(response.to_owned()))
    }

    async fn exec_prompt_impl_gemini(
        &self,
        prompt: &str,
        response_format: llm_iface::OutputFormat,
        provider: &config::BackendConfig,
    ) -> ModuleResult<String> {
        let request = serde_json::json!({
            "contents": [{
                "parts": [
                    {"text": prompt},
                ]
            }],
            "generationConfig": {
                "responseMimeType": match response_format {
                    llm_iface::OutputFormat::Text => "text/plain",
                    llm_iface::OutputFormat::JSON => "application/json",
                },
                "temperature": 0.7,
                "maxOutputTokens": 800,
            }
        });

        let request = serde_json::to_vec(&request)?;
        let res = send_with_retries(|| {
            self.client
                .post(format!(
                    "{}/v1beta/models/{}:generateContent?key={}",
                    provider.host, provider.model, provider.key
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
            .ok_or(anyhow::anyhow!("can't get response field {}", &res))?;
        Ok(Ok(res.into()))
    }

    async fn exec_prompt_impl_ollama(
        &self,
        prompt: &str,
        response_format: llm_iface::OutputFormat,
        provider: &config::BackendConfig,
    ) -> ModuleResult<String> {
        let mut request = serde_json::json!({
            "model": &provider.model,
            "prompt": prompt,
            "stream": false,
        });
        match response_format {
            llm_iface::OutputFormat::Text => {}
            llm_iface::OutputFormat::JSON => {
                request
                    .as_object_mut()
                    .unwrap()
                    .insert("format".into(), "json".into());
            }
        }

        let request = serde_json::to_vec(&request)?;
        let res = send_with_retries(|| {
            self.client
                .post(format!("{}/api/generate", provider.host))
                .body(request.clone())
        })
        .await?;
        let res = genvm_modules_impl_common::read_response(res).await?;
        let val: serde_json::Value = serde_json::from_str(&res)?;
        let response = val
            .as_object()
            .and_then(|v| v.get("response"))
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("can't get response field {}", &res))?;
        Ok(Ok(response.into()))
    }

    async fn exec_prompt_impl_simulator(
        &self,
        prompt: &str,
        response_format: llm_iface::OutputFormat,
        provider: &config::BackendConfig,
    ) -> ModuleResult<String> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "llm_genvm_module_call",
            "params": [&provider.model, prompt, serde_json::to_string(&response_format).unwrap()],
            "id": 1,
        });
        let request = serde_json::to_vec(&request)?;
        // no retries for the simulator
        let res = self
            .client
            .post(format!("{}/api", &provider.host))
            .header("Content-Type", "application/json")
            .body(request)
            .send()
            .await?;
        let res = genvm_modules_impl_common::read_response(res).await?;
        let res: serde_json::Value = serde_json::from_str(&res)?;
        res.pointer("/result/response")
            .and_then(|v| v.as_str())
            .map(String::from)
            .map(Ok)
            .ok_or(anyhow::anyhow!("can't get response field {}", &res))
    }

    async fn exec_prompt_in_provider(
        &self,
        prompt: &str,
        response_format: llm_iface::OutputFormat,
        provider: &config::BackendConfig,
    ) -> ModuleResult<llm_iface::PromptAnswer> {
        log::trace!(prompt = prompt, format:? = response_format; "executing prompt");
        let res_not_sanitized = match provider.provider {
            config::Provider::Ollama => {
                self.exec_prompt_impl_ollama(prompt, response_format, provider)
                    .await
            }
            config::Provider::OpenaiCompatible => {
                self.exec_prompt_impl_openai(prompt, response_format, provider)
                    .await
            }
            config::Provider::Simulator => {
                self.exec_prompt_impl_simulator(prompt, response_format, provider)
                    .await
            }
            config::Provider::Anthropic => {
                self.exec_prompt_impl_anthropic(prompt, response_format, provider)
                    .await
            }
            config::Provider::Google => {
                self.exec_prompt_impl_gemini(prompt, response_format, provider)
                    .await
            }
        }?;

        let res_not_sanitized = match res_not_sanitized {
            Ok(res_not_sanitized) => res_not_sanitized,
            Err(e) => return Ok(Err(e)),
        };

        match response_format {
            llm_iface::OutputFormat::Text => {
                Ok(Ok(llm_iface::PromptAnswer::Text(res_not_sanitized)))
            }
            llm_iface::OutputFormat::JSON => Ok(Ok(llm_iface::PromptAnswer::Text(
                sanitize_json_str(&res_not_sanitized).into(),
            ))),
        }
    }
}

impl Handler {
    async fn exec_prompt(
        &self,
        payload: llm_iface::PromptPayload,
    ) -> ModuleResult<llm_iface::PromptAnswer> {
        log::debug!(payload:serde = payload; "exec_prompt start");
        let llm_iface::PromptPart::Text(prompt) = &payload.parts[0];
        let res = self
            .exec_prompt_in_provider(
                prompt,
                payload.response_format,
                self.config.backends.first_key_value().unwrap().1,
            )
            .await;
        log::info!(payload:serde = payload, result:? = res;  "exec_prompt finished");
        res
    }

    async fn exec_prompt_template(
        &self,
        payload: llm_iface::PromptTemplatePayload,
    ) -> ModuleResult<llm_iface::PromptAnswer> {
        let provider = self.config.backends.first_key_value().unwrap().1;

        match payload {
            llm_iface::PromptTemplatePayload::EqNonComparativeLeader(payload) => {
                let vars = serde_json::to_value(payload.vars)?
                    .as_object()
                    .unwrap()
                    .to_owned();
                let vars: BTreeMap<String, String> = vars
                    .into_iter()
                    .map(|(k, v)| {
                        (
                            k,
                            match v {
                                serde_json::Value::String(s) => s,
                                _ => unreachable!(),
                            },
                        )
                    })
                    .collect();

                let new_prompt = genvm_common::templater::patch_str(
                    &vars,
                    &self.config.prompt_templates.eq_non_comparative_leader,
                    &genvm_common::templater::HASH_UNFOLDER_RE,
                )?;

                self.exec_prompt_in_provider(&new_prompt, llm_iface::OutputFormat::Text, provider)
                    .await
            }
            llm_iface::PromptTemplatePayload::EqComparative(payload) => {
                let vars = serde_json::to_value(payload.vars)?
                    .as_object()
                    .unwrap()
                    .to_owned();
                let vars: BTreeMap<String, String> = vars
                    .into_iter()
                    .map(|(k, v)| {
                        (
                            k,
                            match v {
                                serde_json::Value::String(s) => s,
                                _ => unreachable!(),
                            },
                        )
                    })
                    .collect();

                let new_prompt = genvm_common::templater::patch_str(
                    &vars,
                    &self.config.prompt_templates.eq_comparative,
                    &genvm_common::templater::HASH_UNFOLDER_RE,
                )?;

                let res = self
                    .exec_prompt_in_provider(&new_prompt, llm_iface::OutputFormat::JSON, provider)
                    .await?;
                let res = match res {
                    Ok(res) => res,
                    Err(e) => return Ok(Err(e)),
                };

                Ok(Ok(llm_iface::PromptAnswer::Bool(answer_is_bool(res)?)))
            }
            llm_iface::PromptTemplatePayload::EqNonComparativeValidator(payload) => {
                let vars = serde_json::to_value(payload.vars)?
                    .as_object()
                    .unwrap()
                    .to_owned();
                let vars: BTreeMap<String, String> = vars
                    .into_iter()
                    .map(|(k, v)| {
                        (
                            k,
                            match v {
                                serde_json::Value::String(s) => s,
                                _ => unreachable!(),
                            },
                        )
                    })
                    .collect();

                let new_prompt = genvm_common::templater::patch_str(
                    &vars,
                    &self.config.prompt_templates.eq_non_comparative_validator,
                    &genvm_common::templater::HASH_UNFOLDER_RE,
                )?;

                log::error!(old = self.config.prompt_templates.eq_non_comparative_validator, new = new_prompt, vars:serde = vars; "DEBUG");

                let res = self
                    .exec_prompt_in_provider(&new_prompt, llm_iface::OutputFormat::JSON, provider)
                    .await?;
                let res = match res {
                    Ok(res) => res,
                    Err(e) => return Ok(Err(e)),
                };

                Ok(Ok(llm_iface::PromptAnswer::Bool(answer_is_bool(res)?)))
            }
        }
    }
}

fn answer_is_bool(res: llm_iface::PromptAnswer) -> anyhow::Result<bool> {
    match res {
        llm_iface::PromptAnswer::Text(res) => {
            let map: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&res)?;
            map.get("result")
                .and_then(|x| x.as_bool())
                .ok_or(anyhow::anyhow!("invalid json"))
        }
        llm_iface::PromptAnswer::Bool(b) => Ok(b),
        llm_iface::PromptAnswer::Object(map) => map
            .get("result")
            .and_then(|x| x.as_bool())
            .ok_or(anyhow::anyhow!("invalid json")),
    }
}

#[cfg(test)]
#[allow(non_upper_case_globals, dead_code)]
mod tests {
    use std::sync::Arc;

    use crate::{config, handler::Handler};

    mod conf {
        pub const openai: &str = r#"{
            "host": "https://api.openai.com",
            "provider": "openai-compatible",
            "model": "gpt-4o-mini",
            "key_env_name": "OPENAIKEY",
        }"#;

        pub const heurist: &str = r#"{
            "host": "https://llm-gateway.heurist.xyz",
            "provider": "openai-compatible",
            "model": "meta-llama/llama-3.3-70b-instruct",
            "key_env_name": "HEURISTKEY",
        }"#;

        pub const anthropic: &str = r#"{
            "host": "https://api.anthropic.com",
            "provider": "anthropic",
            "model": "claude-3-5-sonnet-20241022",
            "key_env_name": "ANTHROPICKEY",
        }"#;

        pub const xai: &str = r#"{
            "host": "https://api.x.ai",
            "provider": "openai-compatible",
            "model": "grok-2-1212",
            "key_env_name": "XAIKEY",
        }"#;

        pub const google: &str = r#"{
            "host": "https://generativelanguage.googleapis.com",
            "provider": "google",
            "model": "gemini-1.5-flash",
            "key_env_name": "GEMINIKEY",
        }"#;

        pub const atoma: &str = r#"{
            "host": "https://api.atoma.network",
            "provider": "openai-compatible",
            "model": "meta-llama/Llama-3.3-70B-Instruct",
            "key_env_name": "ATOMAKEY",
        }"#;
    }

    async fn do_test_text(conf: &str) {
        let (cancellation, canceller) = genvm_common::cancellation::make();

        let backend: config::BackendConfig = serde_json::from_str(conf).unwrap();

        let imp = Handler {
            config: Arc::new(config::Config {
                bind_address: Default::default(),
                backends: [].into_iter().collect(),
                prompt_templates: config::PromptTemplates {
                    eq_comparative: Default::default(),
                    eq_non_comparative_leader: Default::default(),
                    eq_non_comparative_validator: Default::default(),
                },
                threads: 0,
                blocking_threads: 0,
            }),
            client: reqwest::Client::new(),
        };

        let res = imp
            .exec_prompt_in_provider(
                "Respond with \"yes\" (without quotes) and only this word",
                genvm_modules_interfaces::llm::OutputFormat::Text,
                &backend,
            )
            .await;

        let res = match res {
            Ok(res) => res,
            Err(ModuleError::Fatal(res))
                if format!("{}", res.root_cause()).contains("llm retries exceeded") =>
            {
                println!("WARNING: test skipped");
                return;
            }
            Err(e) => {
                panic!("err {:?}", e);
            }
        };

        std::mem::drop(canceller); // ensure that it lives up to here

        assert_eq!(res.to_lowercase().trim(), "yes")
    }

    async fn do_test_json(conf: &str) {
        use anyhow::Context;

        let (cancellation, canceller) = genvm_common::cancellation::make();

        let backend: config::BackendConfig = serde_json::from_str(conf).unwrap();

        let imp = Handler {
            config: Arc::new(config::Config {
                bind_address: Default::default(),
                backends: [].into_iter().collect(),
                prompt_templates: config::PromptTemplates {
                    eq_comparative: Default::default(),
                    eq_non_comparative_leader: Default::default(),
                    eq_non_comparative_validator: Default::default(),
                },
                threads: 0,
                blocking_threads: 0,
            }),
            client: reqwest::Client::new(),
        };

        const PROMPT: &str = "respond with json object containing single key \"result\" and associated value being a random integer from 0 to 100 (inclusive), it must be number, not wrapped in quotes";
        let res = imp
            .exec_prompt_in_provider(
                PROMPT,
                genvm_modules_interfaces::llm::OutputFormat::JSON,
                &backend,
            )
            .await;

        let res = match res {
            Ok(res) => res,
            Err(ModuleError::Fatal(res))
                if format!("{}", res.root_cause()).contains("llm retries exceeded") =>
            {
                println!("WARNING: test skipped");
                return;
            }
            Err(e) => {
                panic!("err {:?}", e);
            }
        };

        let res: serde_json::Value = serde_json::from_str(&res)
            .with_context(|| format!("result is {}", &res))
            .unwrap();

        std::mem::drop(canceller); // ensure that it lives up to here

        let res = res.as_object().unwrap();
        assert_eq!(res.len(), 1);
        let res = res.get("result").unwrap().as_i64().unwrap();
        assert!(res >= 0 && res <= 100)
    }

    macro_rules! make_test {
        ($conf:ident) => {
            mod $conf {
                #[tokio::test]
                async fn text() {
                    crate::handler::tests::do_test_text(crate::handler::tests::conf::$conf).await
                }
                #[tokio::test]
                async fn json() {
                    crate::handler::tests::do_test_json(crate::handler::tests::conf::$conf).await
                }
            }
        };
    }

    make_test!(openai);
    make_test!(anthropic);
    make_test!(google);
    make_test!(xai);

    //make_test!(heurist);
    //make_test!(atoma);
}
