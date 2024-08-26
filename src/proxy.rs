use std::collections::HashMap;

use anyhow::Context;
use axum::{
	body::Body,
	debug_handler,
	extract::{Path, Query, State},
	response::{IntoResponse, Response},
	routing::get,
	Router,
};
use serde::Deserialize;
use tower_http::compression::CompressionLayer;

use super::error::Result;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
	url: String,
	key: String,
}

pub fn router(config: Config) -> Router {
	let state = FflogsState {
		client: reqwest::Client::new(),
		config,
	};

	Router::new()
		.route("/fflogs/*path", get(fflogs))
		.with_state(state)
		.layer(CompressionLayer::new())
}

#[derive(Debug, Deserialize)]
struct FflogsPath {
	path: String,
}

#[derive(Debug, Deserialize)]
struct FflogsQuery {
	#[serde(flatten)]
	rest: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct FflogsState {
	client: reqwest::Client,
	config: Config,
}

#[debug_handler]
async fn fflogs(
	Path(FflogsPath { path }): Path<FflogsPath>,
	Query(query): Query<FflogsQuery>,
	State(client): State<FflogsState>,
) -> Result<impl IntoResponse> {
	let upstream_url = format!("{}{path}", client.config.url);

	let reqwest_response = client
		.client
		.get(upstream_url)
		.query(&query.rest)
		.query(&[("api_key", &client.config.key)])
		.send()
		.await
		.map_err(|err| err.without_url())
		.context("request failed")?;

	let mut response_builder = Response::builder().status(reqwest_response.status());
	*response_builder.headers_mut().unwrap() = reqwest_response.headers().clone();
	let response = response_builder
		.body(Body::from_stream(reqwest_response.bytes_stream()))
		.unwrap();

	Ok(response)
}
