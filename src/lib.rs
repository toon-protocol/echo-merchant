//! The Echo Merchant: answers every request with a Mirror.

use axum::{
    body::Bytes,
    http::{HeaderMap, Method, Uri},
    routing::any,
    Json, Router,
};
use serde_json::{json, Map, Value};

/// Prefix of the headers the connector adds to attribute a paid delivery.
const ATTRIBUTION_PREFIX: &str = "x-toon-";

/// The router: every path and method gets a Mirror.
pub fn router() -> Router {
    Router::new().fallback(any(mirror))
}

/// Build the Mirror: the request as it arrived; `payment` is the connector's attribution, else `null`.
async fn mirror(method: Method, uri: Uri, headers: HeaderMap, body: Bytes) -> Json<Value> {
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
    Json(json!({
        "request": {
            "method": method.as_str(),
            "path": path,
            "headers": header_map,
            "body": String::from_utf8_lossy(&body),
            "body_encoding": "utf8",
        },
        "payment": payment,
    }))
}
