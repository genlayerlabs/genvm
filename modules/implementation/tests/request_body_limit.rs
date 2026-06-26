use std::collections::BTreeMap;

use genvm_common::*;
use genvm_modules_interfaces::web as web_iface;

use genvm_modules::common;

#[tokio::test]
async fn response_body_max_size_ok() {
    common::tests::setup();

    let client = common::tests::create_test_client();
    let metrics = sync::DArc::new(genvm_modules::scripting::Metrics::default());
    let url = "https://test-server.genlayer.com/body/echo";

    let body = vec![b'a'; 500];
    let req = genvm_modules::scripting::Request {
        url: url::Url::parse(url).unwrap(),
        method: web_iface::RequestMethod::POST,
        headers: BTreeMap::new(),
        body: Some(body.clone()),
        json: false,
        error_on_status: false,
        sign: false,
        response_body_max_size: Some(1024),
        timeout: None,
        unfiltered: false,
        headers_normalized: false,
    };

    let body_size_limit = req.response_body_max_size.unwrap_or(usize::MAX);
    let reqwst = req.into_reqwest(&client).unwrap();

    let res = genvm_modules::scripting::send_request_get_lua_compatible_response_bytes(
        &metrics,
        url,
        reqwst,
        false,
        body_size_limit,
    )
    .await
    .unwrap();

    assert_eq!(res.status, 200);
    assert_eq!(res.body, body);
}

#[tokio::test]
async fn response_body_max_size_exceeded() {
    common::tests::setup();

    let client = common::tests::create_test_client();
    let metrics = sync::DArc::new(genvm_modules::scripting::Metrics::default());
    let url = "https://test-server.genlayer.com/body/echo";

    let body = vec![b'b'; 2000];
    let req = genvm_modules::scripting::Request {
        url: url::Url::parse(url).unwrap(),
        method: web_iface::RequestMethod::POST,
        headers: BTreeMap::new(),
        body: Some(body),
        json: false,
        error_on_status: false,
        sign: false,
        response_body_max_size: Some(1024),
        timeout: None,
        unfiltered: false,
        headers_normalized: false,
    };

    let body_size_limit = 1024;
    let reqwst = req.into_reqwest(&client).unwrap();

    let err = genvm_modules::scripting::send_request_get_lua_compatible_response_bytes(
        &metrics,
        url,
        reqwst,
        false,
        body_size_limit,
    )
    .await
    .unwrap_err();

    let module_err = err
        .downcast::<genvm_modules::common::ModuleError>()
        .unwrap();
    assert!(module_err.causes.contains(&"READING_BODY".to_owned()));
}
