use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

async fn mirror_of(method: &str, uri: &str, body: &str) -> Value {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("x-probe", "hello")
        .body(Body::from(body.to_owned()))
        .unwrap();
    let response = echo_merchant::router().oneshot(request).await.unwrap();
    assert_eq!(response.status(), 200);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn mirrors_each_method_path_headers_and_body() {
    for (method, uri, body) in [
        ("GET", "/", ""),
        ("GET", "/a/b?x=1&y=2", ""),
        ("POST", "/pay/here", "héllo"),
        ("PUT", "/thing?q=z", "{\"k\":1}"),
        ("DELETE", "/gone", ""),
    ] {
        let mirror = mirror_of(method, uri, body).await;
        let request = &mirror["request"];
        assert_eq!(request["method"], method);
        assert_eq!(request["path"], uri);
        assert_eq!(request["headers"]["x-probe"], "hello");
        assert_eq!(request["body"], body);
        assert_eq!(request["body_encoding"], "utf8");
    }
}

#[tokio::test]
async fn payment_is_null_without_attribution() {
    let mirror = mirror_of("POST", "/x", "data").await;
    assert!(mirror["payment"].is_null());
    assert!(mirror.as_object().unwrap().contains_key("payment"));
}

#[tokio::test]
async fn repeated_header_keeps_every_value() {
    let request = Request::builder()
        .uri("/")
        .header("accept", "text/plain")
        .header("accept", "application/json")
        .body(Body::empty())
        .unwrap();
    let response = echo_merchant::router().oneshot(request).await.unwrap();
    assert_eq!(response.status(), 200);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let mirror: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        mirror["request"]["headers"]["accept"],
        "text/plain, application/json"
    );
}

async fn mirror_with(headers: &[(&str, &str)]) -> Value {
    let mut builder = Request::builder().uri("/");
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let response = echo_merchant::router()
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn assert_no_attribution_headers(mirror: &Value) {
    assert!(mirror["request"]["headers"]
        .as_object()
        .unwrap()
        .keys()
        .all(|name| !name.to_lowercase().starts_with("x-toon-")));
}

#[tokio::test]
async fn all_three_attribution_headers_give_payment() {
    let mirror = mirror_with(&[
        ("X-TOON-Payer", "evm:0xabc"),
        ("x-toon-amount", "123456789012345678901234567890"),
        ("X-Toon-Chain", "evm:31337"),
    ])
    .await;
    assert_eq!(mirror["payment"]["payer"], "evm:0xabc");
    assert!(mirror["payment"]["amount"].is_string());
    assert_eq!(
        mirror["payment"]["amount"],
        "123456789012345678901234567890"
    );
    assert_eq!(mirror["payment"]["chain"], "evm:31337");
    assert_no_attribution_headers(&mirror);
}

#[tokio::test]
async fn no_attribution_gives_null_payment() {
    assert!(mirror_with(&[("x-probe", "1")]).await["payment"].is_null());
}

#[tokio::test]
async fn partial_attribution_gives_null_payment_and_hides_headers() {
    for headers in [
        &[("X-TOON-Payer", "evm:0xabc"), ("x-toon-amount", "5")][..],
        &[
            ("x-toon-payer", "solana:abc"),
            ("X-Toon-Chain", "solana:devnet"),
        ][..],
        &[("X-TOON-AMOUNT", "5")][..],
    ] {
        let mirror = mirror_with(headers).await;
        assert!(mirror["payment"].is_null());
        assert_no_attribution_headers(&mirror);
    }
}

async fn mirror_of_bytes(body: Vec<u8>) -> Value {
    let request = Request::builder()
        .method("POST")
        .uri("/")
        .body(Body::from(body))
        .unwrap();
    let response = echo_merchant::router().oneshot(request).await.unwrap();
    assert_eq!(response.status(), 200);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn non_utf8_body_comes_back_as_base64_of_exact_bytes() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let sent = vec![0xff, 0xfe, 0x00, 0x80, 0xc0, 0x41];
    let mirror = mirror_of_bytes(sent.clone()).await;
    let request = &mirror["request"];
    assert_eq!(request["body_encoding"], "base64");
    let decoded = STANDARD.decode(request["body"].as_str().unwrap()).unwrap();
    assert_eq!(decoded, sent);
}

#[tokio::test]
async fn multibyte_utf8_body_comes_back_as_text() {
    let mirror = mirror_of_bytes("héllo 世界 🦀".as_bytes().to_vec()).await;
    assert_eq!(mirror["request"]["body"], "héllo 世界 🦀");
    assert_eq!(mirror["request"]["body_encoding"], "utf8");
}

#[tokio::test]
async fn empty_body_comes_back_as_empty_text() {
    let mirror = mirror_of_bytes(Vec::new()).await;
    assert_eq!(mirror["request"]["body"], "");
    assert_eq!(mirror["request"]["body_encoding"], "utf8");
}
