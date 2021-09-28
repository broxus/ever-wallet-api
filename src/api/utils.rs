use http::status::StatusCode;
use http::{HeaderValue, Response};
use serde::Deserialize;

#[derive(Debug)]
pub struct BadRequestError(pub String);

impl warp::reject::Reject for BadRequestError {}

#[allow(unused)]
pub async fn parse_body<T>(body: serde_json::Value) -> Result<T, warp::Rejection>
where
    T: for<'de> Deserialize<'de> + Send,
{
    serde_json::from_value::<T>(body).map_err(|e| {
        log::error!("error: {}", e);
        warp::reject::custom(BadRequestError(e.to_string()))
    })
}

pub fn bad_request(err: String) -> http::Response<hyper::Body> {
    let body = serde_json::json!({
        "description": "Bad request",
        "error": err
    })
    .to_string();
    let response = Response::new(body);
    let (mut parts, body) = response.into_parts();
    parts.status = StatusCode::BAD_REQUEST;
    parts
        .headers
        .insert("Content-Type", HeaderValue::from_static("application/json"));
    Response::from_parts(parts, body.into())
}

#[allow(dead_code)]
pub fn not_found_request() -> http::Response<hyper::Body> {
    let body = serde_json::json!({
    "description": "Not found",
    })
    .to_string();
    let response = Response::new(body);
    let (mut parts, body) = response.into_parts();
    parts.status = StatusCode::NOT_FOUND;
    parts
        .headers
        .insert("Content-Type", HeaderValue::from_static("application/json"));
    Response::from_parts(parts, body.into())
}
