//! Type-erased `BoxedGovernorLayer` that hides the key-extractor type parameter.

use std::sync::Arc;

use crate::builder::{ExtractorSlot, GovernorConfig};
use crate::extractor::{AsyncExtractFuture, AsyncKeyExtractor, KeyExtractor, KeyOutcome};
use crate::layer::GovernorLayer;

// ---------------------------------------------------------------------------
// Sync adapter
// ---------------------------------------------------------------------------

struct StringKeyAdapter<K> {
	inner: Arc<dyn KeyExtractor<Key = K>>,
}

impl<K> KeyExtractor for StringKeyAdapter<K>
where
	K: std::hash::Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	type Key = String;

	fn extract(
		&self,
		parts: &http::request::Parts,
	) -> Result<KeyOutcome<String>, crate::ExtractionError> {
		let outcome = self.inner.extract(parts)?;
		Ok(KeyOutcome { key: format!("{:?}", outcome.key), quota_override: outcome.quota_override })
	}

	fn requires_connect_info(&self) -> bool {
		self.inner.requires_connect_info()
	}
}

// ---------------------------------------------------------------------------
// Async adapter
// ---------------------------------------------------------------------------

struct AsyncStringKeyAdapter<K> {
	inner: Arc<dyn AsyncKeyExtractor<Key = K>>,
}

impl<K> AsyncKeyExtractor for AsyncStringKeyAdapter<K>
where
	K: std::hash::Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	type Key = String;

	fn extract<'a>(&'a self, parts: &'a http::request::Parts) -> AsyncExtractFuture<'a, String> {
		let inner = Arc::clone(&self.inner);
		Box::pin(async move {
			let outcome = inner.extract(parts).await?;
			Ok(KeyOutcome { key: format!("{:?}", outcome.key), quota_override: outcome.quota_override })
		})
	}
}

// ---------------------------------------------------------------------------
// BoxedGovernorLayer
// ---------------------------------------------------------------------------

/// A type-erased wrapper around `GovernorLayer<String>`.
///
/// Erases the key-extractor type parameter `K` by mapping every key to its
/// `Debug` representation before hashing. Cost: one `String` allocation and
/// one extra hash per request. In exchange the concrete type carries no
/// generic parameter, making it straightforward to store in shared app state.
pub struct BoxedGovernorLayer {
	inner: GovernorLayer<String>,
}

impl BoxedGovernorLayer {
	/// Erase the key type of `config` and build a `BoxedGovernorLayer`.
	pub fn from_config<K>(config: GovernorConfig<K>) -> Self
	where
		K: std::hash::Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
	{
		let erased_extractor: ExtractorSlot<String> = match config.extractor {
			ExtractorSlot::None => ExtractorSlot::None,
			ExtractorSlot::Sync(arc) => ExtractorSlot::Sync(Arc::new(StringKeyAdapter { inner: arc })),
			ExtractorSlot::Async(arc) => {
				ExtractorSlot::Async(Arc::new(AsyncStringKeyAdapter { inner: arc }))
			}
		};

		let string_config = GovernorConfig {
			extractor: erased_extractor,
			quota_default: config.quota_default,
			quota_methods: config.quota_methods,
			stack: config.stack,
			whitelist_methods: config.whitelist_methods,
			whitelist_paths: config.whitelist_paths,
			whitelist_ips: config.whitelist_ips,
			body_preset: config.body_preset,
			error_handler: config.error_handler,
			gc_interval: config.gc_interval,
			gc_disabled: config.gc_disabled,
			max_keys: config.max_keys,
			connect_info_required: config.connect_info_required,
			legacy_reset_epoch: config.legacy_reset_epoch,
			redact_keys: config.redact_keys,
		};

		Self { inner: GovernorLayer::new(string_config) }
	}

	/// Return a handle for live introspection of the running limiter state.
	pub fn limiter(&self) -> crate::snapshot::LimiterHandle<String> {
		self.inner.limiter()
	}
}

impl<S> tower::Layer<S> for BoxedGovernorLayer
where
	S: tower::Service<http::Request<axum::body::Body>, Response = http::Response<axum::body::Body>>
		+ Clone
		+ Send
		+ 'static,
	S::Future: Send + 'static,
	S::Error: Send + 'static,
{
	type Service = crate::service::Governor<S, String>;

	fn layer(&self, inner: S) -> Self::Service {
		self.inner.layer(inner)
	}
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
	use super::*;
	use crate::builder::GovernorConfigBuilder;
	use crate::extractor::{Global, PeerIp};
	use crate::{Quota, nz};
	use axum::extract::ConnectInfo;
	use http::{Method, Request, Response, StatusCode};
	use std::convert::Infallible;
	use std::net::SocketAddr;
	use tower::{Layer as _, ServiceExt as _};

	fn ok_inner() -> impl tower::Service<
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

	fn req_with_peer(method: Method, path: &str, peer: SocketAddr) -> Request<axum::body::Body> {
		let mut r =
			Request::builder().method(method).uri(path).body(axum::body::Body::empty()).unwrap();
		r.extensions_mut().insert(ConnectInfo::<SocketAddr>(peer));
		r
	}

	fn req(method: Method, path: &str) -> Request<axum::body::Body> {
		Request::builder().method(method).uri(path).body(axum::body::Body::empty()).unwrap()
	}

	#[tokio::test]
	async fn boxed_constructs_from_global_config_ok() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quota_default(Quota::requests_per_second(nz!(10u32)))
			.finish()
			.unwrap();
		let _ = BoxedGovernorLayer::from_config(cfg);
	}

	#[tokio::test]
	async fn boxed_rate_limits_correctly() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quota_default(Quota::requests_per_second(nz!(1u32)))
			.finish()
			.unwrap();
		let layer = BoxedGovernorLayer::from_config(cfg);
		let svc = layer.layer(ok_inner());
		let r1 = svc.clone().oneshot(req(Method::GET, "/")).await.unwrap();
		assert_eq!(r1.status(), StatusCode::OK);
		let r2 = svc.clone().oneshot(req(Method::GET, "/")).await.unwrap();
		assert_eq!(r2.status(), StatusCode::TOO_MANY_REQUESTS);
	}

	#[tokio::test]
	async fn boxed_preserves_distinct_keys_via_debug() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(PeerIp::default())
			.expect_connect_info()
			.quota_default(Quota::requests_per_second(nz!(1u32)))
			.finish()
			.unwrap();
		let layer = BoxedGovernorLayer::from_config(cfg);
		let svc = layer.layer(ok_inner());
		let peer_a: SocketAddr = "1.2.3.4:1234".parse().unwrap();
		let peer_b: SocketAddr = "5.6.7.8:1234".parse().unwrap();
		let r1 = svc.clone().oneshot(req_with_peer(Method::GET, "/", peer_a)).await.unwrap();
		assert_eq!(r1.status(), StatusCode::OK);
		let r2 = svc.clone().oneshot(req_with_peer(Method::GET, "/", peer_b)).await.unwrap();
		assert_eq!(r2.status(), StatusCode::OK);
		let r3 = svc.clone().oneshot(req_with_peer(Method::GET, "/", peer_a)).await.unwrap();
		assert_eq!(r3.status(), StatusCode::TOO_MANY_REQUESTS);
	}
}
