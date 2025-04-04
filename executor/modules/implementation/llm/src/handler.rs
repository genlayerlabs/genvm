use anyhow::Context;
use genvm_modules_impl_common::{MessageHandler, MessageHandlerProvider, ModuleResult};
use std::{collections::BTreeMap, sync::Arc};

use crate::{config, scripting};
use genvm_modules_interfaces::llm as llm_iface;

pub struct HandlerInner {
    pub config: Arc<config::Config>,
    client: reqwest::Client,
}

pub struct Handler {
    pub inner: HandlerInner,
    user_vm: Arc<scripting::UserVM>,
}

#[derive(Debug)]
pub struct OverloadedError;

impl std::error::Error for OverloadedError {}

impl std::fmt::Display for OverloadedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OverloadedError")
    }
}

async fn send_with_retries(
    builder: impl (Fn() -> reqwest::RequestBuilder) + Send,
) -> anyhow::Result<reqwest::Response> {
    let res = builder()
        .send()
        .await
        .with_context(|| "sending request to llm provider")?;

    use reqwest::StatusCode;
    match res.status() {
        StatusCode::REQUEST_TIMEOUT
        | StatusCode::SERVICE_UNAVAILABLE
        | StatusCode::GATEWAY_TIMEOUT => return Err(OverloadedError.into()),
        StatusCode::OK => return Ok(res),
        _ => {}
    }

    let debug = format!("{:?}", &res);
    let body = res.text().await;
    log::error!(response = genvm_modules_impl_common::CENSOR_RESPONSE.replace_all(&debug, "\"<censored>\": \"<censored>\""), body:? = body; "request reading failed");

    anyhow::bail!("llm request failed")
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
    pub user_vm: Arc<scripting::UserVM>,
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

        Ok(HandlerWrapper(Arc::new(Handler {
            inner: HandlerInner {
                config: self.config.clone(),
                client,
            },
            user_vm: self.user_vm.clone(),
        })))
    }
}

struct HandlerWrapper(Arc<Handler>);

impl genvm_modules_impl_common::MessageHandler<llm_iface::Message, llm_iface::PromptAnswer>
    for HandlerWrapper
{
    async fn handle(
        &self,
        message: llm_iface::Message,
    ) -> genvm_modules_impl_common::ModuleResult<llm_iface::PromptAnswer> {
        match message {
            llm_iface::Message::Prompt(payload) => {
                self.0.exec_prompt(self.0.clone(), payload).await
            }
            llm_iface::Message::PromptTemplate(payload) => {
                self.0.exec_prompt_template(payload).await
            }
        }
    }

    async fn cleanup(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

impl HandlerInner {
    async fn exec_prompt_impl_anthropic(
        &self,
        prompt: &str,
        model: &str,
        response_format: llm_iface::OutputFormat,
        provider: &config::BackendConfig,
    ) -> ModuleResult<String> {
        let mut request = serde_json::json!({
            "model": model,
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
        model: &str,
        response_format: llm_iface::OutputFormat,
        provider: &config::BackendConfig,
    ) -> ModuleResult<String> {
        if true {
            return Err(OverloadedError.into());
        }
        let mut request = serde_json::json!({
            "model": model,
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
        model: &str,
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
                    provider.host, model, provider.key
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
        model: &str,
        response_format: llm_iface::OutputFormat,
        provider: &config::BackendConfig,
    ) -> ModuleResult<String> {
        let mut request = serde_json::json!({
            "model": model,
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
        model: &str,
        response_format: llm_iface::OutputFormat,
        provider: &config::BackendConfig,
    ) -> ModuleResult<String> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "llm_genvm_module_call",
            "params": [model, prompt, serde_json::to_string(&response_format).unwrap()],
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

    pub async fn exec_prompt_in_provider(
        &self,
        prompt: &str,
        model: &str,
        response_format: llm_iface::OutputFormat,
        provider_id: &str,
    ) -> ModuleResult<llm_iface::PromptAnswer> {
        log::trace!(prompt = prompt, model = model, provider_id = provider_id, format:? = response_format; "executing prompt");

        let provider: &config::BackendConfig = self
            .config
            .backends
            .get(provider_id)
            .ok_or(mlua::Error::DeserializeError("wrong provider".into()))?;

        let res_not_sanitized = match provider.provider {
            config::Provider::Ollama => {
                self.exec_prompt_impl_ollama(prompt, model, response_format, provider)
                    .await
            }
            config::Provider::OpenaiCompatible => {
                self.exec_prompt_impl_openai(prompt, model, response_format, provider)
                    .await
            }
            config::Provider::Simulator => {
                self.exec_prompt_impl_simulator(prompt, model, response_format, provider)
                    .await
            }
            config::Provider::Anthropic => {
                self.exec_prompt_impl_anthropic(prompt, model, response_format, provider)
                    .await
            }
            config::Provider::Google => {
                self.exec_prompt_impl_gemini(prompt, model, response_format, provider)
                    .await
            }
        };

        let res_not_sanitized = match res_not_sanitized {
            Ok(Ok(res_not_sanitized)) => res_not_sanitized,
            Err(e) => {
                log::warn!(
                    prompt = prompt,
                    model = model,
                    provider_id = provider_id,
                    format:? = response_format,
                    error = genvm_common::log_error(&e);
                    "executing prompt fatal failure"
                );
                return Err(e);
            }
            Ok(Err(e)) => {
                log::debug!(
                    prompt = prompt,
                    model = model,
                    provider_id = provider_id,
                    format:? = response_format,
                    error:serde = e;
                    "executing prompt failure"
                );
                return Ok(Err(e));
            }
        };

        match response_format {
            llm_iface::OutputFormat::Text => {
                Ok(Ok(llm_iface::PromptAnswer::Text(res_not_sanitized)))
            }
            llm_iface::OutputFormat::JSON => {
                let sanitized = sanitize_json_str(&res_not_sanitized);
                let obj = serde_json::from_str(sanitized)?;
                Ok(Ok(llm_iface::PromptAnswer::Object(obj)))
            }
        }
    }
}

impl Handler {
    async fn exec_prompt(
        &self,
        zelf: Arc<Handler>,
        payload: llm_iface::PromptPayload,
    ) -> ModuleResult<llm_iface::PromptAnswer> {
        log::debug!(payload:serde = payload; "exec_prompt start");

        let llm_iface::PromptPart::Text(prompt) = &payload.parts[0];
        let res = self.user_vm.greybox(zelf, prompt).await?;
        log::debug!(result:serde = res; "script returned");

        Ok(res)
    }

    async fn exec_prompt_template(
        &self,
        payload: llm_iface::PromptTemplatePayload,
    ) -> ModuleResult<llm_iface::PromptAnswer> {
        let (provider_id, provider) = self.inner.config.backends.first_key_value().unwrap();

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
                    &self.inner.config.prompt_templates.eq_non_comparative_leader,
                    &genvm_common::templater::HASH_UNFOLDER_RE,
                )?;

                self.inner
                    .exec_prompt_in_provider(
                        &new_prompt,
                        &provider.script_config.models[0],
                        llm_iface::OutputFormat::Text,
                        provider_id,
                    )
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
                    &self.inner.config.prompt_templates.eq_comparative,
                    &genvm_common::templater::HASH_UNFOLDER_RE,
                )?;

                let res = self
                    .inner
                    .exec_prompt_in_provider(
                        &new_prompt,
                        &provider.script_config.models[0],
                        llm_iface::OutputFormat::JSON,
                        provider_id,
                    )
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
                    &self
                        .inner
                        .config
                        .prompt_templates
                        .eq_non_comparative_validator,
                    &genvm_common::templater::HASH_UNFOLDER_RE,
                )?;

                log::error!(old = self.inner.config.prompt_templates.eq_non_comparative_validator, new = new_prompt, vars:serde = vars; "DEBUG");

                let res = self
                    .inner
                    .exec_prompt_in_provider(
                        &new_prompt,
                        &provider.script_config.models[0],
                        llm_iface::OutputFormat::JSON,
                        provider_id,
                    )
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
    use std::{collections::HashMap, sync::Arc};

    use std::sync::Once;

    static INIT: Once = Once::new();

    /// Setup function that is only run once, even if called multiple times.
    fn setup() {
        INIT.call_once(|| {
            let base_conf = genvm_common::BaseConfig {
                blocking_threads: 0,
                log_disable: Default::default(),
                log_level: log::LevelFilter::Trace,
                threads: 0,
            };
            base_conf.setup_logging(std::io::stdout()).unwrap();
        });
    }

    use genvm_common::templater;
    use genvm_modules_interfaces::llm::PromptAnswer;

    use crate::{
        config,
        handler::{HandlerInner, OverloadedError},
    };

    mod conf {
        pub const openai: &str = r#"{
            "host": "https://api.openai.com",
            "provider": "openai-compatible",
            "models": ["gpt-4o-mini"],
            "key": "${ENV[OPENAIKEY]}"
        }"#;

        pub const heurist: &str = r#"{
            "host": "https://llm-gateway.heurist.xyz",
            "provider": "openai-compatible",
            "models": ["meta-llama/llama-3.3-70b-instruct"],
            "key": "${ENV[HEURISTKEY]}"
        }"#;

        pub const anthropic: &str = r#"{
            "host": "https://api.anthropic.com",
            "provider": "anthropic",
            "models": ["claude-3-5-sonnet-20241022"],
            "key": "${ENV[ANTHROPICKEY]}"
        }"#;

        pub const xai: &str = r#"{
            "host": "https://api.x.ai",
            "provider": "openai-compatible",
            "models": ["grok-2-1212"],
            "key": "${ENV[XAIKEY]}"
        }"#;

        pub const google: &str = r#"{
            "host": "https://generativelanguage.googleapis.com",
            "provider": "google",
            "models": ["gemini-1.5-flash"],
            "key": "${ENV[GEMINIKEY]}"
        }"#;

        pub const atoma: &str = r#"{
            "host": "https://api.atoma.network",
            "provider": "openai-compatible",
            "models": ["meta-llama/Llama-3.3-70B-Instruct"],
            "key": "${ENV[ATOMAKEY]}"
        }"#;
    }

    async fn do_test_text(conf: &str) {
        setup();

        let base_conf = genvm_common::BaseConfig {
            blocking_threads: 0,
            log_disable: Default::default(),
            log_level: log::LevelFilter::Trace,
            threads: 0,
        };

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
        let backend_name = "test".to_owned();

        let imp = HandlerInner {
            config: Arc::new(config::Config {
                bind_address: Default::default(),
                backends: [(backend_name.clone(), backend.clone())]
                    .into_iter()
                    .collect(),
                prompt_templates: config::PromptTemplates {
                    eq_comparative: Default::default(),
                    eq_non_comparative_leader: Default::default(),
                    eq_non_comparative_validator: Default::default(),
                },
                base: base_conf,
                lua_script_path: Default::default(),
            }),
            client: reqwest::Client::new(),
        };

        let res = imp
            .exec_prompt_in_provider(
                "Respond with a single word \"yes\" (without quotes) and only this word, lowercase",
                &backend.script_config.models[0],
                genvm_modules_interfaces::llm::OutputFormat::Text,
                &backend_name,
            )
            .await;

        let res = match res {
            Ok(res) => res,
            Err(e) if e.is::<OverloadedError>() => {
                println!("WARNING: test skipped");
                return;
            }
            Err(e) => {
                panic!("err {:?}", e);
            }
        };

        let mut res = res.unwrap();

        res.map_text(|s| *s = s.to_lowercase().trim().to_owned());

        assert_eq!(res, PromptAnswer::Text("yes".into()));
    }

    async fn do_test_json(conf: &str) {
        setup();

        let base_conf = genvm_common::BaseConfig {
            blocking_threads: 0,
            log_disable: Default::default(),
            log_level: log::LevelFilter::Trace,
            threads: 0,
        };

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
        let backend_name = "test".to_owned();

        let imp = HandlerInner {
            config: Arc::new(config::Config {
                bind_address: Default::default(),
                backends: [(backend_name.clone(), backend.clone())]
                    .into_iter()
                    .collect(),
                prompt_templates: config::PromptTemplates {
                    eq_comparative: Default::default(),
                    eq_non_comparative_leader: Default::default(),
                    eq_non_comparative_validator: Default::default(),
                },
                base: base_conf,
                lua_script_path: Default::default(),
            }),
            client: reqwest::Client::new(),
        };

        const PROMPT: &str = "respond with json object containing single key \"result\" and associated value being a random integer from 0 to 100 (inclusive), it must be number, not wrapped in quotes";
        let res = imp
            .exec_prompt_in_provider(
                PROMPT,
                &backend.script_config.models[0],
                genvm_modules_interfaces::llm::OutputFormat::JSON,
                &backend_name,
            )
            .await;

        let res = match res {
            Ok(res) => res,
            Err(e) if e.is::<OverloadedError>() => {
                println!("WARNING: test skipped");
                return;
            }
            Err(e) => {
                panic!("err {:?}", e);
            }
        };

        let res = match res {
            Ok(PromptAnswer::Object(o)) => o,
            res => panic!("invalid! {:?}", res),
        };

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
