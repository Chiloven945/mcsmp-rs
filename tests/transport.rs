use mcsmp_rs::{Auth, Client, Error};
use url::Url;

#[tokio::test]
async fn transport_rejects_non_websocket_endpoints_before_connecting() {
    let error = Client::builder(Url::parse("http://127.0.0.1:25585").unwrap())
        .auth(Auth::none())
        .connect()
        .await
        .expect_err("HTTP endpoints must be rejected before a network attempt");
    assert!(matches!(error, Error::Configuration(_)));
}
