use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use serde_json::Value;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
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

/// Send one request to the router served on a real socket, so hyper encodes the answer.
async fn status_and_mirror(method: &str, uri: &str, headers: &[(&str, &str)]) -> (u16, Value) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, echo_merchant::router()).await });
    let mut request = format!(
        "{method} {uri} HTTP/1.1\r\nhost: {addr}\r\nconnection: close\r\ncontent-length: 0\r\n"
    );
    for (name, value) in headers {
        request += &format!("{name}: {value}\r\n");
    }
    request += "\r\n";
    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    let (head, body) = response.split_once("\r\n\r\n").unwrap();
    let status = head.split(' ').nth(1).unwrap().parse().unwrap();
    (status, serde_json::from_str(body).unwrap())
}

#[tokio::test]
async fn status_answers_each_sendable_code_with_a_mirror() {
    for code in (200..=599u16).filter(|code| ![204, 304].contains(code)) {
        let uri = format!("/status/{code}");
        let (status, mirror) = status_and_mirror("GET", &uri, &[]).await;
        assert_eq!(status, code);
        assert_eq!(mirror["request"]["path"], uri);
    }
}

#[tokio::test]
async fn status_rejects_bad_and_unsendable_codes_with_a_mirror() {
    for uri in [
        "/status/abc",
        "/status/99",
        "/status/600",
        "/status/%FF",
        "/status/+503",
        "/status/-1",
        "/status/99999999",
        "/status/100",
        "/status/101",
        "/status/199",
        "/status/204",
        "/status/304",
    ] {
        let (status, mirror) = status_and_mirror("GET", uri, &[]).await;
        assert_eq!(status, 400, "{uri}");
        assert_eq!(mirror["request"]["method"], "GET", "{uri}");
        assert_eq!(mirror["request"]["path"], uri, "{uri}");
    }
}

#[tokio::test]
async fn status_works_for_any_method() {
    for method in ["GET", "POST", "PUT", "DELETE", "PATCH"] {
        let (status, mirror) = status_and_mirror(method, "/status/503", &[]).await;
        assert_eq!(status, 503);
        assert_eq!(mirror["request"]["method"], method);
    }
}

#[tokio::test]
async fn status_shows_payment_like_any_other_path() {
    let (_, mirror) = status_and_mirror(
        "GET",
        "/status/503",
        &[
            ("X-TOON-Payer", "evm:0xabc"),
            ("x-toon-amount", "42"),
            ("X-Toon-Chain", "evm:31337"),
        ],
    )
    .await;
    assert_eq!(mirror["payment"]["payer"], "evm:0xabc");
    assert_eq!(mirror["payment"]["amount"], "42");
    assert_eq!(mirror["payment"]["chain"], "evm:31337");
    assert_no_attribution_headers(&mirror);
}

#[tokio::test]
async fn paths_beside_status_code_still_answer_200() {
    for uri in ["/status", "/status/", "/status/503/x", "/other"] {
        let (status, _) = status_and_mirror("GET", uri, &[]).await;
        assert_eq!(status, 200, "{uri}");
    }
}
