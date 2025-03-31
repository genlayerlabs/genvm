use anyhow::Context;
use serde_derive::{Deserialize, Serialize};

use genvm_modules_impl_common::*;
use genvm_modules_interfaces::*;
use std::sync::Arc;

mod string_templater;
mod template_ids;

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum LLLMProvider {
    Ollama,
    OpenaiCompatible,
    Simulator,
    Anthropic,
    Google,
}

struct Impl {
    config: Config,
    api_key: String,

    sessions: SessionPool<()>,
    cancellation: Arc<CancellationToken>,
}

impl Drop for Impl {
    fn drop(&mut self) {}
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
        log::error!(response = CENSOR_RESPONSE.replace_all(&debug, "\"<censored>\": \"<censored>\""), body:? = body, retry = i; "llm request failed");

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }

    Err(anyhow::anyhow!("llm retries exceeded"))
}

#[derive(Deserialize)]
struct Prompts {
    eq_comparative: String,
    eq_non_comparative_leader: String,
    eq_non_comparative_validator: String,
}

#[derive(Deserialize)]
struct Config {
    host: String,
    provider: LLLMProvider,
    model: String,
    key_env_name: String,
    prompt_templates: Prompts,
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

#[derive(Clone, Deserialize, Serialize, Copy)]
#[serde(rename_all = "kebab-case")]
enum ExecPromptConfigMode {
    Text,
    Json,
}
#[derive(Deserialize)]
struct ExecPromptConfig {
    response_format: Option<ExecPromptConfigMode>,
}

impl Impl {
    fn try_new(args: CtorArgs) -> anyhow::Result<Self> {
        let config: Config = serde_yaml::from_value(args.config)?;
        let api_key = std::env::var(&config.key_env_name).unwrap_or("".into());
        Ok(Impl {
            config,
            api_key,
            sessions: SessionPool::new(),
            cancellation: args.cancellation,
        })
    }

    async fn consume_gas(&self, _amount: u64) -> anyhow::Result<()> {
        Ok(())
    }

    async fn exec_prompt_impl_anthropic(
        &self,
        prompt: &str,
        response_format: ExecPromptConfigMode,
        session: &mut Session<()>,
    ) -> ModuleResult<String> {
        let mut request = serde_json::json!({
            "model": &self.config.model,
            "messages": [{
                "role": "user",
                "content": prompt,
            }],
            "max_tokens": 1000,
            "stream": false,
            "temperature": 0.7,
        });
        match response_format {
            ExecPromptConfigMode::Text => {}
            ExecPromptConfigMode::Json => {
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
            session
                .client
                .post(format!("{}/v1/messages", self.config.host))
                .header("Content-Type", "application/json")
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .body(request.clone())
        })
        .await?;

        let res = genvm_modules_impl_common::read_response(res).await?;
        let val: serde_json::Value = serde_json::from_str(&res)?;
        match response_format {
            ExecPromptConfigMode::Text => val
                .pointer("/content/0/text")
                .and_then(|x| x.as_str())
                .ok_or(anyhow::anyhow!("can't get response field {}", &res))
                .map(String::from)
                .map_err(Into::into),
            ExecPromptConfigMode::Json => val
                .pointer("/content/0/input/type")
                .ok_or(anyhow::anyhow!("can't get response field {}", &res))
                .and_then(|x| serde_json::to_string(x).map_err(Into::into))
                .map_err(Into::into),
        }
    }

    async fn exec_prompt_impl_openai(
        &self,
        prompt: &str,
        response_format: ExecPromptConfigMode,
        session: &mut Session<()>,
    ) -> ModuleResult<String> {
        let mut request = serde_json::json!({
            "model": &self.config.model,
            "messages": [{
                "role": "user",
                "content": prompt,
            }],
            "max_tokens": 1000,
            "stream": false,
            "temperature": 0.7,
        });
        match response_format {
            ExecPromptConfigMode::Text => {}
            ExecPromptConfigMode::Json => {
                request.as_object_mut().unwrap().insert(
                    "response_format".into(),
                    serde_json::json!({"type": "json_object"}),
                );
            }
        }
        let request = serde_json::to_vec(&request)?;
        let res = send_with_retries(|| {
            session
                .client
                .post(format!("{}/v1/chat/completions", self.config.host))
                .header("Content-Type", "application/json")
                .header("Authorization", &format!("Bearer {}", &self.api_key))
                .body(request.clone())
        })
        .await?;
        let res = genvm_modules_impl_common::read_response(res).await?;
        let val: serde_json::Value = serde_json::from_str(&res)?;
        let response = val
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("can't get response field {}", &res))?;

        let total_tokens = val
            .pointer("/usage/total_tokens")
            .and_then(|v| v.as_u64())
            .ok_or(anyhow::anyhow!("can't get eval_duration field {}", &res))?;
        self.consume_gas(total_tokens << 8).await?;

        Ok(response.into())
    }

    async fn exec_prompt_impl_gemini(
        &self,
        prompt: &str,
        response_format: ExecPromptConfigMode,
        session: &mut Session<()>,
    ) -> ModuleResult<String> {
        let request = serde_json::json!({
            "contents": [{
                "parts": [
                    {"text": prompt},
                ]
            }],
            "generationConfig": {
                "responseMimeType": match response_format {
                    ExecPromptConfigMode::Text => "text/plain",
                    ExecPromptConfigMode::Json => "application/json",
                },
                "temperature": 0.7,
                "maxOutputTokens": 800,
            }
        });

        let request = serde_json::to_vec(&request)?;
        let res = send_with_retries(|| {
            session
                .client
                .post(format!(
                    "{}/v1beta/models/{}:generateContent?key={}",
                    self.config.host, self.config.model, self.api_key
                ))
                .header("Content-Type", "application/json")
                .body(request.clone())
        })
        .await?;

        let res = read_response(res).await?;

        let res: serde_json::Value = serde_json::from_str(&res)?;

        let res = res
            .pointer("/candidates/0/content/parts/0/text")
            .and_then(|x| x.as_str())
            .ok_or(anyhow::anyhow!("can't get response field {}", &res))?;
        Ok(res.into())
    }

    async fn exec_prompt_impl_ollama(
        &self,
        prompt: &str,
        response_format: ExecPromptConfigMode,
        session: &mut Session<()>,
    ) -> ModuleResult<String> {
        let mut request = serde_json::json!({
            "model": &self.config.model,
            "prompt": prompt,
            "stream": false,
        });
        match response_format {
            ExecPromptConfigMode::Text => {}
            ExecPromptConfigMode::Json => {
                request
                    .as_object_mut()
                    .unwrap()
                    .insert("format".into(), "json".into());
            }
        }

        let request = serde_json::to_vec(&request)?;
        let res = send_with_retries(|| {
            session
                .client
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
            .ok_or(anyhow::anyhow!("can't get response field {}", &res))?;
        Ok(response.into())
    }

    async fn exec_prompt_impl_simulator(
        &self,
        prompt: &str,
        response_format: ExecPromptConfigMode,
        session: &mut Session<()>,
    ) -> ModuleResult<String> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "llm_genvm_module_call",
            "params": [&self.config.model, prompt, serde_json::to_string(&response_format).unwrap()],
            "id": 1,
        });
        let request = serde_json::to_vec(&request)?;
        // no retries for the simulator
        let res = session
            .client
            .post(format!("{}/api", &self.config.host))
            .header("Content-Type", "application/json")
            .body(request)
            .send()
            .await?;
        let res = genvm_modules_impl_common::read_response(res).await?;
        let res: serde_json::Value = serde_json::from_str(&res)?;
        res.pointer("/result/response")
            .and_then(|v| v.as_str())
            .map(String::from)
            .ok_or(ModuleError::Fatal(anyhow::anyhow!(
                "can't get response field {}",
                &res
            )))
    }

    async fn exec_prompt_impl(&self, config: &str, prompt: &str) -> ModuleResult<String> {
        let config: ExecPromptConfig =
            make_error_recoverable(serde_json::from_str(config), "invalid configuration")?;
        let response_format = config.response_format.unwrap_or(ExecPromptConfigMode::Text);

        let mut session = match self.sessions.get() {
            Some(session) => session,
            None => Box::new(Session {
                client: reqwest::Client::new(),
                data: (),
            }),
        };
        let res_not_sanitized = match self.config.provider {
            LLLMProvider::Ollama => {
                self.exec_prompt_impl_ollama(prompt, response_format, &mut session)
                    .await
            }
            LLLMProvider::OpenaiCompatible => {
                self.exec_prompt_impl_openai(prompt, response_format, &mut session)
                    .await
            }
            LLLMProvider::Simulator => {
                self.exec_prompt_impl_simulator(prompt, response_format, &mut session)
                    .await
            }
            LLLMProvider::Anthropic => {
                self.exec_prompt_impl_anthropic(prompt, response_format, &mut session)
                    .await
            }
            LLLMProvider::Google => {
                self.exec_prompt_impl_gemini(prompt, response_format, &mut session)
                    .await
            }
        };
        self.sessions.retn(session);

        let res_not_sanitized = res_not_sanitized?;

        match response_format {
            ExecPromptConfigMode::Text => Ok(res_not_sanitized),
            ExecPromptConfigMode::Json => Ok(sanitize_json_str(&res_not_sanitized).into()),
        }
    }
}

const JSON_PROMPT_CONFIG: &str = r#"{"response_format": "json"}"#;

impl Impl {
    async fn exec_prompt(&self, config: &str, prompt: &str) -> ModuleResult<String> {
        log::debug!(event = "exec_prompt", prompt = prompt, config = config, model = self.config.model; "start");
        let res = self.exec_prompt_impl(config, prompt).await;
        log::info!(event = "exec_prompt", prompt = prompt, config = config, model = self.config.model, result:? = res; "finished");
        res
    }

    async fn eq_principle_prompt(&self, template_id: u8, vars: &str) -> ModuleResult<bool> {
        use template_ids::TemplateId;
        let id = make_error_recoverable(
            TemplateId::try_from(template_id)
                .map_err(|_e| anyhow::anyhow!("unknown template id {template_id}")),
            "invalid prompt id",
        )?;
        let template = match id {
            TemplateId::Comparative => &self.config.prompt_templates.eq_comparative,
            TemplateId::NonComparativeValidator => {
                &self.config.prompt_templates.eq_non_comparative_validator
            }
            TemplateId::NonComparativeLeader => {
                return Err(ModuleError::Recoverable("invalid prompt id"))
            }
        };
        let vars: std::collections::BTreeMap<String, String> =
            make_error_recoverable(serde_json::from_str(vars), "invalid variables")?;
        let new_prompt = string_templater::patch_str(&vars, template)?;
        let res = self.exec_prompt(JSON_PROMPT_CONFIG, &new_prompt).await?;
        answer_is_bool(res)
    }

    async fn exec_prompt_id(&self, template_id: u8, vars: &str) -> ModuleResult<String> {
        use template_ids::TemplateId;
        let id = make_error_recoverable(
            TemplateId::try_from(template_id)
                .map_err(|_e| anyhow::anyhow!("unknown template id {template_id}")),
            "invalid prompt id",
        )?;
        let template = match id {
            TemplateId::Comparative | TemplateId::NonComparativeValidator => {
                return Err(ModuleError::Recoverable("illegal prompt id"))
            }
            TemplateId::NonComparativeLeader => {
                &self.config.prompt_templates.eq_non_comparative_leader
            }
        };
        let vars: std::collections::BTreeMap<String, String> =
            make_error_recoverable(serde_json::from_str(vars), "invalid vars")?;
        let new_prompt = string_templater::patch_str(&vars, template)?;
        let res = self.exec_prompt("{}", &new_prompt).await?;
        Ok(res)
    }
}

fn answer_is_bool(res: String) -> ModuleResult<bool> {
    let val: serde_json::Value = serde_json::from_str(&res)?;
    val.pointer("/result")
        .and_then(|x| x.as_bool())
        .ok_or(ModuleError::Fatal(anyhow::anyhow!("invalid json")))
}

struct Proxy(Arc<Impl>);

#[async_trait::async_trait]
impl genvm_modules_interfaces::Llm for Proxy {
    fn exec_prompt(
        &self,
        config: String,
        prompt: String,
    ) -> tokio::task::JoinHandle<anyhow::Result<Box<[u8]>>> {
        async fn forward(zelf: Arc<Impl>, config: String, prompt: String) -> ModuleResult<String> {
            tokio::select! {
                res = zelf.exec_prompt(&config, &prompt) => {
                    res
                }
                _ = zelf.cancellation.chan.closed() => {
                    Err(ModuleError::Fatal(anyhow::anyhow!("timeout")))
                }
            }
        }

        tokio::spawn(genvm_modules_interfaces::module_result_to_future(forward(
            self.0.clone(),
            config,
            prompt,
        )))
    }
    fn exec_prompt_id(
        &self,
        id: u8,
        vars: String,
    ) -> tokio::task::JoinHandle<anyhow::Result<Box<[u8]>>> {
        async fn forward(zelf: Arc<Impl>, id: u8, vars: String) -> ModuleResult<String> {
            zelf.exec_prompt_id(id, &vars).await
        }

        tokio::spawn(genvm_modules_interfaces::module_result_to_future(forward(
            self.0.clone(),
            id,
            vars,
        )))
    }

    fn eq_principle_prompt<'a>(
        &'a self,
        id: u8,
        vars: &'a str,
    ) -> core::pin::Pin<Box<dyn ::core::future::Future<Output = ModuleResult<bool>> + Send + 'a>>
    {
        Box::pin(self.0.eq_principle_prompt(id, vars))
    }
}

#[no_mangle]
pub fn new_llm_module(
    args: CtorArgs,
) -> anyhow::Result<Box<dyn genvm_modules_interfaces::Llm + Send + Sync>> {
    Ok(Box::new(Proxy(Arc::new(Impl::try_new(args)?))))
}

#[cfg(test)]
#[allow(non_upper_case_globals, dead_code)]
mod tests {
    use crate::Impl;

    mod conf {
        pub const openai: &str = r#"{
            "host": "https://api.openai.com",
            "provider": "openai-compatible",
            "model": "gpt-4o-mini",
            "key_env_name": "OPENAIKEY",
            "prompt_templates": {
                "eq_comparative": "",
                "eq_non_comparative_leader": "",
                "eq_non_comparative_validator": ""
            }
        }"#;

        pub const heurist: &str = r#"{
            "host": "https://llm-gateway.heurist.xyz",
            "provider": "openai-compatible",
            "model": "meta-llama/llama-3.3-70b-instruct",
            "key_env_name": "HEURISTKEY",
            "prompt_templates": {
                "eq_comparative": "",
                "eq_non_comparative_leader": "",
                "eq_non_comparative_validator": ""
            }
        }"#;

        pub const anthropic: &str = r#"{
            "host": "https://api.anthropic.com",
            "provider": "anthropic",
            "model": "claude-3-5-sonnet-20241022",
            "key_env_name": "ANTHROPICKEY",
            "prompt_templates": {
                "eq_comparative": "",
                "eq_non_comparative_leader": "",
                "eq_non_comparative_validator": ""
            }
        }"#;

        pub const xai: &str = r#"{
            "host": "https://api.x.ai",
            "provider": "openai-compatible",
            "model": "grok-2-1212",
            "key_env_name": "XAIKEY",
            "prompt_templates": {
                "eq_comparative": "",
                "eq_non_comparative_leader": "",
                "eq_non_comparative_validator": ""
            }
        }"#;

        pub const google: &str = r#"{
            "host": "https://generativelanguage.googleapis.com",
            "provider": "google",
            "model": "gemini-1.5-flash",
            "key_env_name": "GEMINIKEY",
            "prompt_templates": {
                "eq_comparative": "",
                "eq_non_comparative_leader": "",
                "eq_non_comparative_validator": ""
            }
        }"#;

        pub const atoma: &str = r#"{
            "host": "https://api.atoma.network",
            "provider": "openai-compatible",
            "model": "meta-llama/Llama-3.3-70B-Instruct",
            "key_env_name": "ATOMAKEY",
            "prompt_templates": {
                "eq_comparative": "",
                "eq_non_comparative_leader": "",
                "eq_non_comparative_validator": ""
            }
        }"#;
    }

    async fn do_test_text(conf: &str) {
        use genvm_modules_interfaces::*;

        let (cancellation, canceller) = make_cancellation();

        let conf: serde_yaml::Value = serde_json::from_str(conf).unwrap();

        let imp = Impl::try_new(CtorArgs {
            config: conf,
            cancellation,
        })
        .unwrap();

        let res = imp
            .exec_prompt(
                "{}",
                "Respond with \"yes\" (without quotes) and only this word",
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
        use genvm_modules_interfaces::*;

        let (cancellation, canceller) = make_cancellation();

        let conf: serde_yaml::Value = serde_json::from_str(conf).unwrap();

        let imp = Impl::try_new(CtorArgs {
            config: conf,
            cancellation,
        })
        .unwrap();

        const PROMPT: &str = "respond with json object containing single key \"result\" and associated value being a random integer from 0 to 100 (inclusive), it must be number, not wrapped in quotes";
        let res = imp
            .exec_prompt("{\"response_format\": \"json\"}", PROMPT)
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
                    crate::tests::do_test_text(crate::tests::conf::$conf).await
                }
                #[tokio::test]
                async fn json() {
                    crate::tests::do_test_json(crate::tests::conf::$conf).await
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
