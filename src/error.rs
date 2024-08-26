use axum::{
	http::StatusCode,
	response::{IntoResponse, Response},
};

// Don't expose the inner error, it'll probably contain api keys and shit.
#[derive(Debug, thiserror::Error)]
#[error("an error occured")]
pub struct Error {
	#[from]
	inner: anyhow::Error,
}

impl IntoResponse for Error {
	fn into_response(self) -> Response {
		tracing::error!("{}", self.inner);

		(StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
	}
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
