//! Helpers for downstream tests of the Governor layer.
//!
//! Gated behind `feature = "test-utils"` and `cfg(test)` of this crate.

#![cfg(any(test, feature = "test-utils"))]

use std::convert::Infallible;
use std::hash::Hash;
use std::net::SocketAddr;

use axum::extract::ConnectInfo;
use http::{Method, Request, Response, StatusCode};
use tower::{Layer as _, ServiceExt as _};

pub use crate::MockClock;

fn make_inner() -> impl tower::Service<
	Request<axum::body::Body>,
	Response = Response<axum::body::Body>,
	Error = Infallible,
	Future = impl std::future::Future<Output = Result<Response<axum::body::Body>, Infallible>> + Send,
> + Clone
+ Send
+ 'static {
	tower::service_fn(|_req: Request<axum::body::Body>| async {
		Ok::<_, Infallible>(Response::builder().status(200).body(axum::body::Body::empty()).unwrap())
	})
}

/// Drive a synthetic request through a `GovernorLayer` and return
/// the resulting status code. The inner service always responds 200 OK.
pub async fn drive<K>(
	layer: &crate::GovernorLayer<K>,
	method: Method,
	path: &str,
	peer: Option<SocketAddr>,
) -> StatusCode
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	let svc = layer.layer(make_inner());
	let mut req = Request::builder()
		.method(method)
		.uri(path)
		.body(axum::body::Body::empty())
		.expect("valid synthetic request");
	if let Some(peer) = peer {
		req.extensions_mut().insert(ConnectInfo::<SocketAddr>(peer));
	}
	svc.oneshot(req).await.expect("infallible inner").status()
}

/// Same as `drive` but for `BoxedGovernorLayer`.
pub async fn drive_boxed(
	layer: &crate::BoxedGovernorLayer,
	method: Method,
	path: &str,
	peer: Option<SocketAddr>,
) -> StatusCode {
	let svc = layer.layer(make_inner());
	let mut req = Request::builder()
		.method(method)
		.uri(path)
		.body(axum::body::Body::empty())
		.expect("valid synthetic request");
	if let Some(peer) = peer {
		req.extensions_mut().insert(ConnectInfo::<SocketAddr>(peer));
	}
	svc.oneshot(req).await.expect("infallible inner").status()
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
		let s1 = drive(&layer, Method::GET, "/", None).await;
		assert_eq!(s1, StatusCode::OK);
		let s2 = drive(&layer, Method::GET, "/", None).await;
		assert_eq!(s2, StatusCode::TOO_MANY_REQUESTS);
	}

	#[tokio::test]
	async fn mock_clock_alias_resolves() {
		let _: MockClock = MockClock::default();
	}
}
