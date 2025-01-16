use anyhow::Result;
use genvm_modules_common::*;
use serde_derive::{Deserialize, Serialize};

use std::ffi::CStr;

use crate::interfaces::RecoverableError;
use genvm_modules_common::interfaces::web_functions_api;

mod response;
mod string_templater;
mod template_ids;

genvm_modules_common::default_base_functions!(web_functions_api, Impl);

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum LLLMProvider {
    Atoma,
    Ollama,
    Openai,
    Simulator,
}

struct Impl {
    config: Config,
    openai_key: String,
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
    #[serde(default = "default_equivalence_prompt_comparative")]
    equivalence_prompt_comparative: String,
    #[serde(default = "default_equivalence_prompt_non_comparative")]
    equivalence_prompt_non_comparative: String,
    #[serde(default = "default_equivalence_prompt_non_comparative_leader")]
    equivalence_prompt_non_comparative_leader: String,
}

impl Impl {
    fn try_new(args: &CtorArgs) -> Result<Self> {
        let conf: &str = args.config()?;
        let config: Config = serde_json::from_str(conf)?;
        Ok(Impl {
            config,
            log_fd: args.log_fd,
            openai_key: std::env::var("OPENAIKEY").unwrap_or("".into()),
        })
    }

    fn exec_prompt_impl(&mut self, gas: &mut u64, config: &str, prompt: &str) -> Result<String> {
        #[derive(Clone, Deserialize, Serialize)]
        #[serde(rename_all = "kebab-case")]
        enum ExecPromptConfigMode {
            Text,
            Json,
        }
        #[derive(Deserialize)]
        struct ExecPromptConfig {
            response_format: Option<ExecPromptConfigMode>,
        }
        let config: ExecPromptConfig =
            serde_json::from_str(config).map_err(RecoverableError::from_anyhow)?;
        let response_format = config
            .response_format
            .clone()
            .unwrap_or(ExecPromptConfigMode::Text);
        match self.config.provider {
            LLLMProvider::Atoma => {
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
                        .header("Authorization", &format!("Bearer {}", &self.openai_key))
                        .body(serde_json::to_string(&request)?.as_bytes())?,
                )?;
                let res = response::read(&mut res)?;
                let val: serde_json::Value = serde_json::from_str(&res)?;
                let response = val
                    .as_object()
                    .and_then(|v| v.get("choices"))
                    .and_then(|v| v.as_array())
                    .and_then(|v| v.get(0))
                    .and_then(|v| v.as_object())
                    .and_then(|v| v.get("message"))
                    .and_then(|v| v.as_object())
                    .and_then(|v| v.get("content"))
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("can't get response field {}", &res))?;
                let total_tokens = val
                    .as_object()
                    .and_then(|v| v.get("usage"))
                    .and_then(|v| v.as_object())
                    .and_then(|v| v.get("total_tokens"))
                    .and_then(|v| v.as_u64())
                    .ok_or(anyhow::anyhow!("can't get eval_duration field {}", &res))?;
                *gas -= (total_tokens << 8).min(*gas);
                Ok(response.into())
            }
            LLLMProvider::Ollama => {
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
                let res = response::read(&mut res)?;
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
            LLLMProvider::Openai => {
                let mut request = serde_json::json!({
                    "model": &self.config.model,
                    "messages": [{
                        "role": "user",
                        "content": prompt,
                    }],
                    "max_completion_tokens": 1000,
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
                        .header("Authorization", &format!("Bearer {}", &self.openai_key))
                        .body(serde_json::to_string(&request)?.as_bytes())?,
                )?;
                let res = response::read(&mut res)?;
                let val: serde_json::Value = serde_json::from_str(&res)?;
                let response = val
                    .as_object()
                    .and_then(|v| v.get("choices"))
                    .and_then(|v| v.as_array())
                    .and_then(|v| v.get(0))
                    .and_then(|v| v.as_object())
                    .and_then(|v| v.get("message"))
                    .and_then(|v| v.as_object())
                    .and_then(|v| v.get("content"))
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("can't get response field {}", &res))?;
                let total_tokens = val
                    .as_object()
                    .and_then(|v| v.get("usage"))
                    .and_then(|v| v.as_object())
                    .and_then(|v| v.get("total_tokens"))
                    .and_then(|v| v.as_u64())
                    .ok_or(anyhow::anyhow!("can't get eval_duration field {}", &res))?;
                *gas -= (total_tokens << 8).min(*gas);
                Ok(response.into())
            }
            LLLMProvider::Simulator => {
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
                let res = response::read(&mut res)?;
                let res: serde_json::Value = serde_json::from_str(&res)?;
                res.as_object()
                    .and_then(|v| v.get("result"))
                    .and_then(|v| v.as_object())
                    .and_then(|v| v.get("response"))
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .ok_or(anyhow::anyhow!("can't get response field {}", &res))
            }
        }
    }

    fn exec_prompt(&mut self, gas: &mut u64, config: &str, prompt: &str) -> Result<String> {
        let res = self.exec_prompt_impl(gas, config, prompt);
        match serde_json::to_string(&serde_json::json!({
            "prompt": prompt,
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
