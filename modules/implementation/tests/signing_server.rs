use std::collections::BTreeMap;
use std::sync::Arc;

use genvm_common::*;
use genvm_modules_interfaces::web as web_iface;

use genvm_modules::common;
use genvm_modules::scripting::CtxPart;

#[tokio::test]
async fn signing_post_with_server() {
    common::tests::setup();

    let mut req = genvm_modules::scripting::Request {
        url: url::Url::parse("https://test-server.genlayer.com/body/echo-signed").unwrap(),
        method: web_iface::RequestMethod::POST,
        headers: BTreeMap::new(),
        body: Some(b"test body".to_vec()),
        json: false,
        error_on_status: true,
        sign: true,
        response_body_max_size: None,
        timeout: None,
        unfiltered: false,
        headers_normalized: false,
    };

    let part = CtxPart {
        hello: Arc::new(genvm_modules_interfaces::GenVMHello {
            genvm_id: genvm_modules_interfaces::GenVMId(999),
            role: genvm_modules_interfaces::Role::Leader,
            host_data: genvm_modules_interfaces::HostData {
                node_address: "test_address".to_string(),
                tx_id: "test_tx_id".to_string(),
                rest: serde_json::Map::new(),
            },
            gas_data: std::collections::BTreeMap::new(),
            initial_time_units_allocation: 0,
        }),
        client: common::tests::create_test_client(),
        client_unfiltered: common::tests::create_test_client(),
        sign_url: Arc::from("https://test-server.genlayer.com/genvm/sign"),
        sign_headers: Arc::new(BTreeMap::new()),
        sign_vars: BTreeMap::new(),
        node_address: "node_address".to_string(),
        metrics: sync::DArc::new(genvm_modules::scripting::Metrics::default()),
    };

    req.add_rfc9421_sign_headers(&part).await.unwrap();

    let reqwst = req.into_reqwest(&part.client).unwrap();

    let res = reqwst.send().await.unwrap();
    let status = res.status();
    let body = res.bytes().await;
    assert_eq!(status, 200);

    let body = body.unwrap();
    assert_eq!(body.as_ref(), b"test body");
}
