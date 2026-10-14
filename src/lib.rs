//! The Echo Merchant: answers every request with a Mirror.

use axum::{
    body::Bytes,
    extract::{rejection::PathRejection, Path},
    http::{HeaderMap, Method, StatusCode, Uri},
    routing::any,
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Map, Value};

/// Prefix of the headers the connector adds to attribute a paid delivery.
const ATTRIBUTION_PREFIX: &str = "x-toon-";

/// The router: every path and method gets a Mirror; `/status/{code}` answers with status `code`.
pub fn router() -> Router {
    Router::new()
        .route("/status/{code}", any(status))
        .fallback(any(mirror))
}

/// Answer with the Mirror at status 200.
async fn mirror(method: Method, uri: Uri, headers: HeaderMap, body: Bytes) -> Json<Value> {
    Json(build_mirror(&method, &uri, &headers, &body))
}

/// Answer with the Mirror at the status the path names; 400 when the stack cannot send that status with a Mirror.
async fn status(
    code: Result<Path<String>, PathRejection>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> (StatusCode, Json<Value>) {
    let chosen = code
        .ok()
        .and_then(|Path(code)| sendable_status(&code))
        .unwrap_or(StatusCode::BAD_REQUEST);
    (chosen, Json(build_mirror(&method, &uri, &headers, &body)))
}

/// The status a decimal `code` from 100 to 599 names, if the stack can send it with a Mirror.
fn sendable_status(code: &str) -> Option<StatusCode> {
    if code.is_empty() || !code.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let status = StatusCode::from_u16(code.parse().ok()?).ok()?;
    // hyper refuses a 1xx final answer, and drops the body of a 204 or 304.
    let carries_mirror = (200..=599).contains(&status.as_u16())
        && status != StatusCode::NO_CONTENT
        && status != StatusCode::NOT_MODIFIED;
    carries_mirror.then_some(status)
}

/// Build the Mirror: the request as it arrived; `payment` is the connector's attribution, else `null`.
fn build_mirror(method: &Method, uri: &Uri, headers: &HeaderMap, body: &Bytes) -> Value {
    // A header sent more than once keeps every value, joined as HTTP allows.
    let mut header_map = Map::new();
    for name in headers.keys() {
        // Attribution headers belong under `payment`, not in the request as it arrived.
        if name.as_str().starts_with(ATTRIBUTION_PREFIX) {
            continue;
        }
        let values: Vec<_> = headers
            .get_all(name)
            .iter()
            .map(|value| String::from_utf8_lossy(value.as_bytes()).into_owned())
            .collect();
        header_map.insert(name.as_str().to_owned(), Value::String(values.join(", ")));
    }
    let path = uri
        .path_and_query()
        .map_or_else(|| uri.path().to_owned(), |pq| pq.as_str().to_owned());
    let attribution = |field: &str| {
        headers
            .get(format!("{ATTRIBUTION_PREFIX}{field}"))
            .map(|value| String::from_utf8_lossy(value.as_bytes()).into_owned())
    };
    // The connector sends all three attribution headers or none; anything less is not a payment.
    let payment = match (
        attribution("payer"),
        attribution("amount"),
        attribution("chain"),
    ) {
        (Some(payer), Some(amount), Some(chain)) => {
            json!({ "payer": payer, "amount": amount, "chain": chain })
        }
        _ => Value::Null,
    };
    // Valid UTF-8 comes back as text; anything else as base64, so no byte is lost.
    let (body, body_encoding) = match std::str::from_utf8(body) {
        Ok(text) => (text.to_owned(), "utf8"),
        Err(_) => (STANDARD.encode(body), "base64"),
    };
    json!({
        "request": {
            "method": method.as_str(),
            "path": path,
            "headers": header_map,
            "body": body,
            "body_encoding": body_encoding,
        },
        "payment": payment,
    })
}
