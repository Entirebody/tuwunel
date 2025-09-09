use std::str;

use axum::{RequestExt, RequestPartsExt, extract::Path};
use bytes::Bytes;
use http::request::Parts;
use serde::Deserialize;
use tuwunel_core::{Result, err};
use tuwunel_service::Services;

#[derive(Deserialize)]
pub struct QueryParams {
	pub access_token: Option<String>,
	pub user_id: Option<String>,
}

pub struct Request {
	pub path: Path<Vec<String>>,
	pub query: QueryParams,
	pub body: Bytes,
	pub parts: Parts,
}

pub async fn from(
	services: &Services,
	request: hyper::Request<axum::body::Body>,
) -> Result<Request> {
	let limited = request.with_limited_body();
	let (mut parts, body) = limited.into_parts();

	let path: Path<Vec<String>> = parts.extract().await?;
	let query = parts.uri.query().unwrap_or_default();
	let query = serde_html_form::from_str(query)
		.map_err(|e| err!(Request(Unknown("Failed to read query parameters: {e}"))))?;

	let max_body_size = services.server.config.max_request_size;

	let body = axum::body::to_bytes(body, max_body_size)
		.await
		.map_err(|e| err!(Request(TooLarge("Request body too large: {e}"))))?;

	Ok(Request { path, query, body, parts })
}
