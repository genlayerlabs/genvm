use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::Context as _;
use genvm_common::*;
use mlua::LuaSerdeExt as _;

use genvm_modules::common;
use genvm_modules::scripting::{self, Response};

struct TestCtx {
    scripting: scripting::CtxPart,
}

async fn create_test_vm() -> scripting::UserVM<(), (), TestCtx> {
    let mut cwd = std::env::current_dir().unwrap();
    cwd.pop();
    cwd.push("install");
    cwd.push("lib");
    cwd.push("genvm-lua");
    let cwd = cwd
        .canonicalize()
        .with_context(|| format!("canonicalizing {:?}", cwd))
        .unwrap();
    let mut extra_path = cwd.to_str().unwrap().to_owned();
    extra_path.push_str("/?.lua");

    let conf = sync::DArc::new(common::ModuleBaseConfig {
        bind_address: None,
        vm_count: 1,
        lua_script_path: "".to_owned(),
        lua_path: extra_path,
        signer_headers: Arc::new(BTreeMap::new()),
        signer_url: Arc::from(""),
        data_dir: String::new(),
    });

    scripting::UserVM::create(
        &conf.clone(),
        |_| async { Ok(()) },
        Box::new(move |vm, table, ctx: &sync::DArc<TestCtx>| {
            let scripting = ctx.gep(|x| &x.scripting);
            scripting::setup_lua_default_ctx(scripting, vm, table)?;
            Ok(())
        }),
    )
    .await
    .unwrap()
}

fn create_test_ctx() -> sync::DArc<TestCtx> {
    let hello = common::tests::get_hello();
    let metrics = sync::DArc::new(scripting::Metrics::default());
    let conf = sync::DArc::new(common::ModuleBaseConfig {
        bind_address: None,
        vm_count: 1,
        lua_script_path: "".to_owned(),
        lua_path: "".to_owned(),
        signer_headers: Arc::new(BTreeMap::new()),
        signer_url: Arc::from(""),
        data_dir: String::new(),
    });
    let scripting = scripting::create_ctx_part(&hello, &conf, metrics, false).unwrap();
    sync::DArc::new(TestCtx { scripting })
}

async fn test_status(status: u16) {
    common::tests::setup();

    let uvm = create_test_vm().await;

    let mut cwd = std::env::current_dir().unwrap();
    cwd.push("tests");
    cwd.push("lua");
    cwd.push("get_status.lua");
    let test_script = std::fs::read_to_string(cwd).unwrap();

    let chunk = uvm.vm.load(test_script);
    chunk.exec().unwrap();

    let f: mlua::Function = uvm.vm.globals().get("Test").unwrap();

    let test_ctx = create_test_ctx();

    let (_, ctx_lua) = uvm.create_ctx(&test_ctx).unwrap();

    let res: mlua::Value = f.call_async((ctx_lua, status.to_string())).await.unwrap();

    let res: Response = uvm.vm.from_value(res).unwrap();

    assert_eq!(res.status, status);
}

#[tokio::test]
async fn test_status_200() {
    test_status(200).await;
}

#[tokio::test]
async fn test_status_404() {
    test_status(404).await;
}

#[tokio::test]
async fn test_echo_post() {
    common::tests::setup();

    let uvm = create_test_vm().await;

    let mut cwd = std::env::current_dir().unwrap();
    cwd.push("tests");
    cwd.push("lua");
    cwd.push("bytes.lua");
    let test_script = std::fs::read_to_string(cwd).unwrap();

    let chunk = uvm.vm.load(test_script);
    chunk.exec().unwrap();

    let f: mlua::Function = uvm.vm.globals().get("Test").unwrap();

    let expected = b"\xde\xad\xbe\xef";

    let test_ctx = create_test_ctx();

    let (_, ctx_lua) = uvm.create_ctx(&test_ctx).unwrap();

    let res: mlua::Value = f.call_async((ctx_lua,)).await.unwrap();

    let res: Response = uvm.vm.from_value(res).unwrap();

    log_trace!(response:serde = res; "response");

    assert_eq!(res.status, 200);
    assert_eq!(res.body, expected);
}
