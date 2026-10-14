//! The Echo Merchant: answers every request with a Mirror.

use axum::{
    body::Bytes,
    http::{HeaderMap, Method, Uri},
    routing::any,
    Json, Router,
};
use serde_json::{json, Map, Value};

/// The router: every path and method gets a Mirror.
pub fn router() -> Router {
    Router::new().route("/", any(mirror)).fallback(any(mirror))
}

/// Build the Mirror: the request as it arrived; `payment` is `null` until pricing exists.
async fn mirror(method: Method, uri: Uri, headers: HeaderMap, body: Bytes) -> Json<Value> {
    let mut header_map = Map::new();
    for (name, value) in &headers {
        header_map.insert(
            name.as_str().to_owned(),
            Value::String(String::from_utf8_lossy(value.as_bytes()).into_owned()),
        );
    }
    let path = uri
        .path_and_query()
        .map_or_else(|| uri.path().to_owned(), |pq| pq.as_str().to_owned());
    Json(json!({
        "request": {
            "method": method.as_str(),
            "path": path,
            "headers": header_map,
            "body": String::from_utf8_lossy(&body),
            "body_encoding": "utf8",
        },
        "payment": null,
    }))
}
