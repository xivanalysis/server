use std::{collections::HashMap, sync::Arc, time::Duration};

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
use tokio_util::sync::CancellationToken;
use tower_governor::{
	governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer,
};

use super::error::Result;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
	url: String,
	key: String,

	limit: LimitConfig,
}

#[derive(Debug, Deserialize, Clone)]
struct LimitConfig {
	burst: u32,
	recover_secs: u64,
}

pub fn router(config: Config, cancel: CancellationToken) -> Router {
	// Set up the rate limiting governor.
	let governor_config = GovernorConfigBuilder::default()
		.key_extractor(SmartIpKeyExtractor)
		.period(Duration::from_secs(config.limit.recover_secs))
		.burst_size(config.limit.burst)
		.finish()
		.expect("governor config should be valid");

	let governor_config = Arc::new(governor_config);

	// Perform regular maintenance on the governor.
	let governor_limiter = governor_config.limiter().clone();
	tokio::spawn(async move {
		loop {
			tokio::select! {
				_ = cancel.cancelled() => {
					tracing::debug!("governor maintenance cancelled");
					break;
				}
				_ = tokio::time::sleep(Duration::from_secs(60)) => {
					let live = governor_limiter.len();
					tracing::debug!(live, "governor maintenance");
					governor_limiter.retain_recent();
				}
			}
		}
	});

	let state = FflogsState {
		client: reqwest::Client::new(),
		config,
	};

	Router::new()
		.route("/fflogs/*path", get(fflogs))
		.layer(GovernorLayer {
			config: governor_config,
		})
		.with_state(state)
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
