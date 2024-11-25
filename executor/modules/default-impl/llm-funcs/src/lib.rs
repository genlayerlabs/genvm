use anyhow::Result;
use genvm_modules_common::*;
use serde_derive::Deserialize;

use std::collections::HashMap;
use std::ffi::CStr;

use genvm_modules_common::interfaces::web_functions_api;

mod response;
mod string_templater;

genvm_modules_common::default_base_functions!(web_functions_api, Impl);

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum LLLMProvider {
    Ollama,
    Openai,
    Simulator,
}

struct Impl {
    config: Config,
    openai_key: String,
}

impl Drop for Impl {
    fn drop(&mut self) {}
}

fn default_equivalence_prompt() -> String {
    "Given the equivalence principle '#{principle}', decide whether the following two outputs can be considered equivalent.\nLeader's Output: #{leader_answer}\n\nValidator's Output: #{validator_answer}\n\nRespond with: true or false".into()
}

fn default_equivalence_prompt_non_comparative() -> String {
    "Given the following task '#{task}', decide whether the following output is a valid result of doing this task for the given input.\nOutput: #{output}\n\nInput: #{input}\n\nRespond only with: true or false".into()
}

#[derive(Deserialize)]
struct Config {
    host: String,
    provider: LLLMProvider,
    model: String,
    #[serde(default = "default_equivalence_prompt")]
    equivalence_prompt: String,
    #[serde(default = "default_equivalence_prompt_non_comparative")]
    equivalence_prompt_non_comparative: String,
}

#[derive(Deserialize)]
struct EqPrinciplePromptComparative {
    leader_answer: String,
    validator_answer: String,
    principle: String,
}

#[derive(Deserialize)]
struct EqPrinciplePromptNonComparative {
    task: String,
    input: String,
    output: String,
}

impl Impl {
    fn try_new(args: &CtorArgs) -> Result<Self> {
        let conf: &str = args.config()?;
        let config: Config = serde_json::from_str(conf)?;
        Ok(Impl {
            config,
            openai_key: std::env::var("OPENAIKEY").unwrap_or("".into()),
        })
    }

    fn exec_prompt_impl(&mut self, gas: &mut u64, _config: &str, prompt: &str) -> Result<String> {
        match self.config.provider {
            LLLMProvider::Ollama => {
                let request = serde_json::json!({
                    "model": &self.config.model,
                    "prompt": prompt,
                    "stream": false,
                });
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
                let request = serde_json::json!({
                    "model": &self.config.model,
                    "messages": [{
                        "role": "user",
                        "content": prompt,
                    }],
                    "max_completion_tokens": 1000,
                    "stream": false,
                    "temperature": 0.7,
                });
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
                    "params": [&self.config.model, prompt],
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
        eprintln!("Prompt {prompt} ===> {res:?}");
        res
    }

    fn eq_principle_prompt_comparative(&mut self, gas: &mut u64, data: &str) -> Result<bool> {
        let data: EqPrinciplePromptComparative = serde_json::from_str(data)?;
        let map = HashMap::from([
            ("leader_answer".into(), data.leader_answer),
            ("validator_answer".into(), data.validator_answer),
            ("principle".into(), data.principle),
        ]);
        let new_prompt = string_templater::patch_str(&map, &self.config.equivalence_prompt)?;
        let res = self.exec_prompt(gas, "{}".into(), &new_prompt)?;
        answer_is_bool(res)
    }

    fn eq_principle_prompt_non_comparative(&mut self, gas: &mut u64, data: &str) -> Result<bool> {
        let data: EqPrinciplePromptNonComparative = serde_json::from_str(data)?;
        let map = HashMap::from([
            ("task".into(), data.task),
            ("input".into(), data.input),
            ("output".into(), data.output),
        ]);
        let new_prompt =
            string_templater::patch_str(&map, &self.config.equivalence_prompt_non_comparative)?;
        let res = self.exec_prompt(gas, "{}".into(), &new_prompt)?;
        answer_is_bool(res)
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
) -> interfaces::CStrResult {
    let ctx = get_ptr(ctx);
    let config = unsafe { CStr::from_ptr(config as *const std::ffi::c_char) };
    let prompt = unsafe { CStr::from_ptr(prompt as *const std::ffi::c_char) };
    config
        .to_str()
        .map_err(|e| anyhow::Error::from(e))
        .and_then(|config| {
            prompt
                .to_str()
                .map_err(|e| anyhow::Error::from(e))
                .and_then(|prompt| ctx.exec_prompt(gas, config, prompt))
        })
        .into()
}

#[no_mangle]
pub extern "C-unwind" fn eq_principle_prompt_comparative(
    ctx: *const (),
    gas: &mut u64,
    data: *const u8,
) -> interfaces::BoolResult {
    let ctx = get_ptr(ctx);
    let data = unsafe { CStr::from_ptr(data as *const std::ffi::c_char) };
    data.to_str()
        .map_err(|e| anyhow::Error::from(e))
        .and_then(|data| ctx.eq_principle_prompt_comparative(gas, data))
        .into()
}

#[no_mangle]
pub extern "C-unwind" fn eq_principle_prompt_non_comparative(
    ctx: *const (),
    gas: &mut u64,
    data: *const u8,
) -> interfaces::BoolResult {
    let ctx = get_ptr(ctx);
    let data = unsafe { CStr::from_ptr(data as *const std::ffi::c_char) };
    data.to_str()
        .map_err(|e| anyhow::Error::from(e))
        .and_then(|data| ctx.eq_principle_prompt_non_comparative(gas, data))
        .into()
}
