use genvm_common::*;
use genvm_modules::common;
use genvm_modules::scripting::{self, send_request_get_lua_compatible_response_bytes, Metrics};
use tokio::io::AsyncWriteExt;

// Serves a single minimal HTTP/1.1 response and exits.
async fn serve_once(body: &'static [u8]) -> std::net::SocketAddr {
    let server = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr().unwrap();

    tokio::spawn(async move {
        let (mut client, _) = server.accept().await.unwrap();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n",
            body.len()
        );
        client.write_all(response.as_bytes()).await.unwrap();
        client.write_all(body).await.unwrap();
        client.shutdown().await.unwrap();
    });

    addr
}

#[tokio::test]
async fn test_request_to_localhost_resolving_host() {
    common::tests::setup();

    log_info!("starting localhost test");

    let body = b"hello from localhost";
    let addr = serve_once(body).await;

    log_info!(addr:? = addr; "bound");

    // `localhost.direct` resolves to 127.0.0.1 publicly, where our server is
    // bound, so the request reaches it over real DNS.
    let client = reqwest::ClientBuilder::new()
        .user_agent("reqwest")
        .timeout(std::time::Duration::from_secs(40))
        .build()
        .unwrap();

    let url = format!("http://localhost.direct:{}/", addr.port());
    let request = client.get(&url);

    let metrics = sync::DArc::new(Metrics::default());

    let res =
        send_request_get_lua_compatible_response_bytes(&metrics, &url, request, true, usize::MAX)
            .await
            .unwrap();

    assert_eq!(res.status, 200);
    assert_eq!(res.body, body);
}

/// Builds a minimal Lua VM that exposes `__dflt`, loads `lib-web`, and returns
/// its `pin_url_to_good_host` function together with a ready-to-use ctx table.
async fn make_pin_vm() -> (
    scripting::UserVM<mlua::Function, (), scripting::CtxPart>,
    sync::DArc<scripting::CtxPart>,
) {
    let lua_dir = std::path::PathBuf::from("../install/lib/genvm-lua")
        .canonicalize()
        .unwrap();
    let lua_path = format!("{}/?.lua", lua_dir.to_str().unwrap());

    let base_config = common::ModuleBaseConfig {
        bind_address: None,
        lua_script_path: String::new(),
        vm_count: 1,
        lua_path,
        signer_url: std::sync::Arc::from(""),
        signer_headers: std::sync::Arc::new(std::collections::BTreeMap::new()),
        data_dir: String::new(),
    };

    let user_vm = scripting::UserVM::create(
        &base_config,
        |vm: mlua::Lua| async move {
            let module: mlua::Table = vm.load(r#"return require("lib-web")"#).eval()?;
            let f: mlua::Function = module.get("pin_url_to_good_host")?;
            Ok(f)
        },
        Box::new(
            |vm: &mlua::Lua, table: &mlua::Table, ctx: &sync::DArc<scripting::CtxPart>| {
                scripting::setup_lua_default_ctx(ctx.clone(), vm, table)?;
                Ok(())
            },
        ),
    )
    .await
    .unwrap();

    let hello = common::tests::get_hello();
    let base_config = sync::DArc::new(base_config);
    let metrics = sync::DArc::new(Metrics::default());
    let ctx_part =
        sync::DArc::new(scripting::create_ctx_part(&hello, &base_config, metrics).unwrap());

    (user_vm, ctx_part)
}

#[tokio::test]
async fn test_localhost_direct_is_bad() {
    common::tests::setup();

    let (user_vm, ctx_part) = make_pin_vm().await;
    let (_, ctx_value) = user_vm.create_ctx(&ctx_part).unwrap();

    // `localhost.direct` is a public DNS name that resolves only to loopback,
    // so `pin_url_to_good_host` must find no good address and raise
    // ADDRESS_FORBIDDEN.
    let res: anyhow::Result<String> = user_vm
        .call_fn(
            &user_vm.data,
            (ctx_value, "http://localhost.direct/".to_owned()),
        )
        .await;

    let err = res.expect_err("expected localhost.direct to be rejected");
    log_info!(error:ah = &err; "pin_url_to_good_host rejected localhost.direct");

    let module_err = scripting::try_unwrap_any_err(err).unwrap();
    assert!(
        module_err.causes.contains(&"ADDRESS_FORBIDDEN".to_owned()),
        "unexpected causes: {:?}",
        module_err.causes
    );
}

#[tokio::test]
async fn test_ipv6_loopback_literal_is_bad() {
    common::tests::setup();

    let (user_vm, ctx_part) = make_pin_vm().await;
    let (_, ctx_value) = user_vm.create_ctx(&ctx_part).unwrap();

    // `split_url` returns a *bracketed* host for IPv6 literals (`[::1]`), so the
    // `host .. ":" .. port` the Lua builds is a valid socket address
    // (`[::1]:80`) that resolves. `::1` then classifies as loopback =>
    // ADDRESS_FORBIDDEN. If the host were unbracketed (`::1:80`) resolution
    // would error instead, so reaching ADDRESS_FORBIDDEN guards that path.
    let res: anyhow::Result<String> = user_vm
        .call_fn(&user_vm.data, (ctx_value, "http://[::1]/".to_owned()))
        .await;

    let err = res.expect_err("expected ipv6 loopback literal to be rejected");
    log_info!(error:ah = &err; "pin_url_to_good_host rejected [::1]");

    let module_err = scripting::try_unwrap_any_err(err).unwrap();
    assert!(
        module_err.causes.contains(&"ADDRESS_FORBIDDEN".to_owned()),
        "unexpected causes: {:?}",
        module_err.causes
    );
}

#[tokio::test]
async fn test_ipv6_global_literal_is_pinned() {
    common::tests::setup();

    let (user_vm, ctx_part) = make_pin_vm().await;
    let (_, ctx_value) = user_vm.create_ctx(&ctx_part).unwrap();

    // A globally-routable IPv6 literal resolves to itself (no DNS) and must be
    // accepted and pinned back into the URL with its brackets intact.
    let res: anyhow::Result<String> = user_vm
        .call_fn(
            &user_vm.data,
            (ctx_value, "http://[2606:4700:4700::1111]:443/".to_owned()),
        )
        .await;

    let pinned = res.expect("expected global ipv6 literal to be accepted");
    let pinned = reqwest::Url::parse(&pinned).unwrap();
    assert_eq!(pinned.host_str(), Some("[2606:4700:4700::1111]"));
    assert_eq!(pinned.port(), Some(443));
}

#[tokio::test]
async fn test_ipv4_mapped_loopback_is_bad() {
    common::tests::setup();

    let (user_vm, ctx_part) = make_pin_vm().await;

    // Both the dotted-quad and hex spellings of the IPv4-mapped loopback
    // (`::ffff:127.0.0.1` == `::ffff:7f00:1`) must be rejected. `resolve_host`
    // normalizes via Rust's `SocketAddr`, so whatever spelling is fed in, the
    // classifier should see the same address and refuse it.
    for url in ["http://[::ffff:127.0.0.1]/", "http://[::ffff:7f00:1]/"] {
        let (_, ctx_value) = user_vm.create_ctx(&ctx_part).unwrap();
        let res: anyhow::Result<String> = user_vm
            .call_fn(&user_vm.data, (ctx_value, url.to_owned()))
            .await;

        let err = match res {
            Err(e) => e,
            Ok(pinned) => panic!("expected {url} to be rejected, got pinned url {pinned}"),
        };
        let module_err = scripting::try_unwrap_any_err(err).unwrap();
        assert!(
            module_err.causes.contains(&"ADDRESS_FORBIDDEN".to_owned()),
            "{url}: unexpected causes: {:?}",
            module_err.causes
        );
    }
}

#[tokio::test]
async fn test_ipv4_global_literal_with_explicit_port_is_pinned() {
    common::tests::setup();

    let (user_vm, ctx_part) = make_pin_vm().await;
    let (_, ctx_value) = user_vm.create_ctx(&ctx_part).unwrap();

    // Explicit port in the URL: `split_url` must hand the port to the Lua as an
    // integer, otherwise the `host:port` string becomes `1.1.1.1:443.0` and
    // resolution fails with "invalid port value".
    let res: anyhow::Result<String> = user_vm
        .call_fn(&user_vm.data, (ctx_value, "http://1.1.1.1:443/".to_owned()))
        .await;

    let pinned = res.expect("expected global ipv4 literal with port to be accepted");
    let pinned = reqwest::Url::parse(&pinned).unwrap();
    assert_eq!(pinned.host_str(), Some("1.1.1.1"));
    assert_eq!(pinned.port(), Some(443));
}
