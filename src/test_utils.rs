//! Helpers for driving a layer with synthetic requests, for this crate's own tests and,
//! behind `feature = "test-utils"`, for downstream ones.
//!
//! The surface stops at "build a request, push it through the layer, read the
//! response". Anything that needs a router or a real socket is an integration test.

#![cfg(any(test, feature = "test-utils"))]

use std::convert::Infallible;
use std::future::{Ready, ready};
use std::hash::Hash;
use std::net::SocketAddr;
use std::task::{Context, Poll};

use axum::body::Body;
use axum::extract::ConnectInfo;
use http::{Method, Request, Response, StatusCode};
use tower::ServiceExt as _;

pub use crate::MockClock;

/// Inner service that answers every request with an empty `200 OK`, so whatever comes
/// back from the layer is the layer's own doing.
#[derive(Clone, Copy, Debug, Default)]
pub struct OkService;

impl tower::Service<Request<Body>> for OkService {
	type Response = Response<Body>;
	type Error = Infallible;
	type Future = Ready<Result<Response<Body>, Infallible>>;

	fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, _req: Request<Body>) -> Self::Future {
		ready(Ok(Response::new(Body::empty())))
	}
}

/// A request with an empty body and no extensions.
pub fn request(method: Method, path: &str) -> Request<Body> {
	Request::builder().method(method).uri(path).body(Body::empty()).expect("valid synthetic request")
}

/// [`request`] plus the `ConnectInfo<SocketAddr>` extension the address extractors read.
pub fn request_with_peer(method: Method, path: &str, peer: SocketAddr) -> Request<Body> {
	let mut req = request(method, path);
	req.extensions_mut().insert(ConnectInfo::<SocketAddr>(peer));
	req
}

/// Push one request through `layer` over [`OkService`] and return the full response.
/// Works for `GovernorLayer<K>` and `BoxedGovernorLayer` alike.
pub async fn drive_response<L>(layer: &L, req: Request<Body>) -> Response<Body>
where
	L: tower::Layer<OkService>,
	L::Service: tower::Service<Request<Body>, Response = Response<Body>, Error = Infallible>,
{
	tower::Layer::layer(layer, OkService).oneshot(req).await.expect("infallible inner")
}

/// Status-only form of [`drive_response`] for the common admit-or-reject assertion.
pub async fn drive<K>(
	layer: &crate::GovernorLayer<K>,
	method: Method,
	path: &str,
	peer: Option<SocketAddr>,
) -> StatusCode
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	drive_response(layer, with_optional_peer(method, path, peer)).await.status()
}

/// Same as [`drive`] but for `BoxedGovernorLayer`.
pub async fn drive_boxed(
	layer: &crate::BoxedGovernorLayer,
	method: Method,
	path: &str,
	peer: Option<SocketAddr>,
) -> StatusCode {
	drive_response(layer, with_optional_peer(method, path, peer)).await.status()
}

fn with_optional_peer(method: Method, path: &str, peer: Option<SocketAddr>) -> Request<Body> {
	match peer {
		Some(peer) => request_with_peer(method, path, peer),
		None => request(method, path),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::builder::GovernorConfigBuilder;
	use crate::extractor::Global;
	use crate::layer::GovernorLayer;
	use crate::{Quota, nz};

	#[tokio::test]
	async fn drive_admits_first_rejects_second() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quota_default(Quota::requests_per_second(nz!(1u32)))
			.finish()
			.unwrap();
		let layer: GovernorLayer<()> = GovernorLayer::new(cfg);
		assert_eq!(drive(&layer, Method::GET, "/", None).await, StatusCode::OK);
		assert_eq!(drive(&layer, Method::GET, "/", None).await, StatusCode::TOO_MANY_REQUESTS);
	}

	#[tokio::test]
	async fn drive_response_exposes_headers() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quota_default(Quota::requests_per_second(nz!(5u32)))
			.finish()
			.unwrap();
		let layer: GovernorLayer<()> = GovernorLayer::new(cfg);
		let resp = drive_response(&layer, request(Method::GET, "/")).await;
		assert_eq!(resp.headers()["x-ratelimit-limit"], "5");
	}

	#[test]
	fn request_with_peer_carries_connect_info() {
		let peer: SocketAddr = "1.2.3.4:5".parse().unwrap();
		let req = request_with_peer(Method::GET, "/", peer);
		assert_eq!(req.extensions().get::<ConnectInfo<SocketAddr>>().unwrap().0, peer);
	}

	#[tokio::test]
	async fn mock_clock_alias_resolves() {
		let _: MockClock = MockClock::default();
	}
}
