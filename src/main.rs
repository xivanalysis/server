use std::{net::SocketAddr, str::FromStr};

use anyhow::Context;
use axum::{serve, Router};
use figment::{
	providers::{Env, Format, Toml},
	Figment,
};
use serde::{de, Deserialize, Deserializer};
use tokio::{net::TcpListener, signal};
use tokio_util::sync::CancellationToken;
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt, Layer};
use xiva_server::{asset, proxy, xivapi};

#[derive(Debug, Deserialize)]
struct Config {
	tracing: TracingConfig,
	http: HttpConfig,
	fflogs: proxy::Config,
	xivapi: xivapi::Config,
	client: asset::Config,
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

	let shutdown_token = shutdown_token();

	let filter = filter::Targets::new().with_default(config.tracing.level);
	let layer = tracing_subscriber::fmt::layer().with_filter(filter);
	tracing_subscriber::registry().with(layer).init();

	let router = Router::new()
		.nest(
			"/proxy",
			proxy::router(config.fflogs, shutdown_token.clone()),
		)
		.nest("/xivapi", xivapi::router(config.xivapi))
		.fallback(asset::method_router(config.client))
		.layer(CompressionLayer::new())
		// TODO: should probably limit the origins
		.layer(CorsLayer::permissive())
		.layer(TraceLayer::new_for_http());

	let app = router.into_make_service_with_connect_info::<SocketAddr>();

	let address = config.http.address;
	tracing::info!("http binding to {address:?}");
	let listener = TcpListener::bind(address)
		.await
		.context("failed to bind listener")?;
	serve(listener, app)
		.with_graceful_shutdown(shutdown_token.cancelled_owned())
		.await
		.unwrap();

	Ok(())
}

fn shutdown_token() -> CancellationToken {
	let token = CancellationToken::new();

	let inner_token = token.clone();
	tokio::spawn(async move {
		shutdown_signal().await;
		inner_token.cancel();
	});

	token
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
