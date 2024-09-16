use anyhow::Result;
use genvm_modules_common::*;
use serde_derive::Deserialize;

use std::ffi::CStr;

use genvm_modules_common::interfaces::web_functions_api;

genvm_modules_common::default_base_functions!(web_functions_api, Impl);

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum LLLMProvider {
    Ollama,
}

struct Impl {
    config: Config,
}

impl Drop for Impl {
    fn drop(&mut self) {}
}

#[derive(Deserialize)]
struct Config {
    host: String,
    provider: LLLMProvider,
    model: String,
}

impl Impl {
    fn try_new(conf: &CStr) -> Result<Self> {
        let config: Config = serde_json::from_str(conf.to_str()?)?;
        Ok(Impl { config })
    }

    fn call_llm(&mut self, gas: &mut u64, _config: &CStr, prompt: &CStr) -> Result<String> {
        let prompt = prompt.to_str()?;
        match self.config.provider {
            LLLMProvider::Ollama => {
                let request = serde_json::json!({
                    "model": &self.config.model,
                    "prompt": prompt,
                    "stream": false,
                });
                let res = ureq::post(&format!("{}/api/generate", self.config.host))
                    .send_bytes(serde_json::to_string(&request)?.as_bytes())?;
                let res = res.into_string()?;
                let val: serde_json::Value = serde_json::from_str(&res)?;
                let response = val
                    .as_object()
                    .and_then(|v| v.get("response"))
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("can't get response field {}", &res))?;
                let mut eval_duration = val
                    .as_object()
                    .and_then(|v| v.get("eval_duration"))
                    .and_then(|v| v.as_u64())
                    .ok_or(anyhow::anyhow!("can't get eval_duration field {}", &res))?;
                eval_duration <<= 4;
                *gas -= eval_duration.min(*gas);
                Ok(response.into())
            }
        }
    }
}

#[no_mangle]
pub extern "C-unwind" fn call_llm(
    ctx: *const (),
    gas: &mut u64,
    config: *const u8,
    prompt: *const u8,
) -> interfaces::CStrResult {
    let ctx = get_ptr(ctx);
    let config = unsafe { CStr::from_ptr(config as *const i8) };
    let prompt = unsafe { CStr::from_ptr(prompt as *const i8) };
    ctx.call_llm(gas, config, prompt).into()
}
