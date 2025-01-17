use anyhow::Result;
use genvm_modules_common::*;
use serde_derive::{Deserialize, Serialize};

use std::ffi::CStr;

use crate::interfaces::RecoverableError;
use genvm_modules_common::interfaces::web_functions_api;

mod string_templater;
mod template_ids;

genvm_modules_common::default_base_functions!(web_functions_api, Impl);

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum LLLMProvider {
    Ollama,
    Openai,
    Simulator,
    Heurist,
    Anthropic,
    Xai,
    Google,
}

struct Impl {
    config: Config,
    api_key: String,
    log_fd: std::os::fd::RawFd,
}

impl Drop for Impl {
    fn drop(&mut self) {}
}

fn default_equivalence_prompt_comparative() -> String {
    include_str!("prompts/equivalence_prompt_comparative.txt").into()
}

fn default_equivalence_prompt_non_comparative() -> String {
    include_str!("prompts/equivalence_prompt_non_comparative.txt").into()
}

fn default_equivalence_prompt_non_comparative_leader() -> String {
    include_str!("prompts/equivalence_prompt_non_comparative_leader.txt").into()
}

#[derive(Deserialize)]
struct Config {
    host: String,
    provider: LLLMProvider,
    model: String,
    #[serde(default = "String::new")]
    key_env_name: String,
    #[serde(default = "default_equivalence_prompt_comparative")]
    equivalence_prompt_comparative: String,
    #[serde(default = "default_equivalence_prompt_non_comparative")]
    equivalence_prompt_non_comparative: String,
    #[serde(default = "default_equivalence_prompt_non_comparative_leader")]
    equivalence_prompt_non_comparative_leader: String,
}

fn sanitize_json_str<'a>(s: &'a str) -> &'a str {
    let s = s.trim();
    let s = s
        .strip_prefix("```json")
        .or(s.strip_prefix("```"))
        .unwrap_or(s);
    let s = s.strip_suffix("```").unwrap_or(s);
    s.trim().into()
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
    fn try_new(args: &CtorArgs) -> Result<Self> {
        let conf: &str = args.config()?;
        let config: Config = serde_json::from_str(conf)?;
        let api_key = std::env::var(&config.key_env_name).unwrap_or("".into());
        Ok(Impl {
            config,
            log_fd: args.log_fd,
            api_key,
        })
    }

    fn exec_prompt_impl_anthropic(
        &mut self,
        prompt: &str,
        response_format: ExecPromptConfigMode,
        gas: &mut u64,
    ) -> Result<String> {
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
        let mut res = isahc::send(
            isahc::Request::post(&format!("{}/v1/messages", self.config.host))
                .header("Content-Type", "application/json")
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .body(serde_json::to_string(&request)?.as_bytes())?,
        )?;
        let res = genvm_modules_impl_common::read_response(&mut res)?;
        let val: serde_json::Value = serde_json::from_str(&res)?;
        match response_format {
            ExecPromptConfigMode::Text => val
                .pointer("/content/0/text")
                .and_then(|x| x.as_str())
                .ok_or(anyhow::anyhow!("can't get response field {}", &res))
                .map(String::from),
            ExecPromptConfigMode::Json => val
                .pointer("/content/0/input/type")
                .ok_or(anyhow::anyhow!("can't get response field {}", &res))
                .and_then(|x| serde_json::to_string(x).map_err(Into::into)),
        }
    }

    fn exec_prompt_impl_openai(
        &mut self,
        prompt: &str,
        response_format: ExecPromptConfigMode,
        gas: &mut u64,
    ) -> Result<String> {
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
        let mut res = isahc::send(
            isahc::Request::post(&format!("{}/v1/chat/completions", self.config.host))
                .header("Content-Type", "application/json")
                .header("Authorization", &format!("Bearer {}", &self.api_key))
                .body(serde_json::to_string(&request)?.as_bytes())?,
        )?;
        let res = genvm_modules_impl_common::read_response(&mut res)?;
        let val: serde_json::Value = serde_json::from_str(&res)?;
        let response = val
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("can't get response field {}", &res))?;
        let total_tokens = val
            .pointer("/usage/total_tokens")
            .and_then(|v| v.as_u64())
            .ok_or(anyhow::anyhow!("can't get eval_duration field {}", &res))?;
        *gas -= (total_tokens << 8).min(*gas);

        Ok(response.into())
    }

    fn exec_prompt_impl_gemini(
        &mut self,
        prompt: &str,
        response_format: ExecPromptConfigMode,
        gas: &mut u64,
    ) -> Result<String> {
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

        let mut res = isahc::send(
            isahc::Request::post(format!(
                "{}/v1beta/models/{}:generateContent?key={}",
                self.config.host, self.config.model, self.api_key
            ))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&request)?.as_bytes())?,
        )?;
        let res = genvm_modules_impl_common::read_response(&mut res)?;

        let res: serde_json::Value = serde_json::from_str(&res)?;

        let res = res
            .pointer("/candidates/0/content/parts/0/text")
            .and_then(|x| x.as_str())
            .ok_or(anyhow::anyhow!("can't get response field {}", &res))?;
        Ok(res.into())
    }

    fn exec_prompt_impl_ollama(
        &mut self,
        prompt: &str,
        response_format: ExecPromptConfigMode,
        gas: &mut u64,
    ) -> Result<String> {
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
        let mut res = isahc::send(
            isahc::Request::post(&format!("{}/api/generate", self.config.host))
                .body(serde_json::to_string(&request)?.as_bytes())?,
        )?;
        let res = genvm_modules_impl_common::read_response(&mut res)?;
        let val: serde_json::Value = serde_json::from_str(&res)?;
        let response = val
            .as_object()
            .and_then(|v| v.get("response"))
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("can't get response field {}", &res))?;
        let eval_duration = val
            .as_object()
            .and_then(|v| v.get("eval_duration"))
            .and_then(|v| v.as_u64())
            .ok_or(anyhow::anyhow!("can't get eval_duration field {}", &res))?;
        *gas -= (eval_duration << 4).min(*gas);
        Ok(response.into())
    }

    fn exec_prompt_impl_simulator(
        &mut self,
        prompt: &str,
        response_format: ExecPromptConfigMode,
        gas: &mut u64,
    ) -> Result<String> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "llm_genvm_module_call",
            "params": [&self.config.model, prompt, serde_json::to_string(&response_format).unwrap()],
            "id": 1,
        });
        let mut res = isahc::send(
            isahc::Request::post(format!("{}/api", &self.config.host))
                .header("Content-Type", "application/json")
                .body(serde_json::to_string(&request)?.as_bytes())?,
        )?;
        let res = genvm_modules_impl_common::read_response(&mut res)?;
        let res: serde_json::Value = serde_json::from_str(&res)?;
        res.as_object()
            .and_then(|v| v.get("result"))
            .and_then(|v| v.as_object())
            .and_then(|v| v.get("response"))
            .and_then(|v| v.as_str())
            .map(String::from)
            .ok_or(anyhow::anyhow!("can't get response field {}", &res))
    }

    fn exec_prompt_impl(&mut self, gas: &mut u64, config: &str, prompt: &str) -> Result<String> {
        let config: ExecPromptConfig =
            serde_json::from_str(config).map_err(RecoverableError::from_anyhow)?;
        let response_format = config
            .response_format
            .clone()
            .unwrap_or(ExecPromptConfigMode::Text);

        let res_not_sanitized = match self.config.provider {
            LLLMProvider::Ollama => self.exec_prompt_impl_ollama(prompt, response_format, gas),
            LLLMProvider::Openai | LLLMProvider::Heurist | LLLMProvider::Xai => {
                self.exec_prompt_impl_openai(prompt, response_format, gas)
            }
            LLLMProvider::Simulator => {
                self.exec_prompt_impl_simulator(prompt, response_format, gas)
            }
            LLLMProvider::Anthropic => {
                self.exec_prompt_impl_anthropic(prompt, response_format, gas)
            }
            LLLMProvider::Google => self.exec_prompt_impl_gemini(prompt, response_format, gas),
        }?;

        match response_format {
            ExecPromptConfigMode::Text => Ok(res_not_sanitized),
            ExecPromptConfigMode::Json => Ok(sanitize_json_str(&res_not_sanitized).into()),
        }
    }

    fn exec_prompt(&mut self, gas: &mut u64, config: &str, prompt: &str) -> Result<String> {
        let res = self.exec_prompt_impl(gas, config, prompt);
        match serde_json::to_string(&serde_json::json!({
            "event": "exec_prompt",
            "prompt": prompt,
            "config": config,
            "model": self.config.model,
            "result": format!("{res:?}"),
        })) {
            Ok(log_data) => write_to_fd(self.log_fd, &log_data),
            Err(_) => {}
        }
        res
    }

    fn invalid_prompt_id_err() -> anyhow::Error {
        RecoverableError(anyhow::anyhow!("invalid prompt id")).into()
    }

    fn eq_principle_prompt(&mut self, gas: &mut u64, template_id: u8, vars: &str) -> Result<bool> {
        use template_ids::TemplateId;
        let id = TemplateId::try_from(template_id).map_err(|_e| Self::invalid_prompt_id_err())?;
        let template = match id {
            TemplateId::Comparative => &self.config.equivalence_prompt_comparative,
            TemplateId::NonComparative => &self.config.equivalence_prompt_non_comparative,
            TemplateId::NonComparativeLeader => return Err(Self::invalid_prompt_id_err()),
        };
        let vars: std::collections::BTreeMap<String, String> =
            serde_json::from_str(vars).map_err(RecoverableError::from_anyhow)?;
        let new_prompt = string_templater::patch_str(&vars, &template)?;
        let res = self.exec_prompt(gas, "{}".into(), &new_prompt)?;
        answer_is_bool(res)
    }

    fn exec_prompt_id(&mut self, gas: &mut u64, template_id: u8, vars: &str) -> Result<String> {
        use template_ids::TemplateId;
        let id = TemplateId::try_from(template_id).map_err(|_e| Self::invalid_prompt_id_err())?;
        let template = match id {
            TemplateId::Comparative => return Err(Self::invalid_prompt_id_err()),
            TemplateId::NonComparative => return Err(Self::invalid_prompt_id_err()),
            TemplateId::NonComparativeLeader => {
                &self.config.equivalence_prompt_non_comparative_leader
            }
        };
        let vars: std::collections::BTreeMap<String, String> =
            serde_json::from_str(vars).map_err(RecoverableError::from_anyhow)?;
        let new_prompt = string_templater::patch_str(&vars, &template)?;
        let res = self.exec_prompt(gas, "{}".into(), &new_prompt)?;
        Ok(res)
    }
}

fn answer_is_bool(mut res: String) -> Result<bool> {
    res.make_ascii_lowercase();
    let has_true = res.contains("true");
    let has_false = res.contains("false");
    if has_true == has_false {
        anyhow::bail!("contains both true and false");
    }
    Ok(has_true)
}

#[no_mangle]
pub extern "C-unwind" fn exec_prompt(
    ctx: *const (),
    gas: &mut u64,
    config: *const u8,
    prompt: *const u8,
) -> interfaces::BytesResult {
    let ctx = get_ptr(ctx);
    let config = unsafe { CStr::from_ptr(config as *const std::ffi::c_char) };
    let prompt = unsafe { CStr::from_ptr(prompt as *const std::ffi::c_char) };
    let res = config
        .to_str()
        .map_err(|e| anyhow::Error::from(e))
        .and_then(|config| {
            prompt
                .to_str()
                .map_err(|e| anyhow::Error::from(e))
                .and_then(|prompt| ctx.exec_prompt(gas, config, prompt))
        });
    interfaces::serialize_result(res)
}

#[no_mangle]
pub extern "C-unwind" fn eq_principle_prompt(
    ctx: *const (),
    gas: &mut u64,
    template_id: u8,
    vars: *const u8,
) -> interfaces::BytesResult {
    let ctx = get_ptr(ctx);
    let vars = unsafe { CStr::from_ptr(vars as *const std::ffi::c_char) };
    let res = vars
        .to_str()
        .map_err(|e| anyhow::Error::from(e))
        .and_then(|vars| ctx.eq_principle_prompt(gas, template_id, vars));
    interfaces::serialize_result(res)
}

#[no_mangle]
pub extern "C-unwind" fn exec_prompt_id(
    ctx: *const (),
    gas: &mut u64,
    template_id: u8,
    vars: *const u8,
) -> interfaces::BytesResult {
    let ctx = get_ptr(ctx);
    let vars = unsafe { CStr::from_ptr(vars as *const std::ffi::c_char) };
    let res = vars
        .to_str()
        .map_err(|e| anyhow::Error::from(e))
        .and_then(|vars| ctx.exec_prompt_id(gas, template_id, vars));
    interfaces::serialize_result(res)
}

#[cfg(test)]
#[allow(non_upper_case_globals)]
mod tests {
    use std::sync::atomic::AtomicU32;

    use genvm_modules_common::interfaces::llm_functions_api::VERSION;

    use crate::Impl;

    mod conf {
        pub const openai: &str = r#"{
            "host": "https://api.openai.com",
            "provider": "openai",
            "model": "gpt-4o-mini",
            "key_env_name": "OPENAIKEY"
        }"#;

        pub const heurist: &str = r#"{
            "host": "https://llm-gateway.heurist.xyz",
            "provider": "heurist",
            "model": "meta-llama/llama-3.3-70b-instruct",
            "key_env_name": "HEURISTKEY"
        }"#;

        pub const anthropic: &str = r#"{
            "host": "https://api.anthropic.com",
            "provider": "anthropic",
            "model": "claude-3-5-sonnet-20241022",
            "key_env_name": "ANTHROPICKEY"
        }"#;

        pub const xai: &str = r#"{
            "host": "https://api.x.ai/v1",
            "provider": "xai",
            "model": "grok-2-1212",
            "key_env_name": "XAIKEY"
        }"#;

        pub const google: &str = r#"{
            "host": "https://generativelanguage.googleapis.com",
            "provider": "google",
            "model": "gemini-1.5-flash",
            "key_env_name": "GEMINIKEY"
        }"#;
    }

    extern "C-unwind" fn no_thread_pool(
        _: *const (),
        _: *const (),
        _: extern "C-unwind" fn(*const ()),
    ) {
    }

    fn do_test_text(conf: &str) {
        use genvm_modules_common::*;
        let should_quit = AtomicU32::new(0);
        let mut imp = Impl::try_new(&CtorArgs {
            version: VERSION,
            thread_pool: SharedThreadPoolABI {
                ctx: std::ptr::null(),
                submit_task: no_thread_pool,
            },
            module_config: conf.as_ptr(),
            module_config_len: conf.len(),
            log_fd: 2,
            should_quit: should_quit.as_ptr(),
        })
        .unwrap();

        let mut fake_gas = 0;
        let res = imp
            .exec_prompt(
                &mut fake_gas,
                "{}",
                "Respond with \"yes\" (without quotes) and only this word",
            )
            .unwrap();

        assert_eq!(res.to_lowercase().trim(), "yes")
    }

    fn do_test_json(conf: &str) {
        use anyhow::Context;
        use genvm_modules_common::*;

        let should_quit = AtomicU32::new(0);
        let mut imp = Impl::try_new(&CtorArgs {
            version: VERSION,
            thread_pool: SharedThreadPoolABI {
                ctx: std::ptr::null(),
                submit_task: no_thread_pool,
            },
            module_config: conf.as_ptr(),
            module_config_len: conf.len(),
            log_fd: 2,
            should_quit: should_quit.as_ptr(),
        })
        .unwrap();

        let mut fake_gas = 0;
        let res = imp.exec_prompt(&mut fake_gas, "{\"response_format\": \"json\"}", "respond with json object containing single key \"result\" and associated value being a random integer from 0 to 100 (inclusive), it must be number, not wrapped in quotes").unwrap();

        let res: serde_json::Value = serde_json::from_str(&res)
            .with_context(|| format!("result is {}", &res))
            .unwrap();
        let res = res.as_object().unwrap();
        assert_eq!(res.len(), 1);
        let res = res.get("result").unwrap().as_i64().unwrap();
        assert!(res >= 0 && res <= 100)
    }

    macro_rules! make_test {
        ($conf:ident) => {
            mod $conf {
                #[test]
                fn text() {
                    crate::tests::do_test_text(crate::tests::conf::$conf)
                }
                #[test]
                fn json() {
                    crate::tests::do_test_json(crate::tests::conf::$conf)
                }
            }
        };
    }

    make_test!(openai);
    make_test!(heurist);
    make_test!(anthropic);
    //make_test!(xai);
    make_test!(google);
}
