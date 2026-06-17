#![allow(non_upper_case_globals, dead_code)]

use std::collections::BTreeMap;
use std::collections::HashMap;

use anyhow::Context as _;
use genvm_common::sync;
use genvm_common::templater;

use genvm_modules::common;
use genvm_modules::llm::providers::Provider as _;
use genvm_modules::llm::{self, config, prompt};
use genvm_modules::scripting;

fn is_overloaded(e: &anyhow::Error) -> bool {
    let e = match e.downcast_ref::<common::ModuleError>() {
        None => return false,
        Some(e) => e,
    };

    if !e
        .causes
        .iter()
        .any(|e| e == &common::ErrorKind::STATUS_NOT_OK.to_string())
    {
        return true;
    }

    match e.ctx.get("status").and_then(|x| x.as_num()) {
        None => false,
        Some(status) => [408, 503, 429, 504, 529].contains(&(status as i32)),
    }
}

mod conf {
    pub const openai: &str = r#"{
        "host": "https://openrouter.ai/api",
        "provider": "openai-compatible",
        "models": {
            "openrouter/auto": { "supports_json": true }
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
        "models": { "claude-haiku-4-5-20251001" : {} },
        "key": "${ENV[ANTHROPICKEY]}"
    }"#;

    pub const xai: &str = r#"{
        "host": "https://api.x.ai",
        "provider": "openai-compatible",
        "models": { "grok-3" : { "supports_json": true } },
        "key": "${ENV[XAIKEY]}"
    }"#;

    pub const google: &str = r#"{
        "host": "https://generativelanguage.googleapis.com",
        "provider": "google",
        "models": { "gemini-2.5-flash": { "supports_json": true } },
        "key": "${ENV[GEMINIKEY]}"
    }"#;

    pub const atoma: &str = r#"{
        "host": "https://api.atoma.network",
        "provider": "openai-compatible",
        "models": { "meta-llama/Llama-3.3-70B-Instruct": {} },
        "key": "${ENV[ATOMAKEY]}"
    }"#;
}

fn make_test_ctx() -> anyhow::Result<scripting::CtxPart> {
    Ok(scripting::CtxPart {
        client: common::create_client()?,
        metrics: sync::DArc::new(scripting::Metrics::default()),
        node_address: "test_node".to_owned(),
        sign_headers: std::sync::Arc::new(BTreeMap::new()),
        sign_url: std::sync::Arc::from("test_url"),
        sign_vars: BTreeMap::new(),
        hello: std::sync::Arc::new(genvm_modules_interfaces::GenVMHello {
            genvm_id: genvm_modules_interfaces::GenVMId(999),
            role: genvm_modules_interfaces::Role::Leader,
            host_data: genvm_modules_interfaces::HostData {
                tx_id: "test_tx".to_owned(),
                node_address: "test_node".to_owned(),
                rest: serde_json::Map::new(),
            },
            gas_data: std::collections::BTreeMap::new(),
            initial_time_units_allocation: 0,
        }),
    })
}

fn parse_backend(conf: &str) -> anyhow::Result<config::BackendConfig> {
    let backend: serde_json::Value = serde_json::from_str(conf)?;
    let mut vars = HashMap::new();
    for (mut name, value) in std::env::vars() {
        name.insert_str(0, "ENV[");
        name.push(']');

        vars.insert(name, value);
    }
    let backend =
        genvm_common::templater::patch_json(&vars, backend, &templater::DOLLAR_UNFOLDER_RE)?;
    Ok(serde_json::from_value(backend)?)
}

async fn do_test_text(conf: &str) -> anyhow::Result<()> {
    common::tests::setup();

    let backend = parse_backend(conf)?;
    if backend.key.is_empty() {
        eprintln!("skipping test: API key is not set");
        return Ok(());
    }
    let provider = backend.to_provider();

    let ctx = make_test_ctx()?;

    let res = provider
        .exec_prompt_text(
            &ctx,
            &prompt::Internal {
                system_message: None,
                temperature: 0.7,
                user_message: llm::TEST_PROMPT_FOR_OK.to_owned(),
                images: Vec::new(),
                max_tokens: 500,
                use_max_completion_tokens: true,
                seed: Some(42),
                extra: Default::default(),
                extra_merge_strategy: Default::default(),
                timeout: None,
            },
            backend
                .script_config
                .models
                .first_key_value()
                .context("no models configured")?
                .0,
        )
        .await;

    let res = match res {
        Ok(res) => res,
        Err(e) if is_overloaded(&e) => {
            eprintln!("Overloaded, skipping test: {e}");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    res.tokens.sanity_check()?;

    let text = res.result.trim().to_lowercase();

    anyhow::ensure!(text == "ok", "expected 'ok', got '{text}'");
    Ok(())
}

const BIG_PROMPT: &str = r#"
    🌆 Poem Prompt: The Shadow Citizen
    Task: Write a poem, approximately 16-20 lines, about the urban rat. Your goal is to move beyond the simple idea of "pest" and explore the rat as a complex, parallel inhabitant of the city.

    Core Theme: Focus on the rat as a secret-keeper or a historian of the discarded. It moves through the spaces we ignore—the subway tunnels, the forgotten foundations, the labyrinth of pipes. It thrives on what we throw away.

    Guiding Questions & Imagery:

    Perspective: Is the poem from the rat's point of view, or from an observer who suddenly sees the rat in a new light?

    Sensory Details: What does it hear? The "rumble of the steel train" from below? The "whispers of the lost" in the alley?

    The "Kingdom": Describe its environment. Is it a "concrete maze," a "kingdom of rust and refuse," or a "shadow empire"?

    Contrast: How does its quick, intelligent, and cautious life contrast with the loud, oblivious human world above? Consider its "onyx eye" reflecting the "neon glare."

    Your challenge: Craft a portrait that is both gritty and graceful. Acknowledge its maligned status but give it a sense of agency, intelligence, and undeniable belonging to the city's hidden pulse.
"#;

async fn do_test_text_out_of_tokens(conf: &str) -> anyhow::Result<()> {
    common::tests::setup();

    let backend = parse_backend(conf)?;
    if backend.key.is_empty() {
        eprintln!("skipping test: API key is not set");
        return Ok(());
    }
    let provider = backend.to_provider();

    let ctx = make_test_ctx()?;

    let res = provider
        .exec_prompt_text(
            &ctx,
            &prompt::Internal {
                system_message: None,
                temperature: 0.7,
                user_message: BIG_PROMPT.to_owned(),
                images: Vec::new(),
                max_tokens: 50,
                use_max_completion_tokens: true,
                seed: Some(123),
                extra: Default::default(),
                extra_merge_strategy: Default::default(),
                timeout: None,
            },
            backend
                .script_config
                .models
                .first_key_value()
                .context("no models configured")?
                .0,
        )
        .await;

    let res = match res {
        Ok(res) => res,
        Err(e) if is_overloaded(&e) => {
            eprintln!("Overloaded, skipping test: {e}");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    res.tokens.sanity_check()?;

    let text = res.result.trim().to_lowercase();

    println!("result is {text}");
    Ok(())
}

async fn do_test_json(conf: &str) -> anyhow::Result<()> {
    common::tests::setup();

    let backend = parse_backend(conf)?;
    if backend.key.is_empty() {
        eprintln!("skipping test: API key is not set");
        return Ok(());
    }

    if !backend
        .script_config
        .models
        .first_key_value()
        .context("no models configured")?
        .1
        .supports_json
    {
        return Ok(());
    }

    let provider = backend.to_provider();

    let ctx = make_test_ctx()?;

    const PROMPT: &str = r#"respond with json object containing single key "result" and associated value being a random integer from 0 to 100 (inclusive), it must be number, not wrapped in quotes. This object must not be wrapped into other objects. Example: {"result": 10}"#;
    let res = provider
        .exec_prompt_json(
            &ctx,
            &prompt::Internal {
                system_message: Some("respond with json".to_owned()),
                temperature: 0.7,
                user_message: PROMPT.to_owned(),
                images: Vec::new(),
                max_tokens: 500,
                use_max_completion_tokens: true,
                seed: Some(456),
                extra: Default::default(),
                extra_merge_strategy: Default::default(),
                timeout: None,
            },
            backend
                .script_config
                .models
                .first_key_value()
                .context("no models configured")?
                .0,
        )
        .await;
    eprintln!("{res:?}");

    let res = match res {
        Ok(res) => res,
        Err(e) if is_overloaded(&e) => {
            eprintln!("Overloaded, skipping test: {e}");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    res.tokens.sanity_check()?;

    let as_val = serde_json::Value::Object(res.result);

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
    ]
    .into_iter()
    .flatten()
    {
        anyhow::ensure!(
            (0..=100).contains(&potential),
            "result {potential} not in 0..=100"
        );
        return Ok(());
    }
    anyhow::bail!("no result found in {as_val:?}");
}

async fn do_test_json_out_of_tokens(conf: &str) -> anyhow::Result<()> {
    common::tests::setup();

    let backend = parse_backend(conf)?;
    if backend.key.is_empty() {
        eprintln!("skipping test: API key is not set");
        return Ok(());
    }

    if !backend
        .script_config
        .models
        .first_key_value()
        .context("no models configured")?
        .1
        .supports_json
    {
        return Ok(());
    }

    let provider = backend.to_provider();

    let ctx = make_test_ctx()?;

    const PROMPT: &str = r#"respond with json object containing two keys. First key is a poem about rats and second key "result" and associated value being a random integer from 0 to 100 (inclusive), it must be number, not wrapped in quotes. This object must not be wrapped into other objects. Example: {"poem": "A kingdom built of rust and steam, Beneath the concrete, cold and vast, He navigates the broken dream, A living shadow, built to last. He slips between the pipe and wire, A citizen of drain and seam, Ignoring all the surface fire Of our oblivious, waking stream.", "result": 10}"#;
    let res = provider
        .exec_prompt_json(
            &ctx,
            &prompt::Internal {
                system_message: Some("respond with json".to_owned()),
                temperature: 0.7,
                user_message: PROMPT.to_owned(),
                images: Vec::new(),
                max_tokens: 50,
                use_max_completion_tokens: true,
                seed: Some(789),
                extra: Default::default(),
                extra_merge_strategy: Default::default(),
                timeout: None,
            },
            backend
                .script_config
                .models
                .first_key_value()
                .context("no models configured")?
                .0,
        )
        .await;
    eprintln!("{res:?}");

    let res = match res {
        Ok(res) => res,
        Err(e) if is_overloaded(&e) => {
            eprintln!("Overloaded, skipping test: {e}");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    res.tokens.sanity_check()?;
    Ok(())
}

async fn with_retries<F, Fut>(f: F) -> anyhow::Result<()>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<()>>,
{
    const RETRIES: u32 = 3;
    for attempt in 1..=RETRIES {
        match f().await {
            Ok(()) => return Ok(()),
            Err(e) if attempt < RETRIES => {
                eprintln!("attempt {attempt}/{RETRIES} failed: {e:#}, retrying in 5s");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

macro_rules! make_test {
    ($conf:ident) => {
        mod $conf {
            use genvm_modules::common;

            #[tokio::test]
            async fn text() -> anyhow::Result<()> {
                let conf = super::conf::$conf;
                super::with_retries(|| {
                    common::test_with_genvm_id(genvm_modules_interfaces::GenVMId(999), async {
                        super::do_test_text(conf).await
                    })
                })
                .await
            }
            #[tokio::test]
            async fn json() -> anyhow::Result<()> {
                let conf = super::conf::$conf;
                super::with_retries(|| {
                    common::test_with_genvm_id(genvm_modules_interfaces::GenVMId(999), async {
                        super::do_test_json(conf).await
                    })
                })
                .await
            }

            #[tokio::test]
            async fn text_out_of_tokens() -> anyhow::Result<()> {
                let conf = super::conf::$conf;
                super::with_retries(|| {
                    common::test_with_genvm_id(genvm_modules_interfaces::GenVMId(999), async {
                        super::do_test_text_out_of_tokens(conf).await
                    })
                })
                .await
            }

            #[tokio::test]
            async fn json_out_of_tokens() -> anyhow::Result<()> {
                let conf = super::conf::$conf;
                super::with_retries(|| {
                    common::test_with_genvm_id(genvm_modules_interfaces::GenVMId(999), async {
                        super::do_test_json_out_of_tokens(conf).await
                    })
                })
                .await
            }
        }
    };
}

make_test!(openai);
make_test!(anthropic);
make_test!(google);
//make_test!(xai);

make_test!(heurist);
make_test!(heurist_deepseek);
//make_test!(atoma);
