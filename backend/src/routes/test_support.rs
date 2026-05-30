use axum::{
    body::Body,
    http::{Method, Request, Response},
};
use http_body_util::BodyExt;
use serde_json::Value;

pub(super) fn empty_request(uri: impl AsRef<str>) -> Request<Body> {
    Request::builder()
        .uri(uri.as_ref())
        .body(Body::empty())
        .unwrap()
}

pub(super) fn json_request(
    method: Method,
    uri: impl AsRef<str>,
    body: impl Into<Body>,
) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri.as_ref())
        .header("content-type", "application/json")
        .body(body.into())
        .unwrap()
}

pub(super) async fn json_body(response: Response<Body>) -> Value {
    let body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}
