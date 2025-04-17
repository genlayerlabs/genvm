use genvm_modules_impl_common::{MessageHandler, MessageHandlerProvider, ModuleResult};
use std::{collections::BTreeMap, sync::Arc};

use crate::{config, prompt, providers, scripting};
use genvm_modules_interfaces::llm as llm_iface;

pub struct Handler {
    pub providers: Arc<BTreeMap<String, Box<dyn providers::Provider + Send + Sync>>>,
    user_vm: Arc<scripting::UserVM>,

    pub hello: genvm_modules_interfaces::GenVMHello,
}

#[derive(Debug)]
pub struct OverloadedError;

impl std::error::Error for OverloadedError {}

impl std::fmt::Display for OverloadedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OverloadedError")
    }
}

pub struct HandlerProvider {
    pub providers: Arc<BTreeMap<String, Box<dyn providers::Provider + Send + Sync>>>,
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
        hello: genvm_modules_interfaces::GenVMHello,
    ) -> anyhow::Result<
        impl MessageHandler<
            genvm_modules_interfaces::llm::Message,
            genvm_modules_interfaces::llm::PromptAnswer,
        >,
    > {
        let _ = &self.config; // make used

        Ok(HandlerWrapper(Arc::new(Handler {
            providers: self.providers.clone(),
            user_vm: self.user_vm.clone(),
            hello,
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
                self.0.exec_prompt_template(self.0.clone(), payload).await
            }
        }
    }

    async fn cleanup(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

impl Handler {
    pub async fn exec_prompt_in_provider(
        &self,
        prompt: &prompt::Internal,
        model: &str,
        provider_id: &str,
        format: prompt::ExtendedOutputFormat,
    ) -> ModuleResult<llm_iface::PromptAnswer> {
        log::debug!(
            prompt:serde = prompt,
            provider_id = provider_id,
            model = model,
            format:serde = format;
            "exec_in_backend"
        );

        let provider = self
            .providers
            .get(provider_id)
            .ok_or_else(|| anyhow::anyhow!("absent provider_id `{provider_id}`"))?;

        let res = match format {
            prompt::ExtendedOutputFormat::Text => provider
                .exec_prompt_text(prompt, model)
                .await
                .map(llm_iface::PromptAnswer::Text),
            prompt::ExtendedOutputFormat::JSON => provider
                .exec_prompt_json(prompt, model)
                .await
                .map(llm_iface::PromptAnswer::Object),
            prompt::ExtendedOutputFormat::Bool => provider
                .exec_prompt_bool_reason(prompt, model)
                .await
                .map(llm_iface::PromptAnswer::Bool),
        };

        res.inspect_err(|err| {
            log::error!(prompt:serde = prompt, model = model, mode:? = format, provider_id = provider_id, error = genvm_common::log_error(err), cookie = self.hello.cookie; "prompt execution error");
        })
    }

    async fn exec_prompt(
        &self,
        zelf: Arc<Handler>,
        payload: llm_iface::PromptPayload,
    ) -> ModuleResult<llm_iface::PromptAnswer> {
        log::debug!(payload:serde = payload, cookie = self.hello.cookie; "exec_prompt start");
        let res = self.user_vm.greybox(zelf, &payload).await?;
        log::debug!(result:serde = res, cookie = self.hello.cookie; "exec_prompt returned");

        Ok(res)
    }

    async fn exec_prompt_template(
        &self,
        zelf: Arc<Handler>,
        payload: llm_iface::PromptTemplatePayload,
    ) -> ModuleResult<llm_iface::PromptAnswer> {
        log::debug!(payload:serde = payload, cookie = self.hello.cookie; "exec_prompt_template start");
        let res = self.user_vm.greybox_template(zelf, payload).await?;
        log::debug!(result:serde = res, cookie = self.hello.cookie; "exec_prompt_template returned");

        Ok(res)
    }
}

#[cfg(test)]
#[allow(non_upper_case_globals, dead_code)]
mod tests {
    use std::collections::HashMap;

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

    use crate::{config, prompt};
    use genvm_common::templater;

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
        let provider = backend.to_provider(reqwest::Client::new());

        let res = provider
            .exec_prompt_text(
                &prompt::Internal {
                    system_message: None,
                    temperature: 0.7,
                    user_message: "Respond with a single word \"yes\" (without quotes) and only this word, lowercase".to_owned(),
                },
                &backend.script_config.models[0],
            )
            .await
            .unwrap();

        let res = res.trim().to_lowercase();

        assert_eq!(res, "yes");
    }

    async fn do_test_json(conf: &str) {
        setup();

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
        let provider = backend.to_provider(reqwest::Client::new());

        const PROMPT: &str = r#"respond with json object containing single key "result" and associated value being a random integer from 0 to 100 (inclusive), it must be number, not wrapped in quotes. This object must not be wrapped into other objects. Example: {"result": 10}"#;
        let res = provider
            .exec_prompt_json(
                &prompt::Internal {
                    system_message: Some("respond with json".to_owned()),
                    temperature: 0.7,
                    user_message: PROMPT.to_owned(),
                },
                &backend.script_config.models[0],
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
        ] {
            if let Some(v) = potential {
                assert!(v >= 0 && v <= 100);
                return;
            }
        }
        assert!(false);
    }

    macro_rules! make_test {
        ($conf:ident) => {
            mod $conf {
                #[tokio::test]
                async fn text() {
                    let conf = crate::handler::tests::conf::$conf;
                    genvm_modules_impl_common::test_with_cookie(conf, async {
                        crate::handler::tests::do_test_text(conf).await
                    })
                    .await;
                }
                #[tokio::test]
                async fn json() {
                    let conf = crate::handler::tests::conf::$conf;
                    genvm_modules_impl_common::test_with_cookie(conf, async {
                        crate::handler::tests::do_test_json(conf).await
                    })
                    .await;
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
