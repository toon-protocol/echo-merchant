#[tokio::main]
async fn main() {
    let addr = std::env::var("ECHO_MERCHANT_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".into());
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("bind listener");
    axum::serve(listener, echo_merchant::router())
        .await
        .expect("serve");
}
