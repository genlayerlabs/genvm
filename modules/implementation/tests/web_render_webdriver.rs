// Regression tests for the web module's Render transport: the request to the
// operator-configured `webdriver_host` must bypass the SSRF-filtering client.
// In docker topologies the webdriver hostname resolves to an RFC1918 address,
// and routing that hop through the filtered client crash-looped every docker
// deploy with ADDRESS_FORBIDDEN. `localhost.direct` (public DNS, resolves only
// to loopback) stands in for the private webdriver hostname.

use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::Context as _;
use genvm_common::*;
use tokio::io::AsyncWriteExt;

use genvm_modules::common;
use genvm_modules::scripting;

struct TestCtx {
    scripting: scripting::CtxPart,
}

// Serves a single fake webdriver `/render` response and exits.
async fn serve_webdriver_once(body: &'static [u8]) -> std::net::SocketAddr {
    let server = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr().unwrap();

    tokio::spawn(async move {
        let (mut client, _) = server.accept().await.unwrap();
        let response = format!(
            "HTTP/1.1 200 OK\r\nresulting-status: 200\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n",
            body.len()
        );
        client.write_all(response.as_bytes()).await.unwrap();
        client.write_all(body).await.unwrap();
        client.shutdown().await.unwrap();
    });

    addr
}

fn lua_lib_path() -> String {
    let mut cwd = std::env::current_dir().unwrap();
    cwd.pop();
    cwd.push("install");
    cwd.push("lib");
    cwd.push("genvm-lua");
    let cwd = cwd
        .canonicalize()
        .with_context(|| format!("canonicalizing {cwd:?}"))
        .unwrap();
    let mut extra_path = cwd.to_str().unwrap().to_owned();
    extra_path.push_str("/?.lua");
    extra_path
}

fn web_script_path() -> String {
    let mut cwd = std::env::current_dir().unwrap();
    cwd.pop();
    cwd.push("install");
    cwd.push("config");
    cwd.push("genvm-web-default.lua");
    cwd.canonicalize().unwrap().to_str().unwrap().to_owned()
}

// Boots a VM with the real `genvm-web-default.lua` and a hand-built `__web`
// global, mirroring what `create_web_module` sets up.
async fn create_web_vm(webdriver_host: String) -> scripting::UserVM<(), (), TestCtx> {
    let conf = sync::DArc::new(common::ModuleBaseConfig {
        bind_address: None,
        vm_count: 1,
        lua_script_path: "".to_owned(),
        lua_path: lua_lib_path(),
        signer_headers: Arc::new(BTreeMap::new()),
        signer_url: Arc::from(""),
        data_dir: String::new(),
    });

    scripting::UserVM::create(
        &conf.clone(),
        move |vm: mlua::Lua| async move {
            let config = vm.create_table()?;
            config.set("webdriver_host", webdriver_host)?;
            config.set("always_allow_hosts", vm.create_table()?)?;

            let allowed_tld = vm.create_table()?;
            allowed_tld.set("org", true)?;

            let web = vm.create_table()?;
            web.set("config", config)?;
            web.set("allowed_tld", allowed_tld)?;
            vm.globals().set("__web", web)?;

            scripting::load_script(&vm, web_script_path()).await?;
            Ok(())
        },
        Box::new(move |vm, table, ctx: &sync::DArc<TestCtx>| {
            let scripting = ctx.gep(|x| &x.scripting);
            scripting::setup_lua_default_ctx(scripting, vm, table)?;
            Ok(())
        }),
    )
    .await
    .unwrap()
}

// Like the real web module (`filter_dns: true`): the default client filters
// non-globally-routable addresses.
fn create_filtered_ctx() -> sync::DArc<TestCtx> {
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
    let scripting = scripting::create_ctx_part(&hello, &conf, metrics, true).unwrap();
    sync::DArc::new(TestCtx { scripting })
}

#[tokio::test]
async fn render_reaches_loopback_only_webdriver_host() {
    common::tests::setup();

    let body = b"rendered page text";
    let addr = serve_webdriver_once(body).await;
    let webdriver_host = format!("http://localhost.direct:{}", addr.port());

    let uvm = create_web_vm(webdriver_host).await;
    let test_ctx = create_filtered_ctx();
    let (_, ctx_lua) = uvm.create_ctx(&test_ctx).unwrap();

    let payload = uvm.vm.create_table().unwrap();
    payload.set("url", "https://example.org/").unwrap();
    payload.set("mode", "text").unwrap();
    payload.set("wait_after_loaded", 0).unwrap();

    let render: mlua::Function = uvm.vm.globals().get("Render").unwrap();
    let res: mlua::Table = uvm
        .call_fn(&render, (ctx_lua, payload))
        .await
        .expect("Render must reach the webdriver despite its loopback-only hostname");

    let text: mlua::String = res.get("text").unwrap();
    assert_eq!(text.as_bytes(), body.as_slice());
}

#[tokio::test]
async fn filtered_default_client_still_blocks_loopback() {
    common::tests::setup();

    // Sanity check for the test above: the same ctx blocks a plain (filtered)
    // request to the same loopback-only hostname, so Render succeeding is
    // attributable to its `unfiltered` opt-out, not to a permissive filter.
    let uvm = create_web_vm("http://unused.invalid".to_owned()).await;
    let test_ctx = create_filtered_ctx();
    let (_, ctx_lua) = uvm.create_ctx(&test_ctx).unwrap();

    let f: mlua::Function = uvm
        .vm
        .load(
            r#"
            local lib = require("lib-genvm")
            return function(ctx, url)
                return lib.rs.request(ctx, {
                    method = "GET",
                    url = url,
                    headers = {},
                    error_on_status = true,
                })
            end
            "#,
        )
        .eval()
        .unwrap();

    let err = uvm
        .call_fn::<mlua::Value>(&f, (ctx_lua, "http://localhost.direct/"))
        .await
        .expect_err("filtered client must reject a loopback-only host");

    let err_str = format!("{err:?}");
    assert!(
        err_str.contains("ADDRESS_FORBIDDEN"),
        "unexpected error: {err_str}"
    );
}
