use std::{
	collections::HashMap,
	sync::{Arc, RwLock},
};

use anyhow::{anyhow, Context};
use axum::{
	debug_handler,
	extract::{Path, State},
	response::{IntoResponse, Redirect},
	routing::get,
	Router,
};
use serde::Deserialize;

use super::error::Result;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
	url: String,
}

#[derive(Debug, Clone)]
struct XivapiState {
	client: reqwest::Client,
	config: Config,
	cache: Arc<RwLock<HashMap<u32, String>>>,
}

pub fn router(config: Config) -> Router {
	let state = XivapiState {
		client: reqwest::Client::new(),
		config,
		cache: Default::default(),
	};

	Router::new()
		.route("/zone-banner/:zone_id", get(zone_banner))
		.with_state(state)
}

#[derive(Debug, Deserialize)]
struct ZoneBannerPath {
	zone_id: u32,
}

#[debug_handler]
async fn zone_banner(
	Path(ZoneBannerPath { zone_id }): Path<ZoneBannerPath>,
	State(client): State<XivapiState>,
) -> Result<impl IntoResponse> {
	if let Some(url) = client.cache.read().expect("poisoned").get(&zone_id) {
		return Ok(Redirect::temporary(url));
	}

	let upstream_url = format!("{}sheet/TerritoryType/{}", client.config.url, zone_id);
	let response = client
		.client
		.get(upstream_url)
		.query(&[
			("fields", "ContentFinderCondition.Image"),
			("transient", ""),
		])
		.send()
		.await
		.context("request failed")?;

	type ZoneBannerResponse = XivapiResponse<XivapiSheet<XivapiZoneBannerFields>>;
	let xivapi_response = match response
		.json::<ZoneBannerResponse>()
		.await
		.context("invalid response")?
	{
		XivapiResponse::Error(error) => Err(anyhow!("xivapi error {error:?}"))?,
		XivapiResponse::Success(value) => value,
	};

	let image_path = xivapi_response
		.row
		.fields
		.content_finder_condition
		.row
		.fields
		.image
		.path_hr1;

	let target_url = format!("{}asset/{image_path}?format=png", client.config.url);

	client
		.cache
		.write()
		.expect("poisoned")
		.insert(zone_id, target_url.clone());

	Ok(Redirect::temporary(&target_url))
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum XivapiResponse<T> {
	Error(XivapiError),
	Success(T),
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct XivapiError {
	code: u16,
	message: String,
}

#[derive(Debug, Deserialize)]
struct XivapiSheet<F> {
	// schema: String,
	#[serde(flatten)]
	row: XivapiSheetRow<F>,
}

#[derive(Debug, Deserialize)]
struct XivapiRelationship<F> {
	// value: u32,
	// sheet: String,
	#[serde(flatten)]
	row: XivapiSheetRow<F>,
}

#[derive(Debug, Deserialize)]
struct XivapiSheetRow<F> {
	// row_id: u32,
	fields: F,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XivapiZoneBannerFields {
	content_finder_condition: XivapiRelationship<XivapiZoneBannerCFCFields>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XivapiZoneBannerCFCFields {
	image: XivapiImage,
}

#[derive(Debug, Deserialize)]
struct XivapiImage {
	// id: u32,
	// path: String,
	path_hr1: String,
}
