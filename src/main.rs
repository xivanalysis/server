use axum::{debug_handler, response::IntoResponse, routing::get, serve, Router};
use tokio::{net::TcpListener, signal};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let router = Router::new().route("/", get(root));

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
}

#[debug_handler]
async fn root() -> impl IntoResponse {
	"scaffold"
}
