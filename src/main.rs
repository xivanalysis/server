use axum::{debug_handler, response::IntoResponse, routing::get, serve, Router};
use tokio::{net::TcpListener, signal};
use tower_http::trace::TraceLayer;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt, Layer};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let filter = filter::Targets::new().with_default(LevelFilter::DEBUG);
	let layer = tracing_subscriber::fmt::layer().with_filter(filter);
	tracing_subscriber::registry().with(layer).init();

	let router = Router::new()
		.route("/", get(root))
		.layer(TraceLayer::new_for_http());

	let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
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

#[debug_handler]
async fn root() -> impl IntoResponse {
	"scaffold"
}
