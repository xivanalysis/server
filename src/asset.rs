use std::path::PathBuf;

use anyhow::{anyhow, Context};
use axum::{
	async_trait, debug_handler,
	extract::{FromRef, FromRequestParts, Host, Request, State},
	handler::Handler,
	http::{request::Parts, StatusCode},
	response::IntoResponse,
	routing::{get, MethodRouter},
	RequestPartsExt,
};
use figment::value::magic::RelativePathBuf;
use serde::Deserialize;
use tower::ServiceExt;
use tower_http::services::{ServeDir, ServeFile};

use super::error::Error;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
	public_path: RelativePathBuf,
	default_branch: String,
}

pub fn method_router(config: Config) -> MethodRouter {
	get(client).with_state(config)
}

#[debug_handler(state = Config)]
async fn client(AssetPath(asset_path): AssetPath, request: Request) -> impl IntoResponse {
	let service = ServeDir::new(&asset_path).fallback(fuck.with_state(asset_path));
	service.oneshot(request).await
}

#[debug_handler]
async fn fuck(
	State(asset_path): State<PathBuf>,
	request: Request,
) -> Result<impl IntoResponse, StatusCode> {
	// If the request is targeting the assets dir, hard fail it.
	if request.uri().path().starts_with("/assets") {
		return Err(StatusCode::NOT_FOUND);
	}

	// Otherwise, it might be a frontend route, respond with the client index
	let response = ServeFile::new(asset_path.join("index.html"))
		.oneshot(request)
		.await
		.expect("infallible");

	Ok(response)
}

struct AssetPath(PathBuf);

#[async_trait]
impl<S> FromRequestParts<S> for AssetPath
where
	S: Send + Sync,
	Config: FromRef<S>,
{
	type Rejection = Error;

	async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
		let config = Config::from_ref(state);
		let base_path = config.public_path.relative();

		// Client can control the host (because headers, because I can't get the
		// request target??) - ensure that we're staying inside the public path.
		let path_valid = |path: &PathBuf| path.starts_with(&base_path) && path.exists();

		let Host(host) = parts.extract().await.context("no host information")?;

		// Try using the first domain - this will fail for i.e. a subdomain-less url.
		if let Some((subdomain, _)) = host.split_once('.') {
			let path = base_path.join(subdomain);
			if path_valid(&path) {
				return Ok(Self(path));
			}
		}

		// Try with the default branch.
		let path = base_path.join(config.default_branch);
		if path_valid(&path) {
			return Ok(Self(path));
		}

		// Default also failed, fall back to a flat public path.
		if path_valid(&base_path) {
			return Ok(Self(base_path));
		}

		// Fallback also failed, error out.
		Err(anyhow!("cold not derive a valid base asset path"))?
	}
}
