use std::{collections::HashMap, net::SocketAddr, str::FromStr};

use anyhow::Context;
use axum::{
	body::Body,
	debug_handler,
	extract::{Path, Query, State},
	response::{IntoResponse, Redirect},
	routing::get,
	serve, Router,
};
use figment::{
	providers::{Env, Format, Toml},
	Figment,
};
use reqwest::Client;
use serde::{de, Deserialize, Deserializer};
use tokio::{net::TcpListener, signal};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt, Layer};

#[derive(Debug, Deserialize)]
struct Config {
	tracing: TracingConfig,
	http: HttpConfig,
	fflogs: FflogsConfig,
	xivapi: XivapiConfig,
}

#[derive(Debug, Deserialize)]
struct TracingConfig {
	#[serde(deserialize_with = "deserialize_level")]
	level: LevelFilter,
}

fn deserialize_level<'de, D>(deserializer: D) -> Result<LevelFilter, D::Error>
where
	D: Deserializer<'de>,
{
	let string = String::deserialize(deserializer)?;
	LevelFilter::from_str(&string).map_err(de::Error::custom)
}

#[derive(Debug, Deserialize)]
struct HttpConfig {
	address: SocketAddr,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let figment = Figment::new()
		.merge(Toml::file("xiva.toml"))
		.merge(Env::prefixed("XIVA_").split("_"));

	let config = figment
		.extract::<Config>()
		.context("failed to extract config")?;

	let filter = filter::Targets::new().with_default(config.tracing.level);
	let layer = tracing_subscriber::fmt::layer().with_filter(filter);
	tracing_subscriber::registry().with(layer).init();

	let client = Client::new();
	let fflogs_client = FflogsClient {
		client: client.clone(),
		config: config.fflogs,
	};
	let xivapi_client = XivapiClient {
		client,
		config: config.xivapi,
	};

	let router = Router::new()
		.route(
			"/proxy/fflogs/*path",
			get(proxy_fflogs).with_state(fflogs_client),
		)
		.route(
			"/xivapi/zone-banner/:zone_id",
			get(xivapi_zone_banner).with_state(xivapi_client),
		)
		// TODO: should probably limit the origins
		.layer(CorsLayer::permissive())
		.layer(TraceLayer::new_for_http());

	let address = config.http.address;
	tracing::info!("http binding to {address:?}");
	let listener = TcpListener::bind(address)
		.await
		.context("failed to bind listener")?;
	serve(listener, router)
		.with_graceful_shutdown(shutdown_signal())
		.await
		.unwrap();

	Ok(())
}

async fn shutdown_signal() {
	let ctrl_c = async {
		signal::ctrl_c()
			.await
			.expect("Failed to install Ctrl+C handler.");
	};

	#[cfg(unix)]
	let terminate = async {
		signal::unix::signal(signal::unix::SignalKind::terminate())
			.expect("Failed to install SIGTERM handler.")
			.recv()
			.await;
	};

	#[cfg(not(unix))]
	let terminate = std::future::pending::<()>();

	tokio::select! {
		_ = ctrl_c => {},
		_ = terminate => {},
	}

	tracing::info!("shutdown signal received")
}

#[derive(Debug, Deserialize, Clone)]
struct FflogsConfig {
	url: String,
	key: String,
}

#[derive(Debug, Deserialize)]
struct ProxyFflogsPath {
	path: String,
}

#[derive(Debug, Deserialize)]
struct ProxyFFlogsQuery {
	#[serde(flatten)]
	rest: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct FflogsClient {
	client: Client,
	config: FflogsConfig,
}

#[debug_handler]
async fn proxy_fflogs(
	Path(ProxyFflogsPath { path }): Path<ProxyFflogsPath>,
	Query(query): Query<ProxyFFlogsQuery>,
	State(client): State<FflogsClient>,
) -> impl IntoResponse {
	let upstream_url = format!("{}{path}", client.config.url);

	let response = client
		.client
		.get(upstream_url)
		.query(&query.rest)
		.query(&[("api_key", &client.config.key)])
		.send()
		.await
		.map_err(|err| err.without_url())
		.expect("TODO");

	// TODO: echo the response code &c
	Body::from_stream(response.bytes_stream())
}

#[derive(Debug, Deserialize, Clone)]
struct XivapiConfig {
	url: String,
}

#[derive(Debug, Deserialize)]
struct XivapiZoneBannerPath {
	zone_id: u32,
}

#[derive(Debug, Clone)]
struct XivapiClient {
	client: Client,
	config: XivapiConfig,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum XivapiResponse<T> {
	Error(XivapiError),
	Success(T),
}

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

#[debug_handler]
async fn xivapi_zone_banner(
	Path(path): Path<XivapiZoneBannerPath>,
	State(client): State<XivapiClient>,
) -> impl IntoResponse {
	// TODO: cache

	let upstream_url = format!("{}sheet/TerritoryType/{}", client.config.url, path.zone_id);
	let response = client
		.client
		.get(upstream_url)
		.query(&[
			("fields", "ContentFinderCondition.Image"),
			("transient", ""),
		])
		.send()
		.await
		.expect("TODO");

	type ZoneBannerResponse = XivapiResponse<XivapiSheet<XivapiZoneBannerFields>>;
	let xivapi_response = match response.json::<ZoneBannerResponse>().await.expect("TODO") {
		XivapiResponse::Error(error) => todo!("xivapi error {error:?}"),
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
	Redirect::temporary(&target_url)
}
