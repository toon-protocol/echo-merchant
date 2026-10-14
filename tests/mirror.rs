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
