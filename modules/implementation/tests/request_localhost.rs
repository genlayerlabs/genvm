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

#[tokio::test]
async fn test_filtering_resolver_rejects_loopback_host() {
    common::tests::setup();

    // `localhost.direct` is a public DNS name that resolves only to loopback, so
    // the filtering client must drop every address. The failure must surface as
    // a non-fatal `ADDRESS_FORBIDDEN` module error so the contract can recover.
    let client = common::create_client_filtered().unwrap();

    let url = "http://localhost.direct/".to_owned();
    let metrics = sync::DArc::new(Metrics::default());

    let res = send_request_get_lua_compatible_response_bytes(
        &metrics,
        &url,
        client.get(&url),
        true,
        usize::MAX,
    )
    .await;

    let err = res.expect_err("expected loopback-only host to be rejected by the resolver");
    log_info!(error:ah = &err; "filtering resolver rejected localhost.direct");

    let module_err = scripting::try_unwrap_any_err(err).expect("expected a module error");
    assert!(
        module_err.causes.contains(&"ADDRESS_FORBIDDEN".to_owned()),
        "unexpected causes: {:?}",
        module_err.causes
    );
    assert!(!module_err.fatal, "ADDRESS_FORBIDDEN must be non-fatal");
}

#[tokio::test]
async fn test_unfiltered_client_allows_loopback() {
    common::tests::setup();

    let body = b"hello from allowlisted host";
    let addr = serve_once(body).await;

    // The same loopback-only host succeeds through the unfiltered client (the one
    // used for `always_allow_hosts`): no address filtering, and the hostname
    // stays in the URL the whole time.
    let client = common::create_client_unfiltered().unwrap();

    let url = format!("http://localhost.direct:{}/", addr.port());
    let metrics = sync::DArc::new(Metrics::default());

    let res = send_request_get_lua_compatible_response_bytes(
        &metrics,
        &url,
        client.get(&url),
        true,
        usize::MAX,
    )
    .await
    .expect("expected the unfiltered client to reach the loopback host");

    assert_eq!(res.status, 200);
    assert_eq!(res.body, body);
}
