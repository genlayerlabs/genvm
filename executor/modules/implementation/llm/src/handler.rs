use genvm_modules_impl_common::{MessageHandler, MessageHandlerProvider, ModuleResult};
use std::{collections::BTreeMap, sync::Arc};

use crate::{config, providers, scripting};
use genvm_modules_interfaces::llm as llm_iface;

pub struct Handler {
    pub providers: Arc<BTreeMap<String, Box<dyn providers::Provider + Send + Sync>>>,
    config: Arc<config::Config>,
    user_vm: Arc<scripting::UserVM>,
    hello: genvm_modules_interfaces::GenVMHello,
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
        Ok(HandlerWrapper(Arc::new(Handler {
            providers: self.providers.clone(),
            user_vm: self.user_vm.clone(),
            config: self.config.clone(),
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
                self.0.exec_prompt_template(payload).await
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
        prompt: &str,
        model: &str,
        provider_id: &str,
        mode: llm_iface::OutputFormat,
    ) -> ModuleResult<llm_iface::PromptAnswer> {
        let provider = self
            .providers
            .get(provider_id)
            .ok_or_else(|| anyhow::anyhow!("absent provider_id `{provider_id}`"))?;

        let res = match mode {
            llm_iface::OutputFormat::Text => provider
                .exec_prompt_text(prompt, model)
                .await
                .map(llm_iface::PromptAnswer::Text),
            llm_iface::OutputFormat::JSON => provider
                .exec_prompt_json(prompt, model)
                .await
                .map(llm_iface::PromptAnswer::Object),
        };

        res.inspect_err(|err| {
            log::error!(prompt = prompt, model = model, mode:? = mode, provider_id = provider_id, error = genvm_common::log_error(err), cookie = self.hello.cookie; "prompt execution error");
        })
    }

    async fn exec_prompt(
        &self,
        zelf: Arc<Handler>,
        payload: llm_iface::PromptPayload,
    ) -> ModuleResult<llm_iface::PromptAnswer> {
        log::debug!(payload:serde = payload, cookie = self.hello.cookie; "exec_prompt start");

        let llm_iface::PromptPart::Text(prompt) = &payload.parts[0];
        let res = self.user_vm.greybox(zelf, prompt).await?;
        log::debug!(result:serde = res, cookie = self.hello.cookie; "script returned");

        Ok(res)
    }

    async fn exec_prompt_template(
        &self,
        payload: llm_iface::PromptTemplatePayload,
    ) -> ModuleResult<llm_iface::PromptAnswer> {
        let (provider_id, provider) = self.providers.first_key_value().unwrap();

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

                let model = self
                    .config
                    .backends
                    .get(provider_id)
                    .unwrap()
                    .script_config
                    .models
                    .first()
                    .unwrap();

                provider
                    .exec_prompt_text(&new_prompt, model)
                    .await
                    .map(llm_iface::PromptAnswer::Text)
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

                let model = self
                    .config
                    .backends
                    .get(provider_id)
                    .unwrap()
                    .script_config
                    .models
                    .first()
                    .unwrap();

                provider
                    .exec_prompt_bool_reason(&new_prompt, model)
                    .await
                    .map(llm_iface::PromptAnswer::Bool)
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

                let model = self
                    .config
                    .backends
                    .get(provider_id)
                    .unwrap()
                    .script_config
                    .models
                    .first()
                    .unwrap();

                provider
                    .exec_prompt_bool_reason(&new_prompt, model)
                    .await
                    .map(llm_iface::PromptAnswer::Bool)
            }
        }
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

    use crate::config;
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
                "Respond with a single word \"yes\" (without quotes) and only this word, lowercase",
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

        const PROMPT: &str = "respond with json object containing single key \"result\" and associated value being a random integer from 0 to 100 (inclusive), it must be number, not wrapped in quotes";
        let res = provider
            .exec_prompt_json(PROMPT, &backend.script_config.models[0])
            .await;
        eprintln!("{res:?}");
        let res = res.unwrap();
        eprintln!("{res:?}");
        assert_eq!(res.len(), 1);
        let res = res.get("result").unwrap().as_i64().unwrap();
        assert!(res >= 0 && res <= 100)
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
