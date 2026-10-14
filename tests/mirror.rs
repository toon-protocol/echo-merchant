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
async fn payment_is_null() {
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
    for (k, v) in headers {
        builder = builder.header(*k, *v);
    }
    let response = echo_merchant::router()
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
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
    assert_eq!(
        mirror["payment"]["amount"],
        "123456789012345678901234567890"
    );
    assert_eq!(mirror["payment"]["chain"], "evm:31337");
    assert!(mirror["request"]["headers"]
        .as_object()
        .unwrap()
        .keys()
        .all(|k| !k.to_lowercase().starts_with("x-toon-")));
}

#[tokio::test]
async fn no_attribution_gives_null_payment() {
    assert!(mirror_with(&[("x-probe", "1")]).await["payment"].is_null());
}

#[tokio::test]
async fn partial_attribution_gives_null_payment_and_hides_headers() {
    let mirror = mirror_with(&[("x-toon-payer", "evm:0xabc"), ("x-toon-amount", "5")]).await;
    assert!(mirror["payment"].is_null());
    assert!(mirror["request"]["headers"]
        .as_object()
        .unwrap()
        .keys()
        .all(|k| !k.starts_with("x-toon-")));
}
