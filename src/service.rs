//! `tower::Service` per-request flow.

use std::future::Future;
use std::hash::Hash;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use http::{HeaderMap, Request, Response};
use pin_project_lite::pin_project;

use crate::Quota;
use crate::builder::ExtractorSlot;
use crate::error::RejectionReason;
use crate::headers::{
	PolicyDescriptor, write_ietf_policy_set, write_ietf_rate_limit, write_legacy_rate_limit,
	write_retry_after,
};
use crate::layer::LimiterShared;
use crate::response::{default_body, default_status};

#[derive(Clone)]
pub struct Governor<S, K>
where
	K: Hash + Eq + Clone + Send + Sync + 'static,
{
	pub(crate) inner: S,
	pub(crate) shared: Arc<LimiterShared<K>>,
}

pin_project! {
	pub struct GovernorFuture<F> {
		#[pin] state: GovernorFutureState<F>,
	}
}

pin_project! {
	#[project = StateProj]
	enum GovernorFutureState<F> {
		Admit {
			#[pin] inner: F,
			headers: Option<HeaderMap>,
		},
		Reject {
			response: Option<Response<axum::body::Body>>,
		},
	}
}

impl<S, K, ReqBody> tower::Service<Request<ReqBody>> for Governor<S, K>
where
	S: tower::Service<Request<ReqBody>, Response = Response<axum::body::Body>>,
	K: Hash + Eq + Clone + Send + Sync + 'static,
{
	type Response = Response<axum::body::Body>;
	type Error = S::Error;
	type Future = GovernorFuture<S::Future>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), S::Error>> {
		self.inner.poll_ready(cx)
	}

	fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
		// Stage 6b will read these fields — listed here so deferred items are visible.
		// Currently ignored: shared.config.quota_methods, shared.config.stack,
		// and KeyOutcome::quota_override from the extractor result.
		let (parts, body) = req.into_parts();
		let cfg = &self.shared.config;

		// 1. Whitelist precedence — any hit bypasses the limiter with no header injection.
		if cfg.whitelist_methods.contains(&parts.method)
			|| cfg.whitelist_paths.iter().any(|p| crate::glob::path_matches(p, parts.uri.path()))
			|| peer_ip_from(&parts).is_some_and(|ip| cfg.whitelist_ips.iter().any(|n| n.contains(&ip)))
		{
			let req = Request::from_parts(parts, body);
			return GovernorFuture::admit(self.inner.call(req), HeaderMap::new());
		}

		// 2. Extract rate-limit key.
		let outcome = match &cfg.extractor {
			ExtractorSlot::Sync(e) => e.extract(&parts),
			ExtractorSlot::Async(_) => unreachable!("guarded at Layer::new"),
			ExtractorSlot::None => unreachable!("guarded at GovernorConfigBuilder::finish"),
		};
		let outcome = match outcome {
			Ok(o) => o,
			Err(err) => {
				let reason = RejectionReason::KeyExtractionFailed(err);
				return GovernorFuture::reject(build_reject_no_headers(cfg, reason));
			}
		};

		// 3. Check default limiter; pass through if no quota is configured.
		let Some(limiter) = self.shared.default_limiter.as_ref() else {
			let req = Request::from_parts(parts, body);
			return GovernorFuture::admit(self.inner.call(req), HeaderMap::new());
		};

		match limiter.check_key(&outcome.key) {
			Ok(snapshot) => {
				let q = cfg.quota_default.expect("limiter present ⇒ quota_default Some");
				let headers = build_admit_headers(cfg.legacy_reset_epoch, q, &snapshot);
				let req = Request::from_parts(parts, body);
				GovernorFuture::admit(self.inner.call(req), headers)
			}
			Err(not_until) => {
				use governor::clock::Clock as _;
				let now = governor::clock::DefaultClock::default().now();
				let wait = not_until.wait_time_from(now);

				// NotUntil<P> does not expose its internal StateSnapshot via the public API.
				// Construct a synthetic snapshot from the same quota. Its
				// remaining_burst_capacity() will be burst-1 rather than 0; all emitted headers
				// use remaining=0 computed directly from wait.
				let snapshot = governor::RateLimiter::direct(not_until.quota())
					.with_middleware::<governor::middleware::StateInformationMiddleware>()
					.check()
					.expect("fresh direct limiter always allows first check");

				let q = cfg.quota_default.expect("limiter present ⇒ quota_default Some");
				let reason = RejectionReason::QuotaExceeded {
					wait,
					snapshot,
					key: Box::new(outcome.key.clone()) as Box<dyn std::any::Any + Send>,
					policy_name: "default",
				};
				GovernorFuture::reject(build_reject_response(cfg, q, wait, reason))
			}
		}
	}
}

fn peer_ip_from(parts: &http::request::Parts) -> Option<std::net::IpAddr> {
	parts.extensions.get::<axum::extract::ConnectInfo<SocketAddr>>().map(|ci| ci.0.ip())
}

fn build_admit_headers(
	legacy_reset_epoch: bool,
	quota: Quota,
	snapshot: &governor::middleware::StateSnapshot,
) -> HeaderMap {
	let mut headers = HeaderMap::new();
	let burst = quota.inner().burst_size().get();
	let remaining = snapshot.remaining_burst_capacity();

	// t = whole seconds until the bucket can absorb (burst - remaining) more requests.
	let replenish_nanos = quota.inner().replenish_interval().as_nanos();
	let consumed = (burst - remaining) as u128;
	let t = ((consumed * replenish_nanos) / 1_000_000_000) as u64;

	let reset = if legacy_reset_epoch {
		SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() + t
	} else {
		t
	};

	write_ietf_policy_set(&mut headers, &[PolicyDescriptor { name: "default", quota }]);
	write_ietf_rate_limit(&mut headers, "default", remaining, t);
	write_legacy_rate_limit(&mut headers, burst, remaining, reset);

	headers
}

fn build_reject_response<K>(
	cfg: &crate::builder::GovernorConfig<K>,
	quota: Quota,
	wait: Duration,
	reason: RejectionReason,
) -> Response<axum::body::Body> {
	let burst = quota.inner().burst_size().get();
	let delta = wait.as_secs();
	let retry_after = delta.max(1);

	let reset = if cfg.legacy_reset_epoch {
		SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() + delta
	} else {
		delta
	};

	let mut response = build_base_response(cfg, reason);

	let h = response.headers_mut();
	write_retry_after(h, retry_after);
	write_ietf_policy_set(h, &[PolicyDescriptor { name: "default", quota }]);
	write_ietf_rate_limit(h, "default", 0, delta);
	write_legacy_rate_limit(h, burst, 0, reset);

	response
}

fn build_reject_no_headers<K>(
	cfg: &crate::builder::GovernorConfig<K>,
	reason: RejectionReason,
) -> Response<axum::body::Body> {
	build_base_response(cfg, reason)
}

fn build_base_response<K>(
	cfg: &crate::builder::GovernorConfig<K>,
	reason: RejectionReason,
) -> Response<axum::body::Body> {
	if let Some(handler) = &cfg.error_handler {
		handler(reason)
	} else {
		let status = default_status(&reason);
		let (ct, body) = default_body(cfg.body_preset, &reason);
		let mut resp = Response::builder()
			.status(status)
			.body(axum::body::Body::from(body))
			.expect("static response shape is always valid");
		resp.headers_mut().insert(http::header::CONTENT_TYPE, ct);
		resp
	}
}

impl<F> GovernorFuture<F> {
	fn admit(inner: F, headers: HeaderMap) -> Self {
		Self { state: GovernorFutureState::Admit { inner, headers: Some(headers) } }
	}

	fn reject(response: Response<axum::body::Body>) -> Self {
		Self { state: GovernorFutureState::Reject { response: Some(response) } }
	}
}

impl<F, E> Future for GovernorFuture<F>
where
	F: Future<Output = Result<Response<axum::body::Body>, E>>,
{
	type Output = Result<Response<axum::body::Body>, E>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let this = self.project();
		match this.state.project() {
			StateProj::Admit { inner, headers } => match inner.poll(cx) {
				Poll::Ready(Ok(mut resp)) => {
					if let Some(extra) = headers.take() {
						let dst = resp.headers_mut();
						for (name, value) in extra {
							if let Some(n) = name {
								dst.insert(n, value);
							}
						}
					}
					Poll::Ready(Ok(resp))
				}
				Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
				Poll::Pending => Poll::Pending,
			},
			StateProj::Reject { response } => {
				Poll::Ready(Ok(response.take().expect("polled GovernorFuture::Reject after completion")))
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use std::convert::Infallible;
	use std::net::SocketAddr;

	use axum::extract::ConnectInfo;
	use http::{Method, Request, Response, StatusCode};
	use ipnet::IpNet;
	use tower::ServiceExt as _;

	use tower::Layer as _;

	use super::*;
	use crate::builder::GovernorConfigBuilder;
	use crate::extractor::{Global, PeerIp};
	use crate::layer::GovernorLayer;
	use crate::{Quota, nz};

	fn ok_inner() -> impl tower::Service<
		Request<axum::body::Body>,
		Response = Response<axum::body::Body>,
		Error = Infallible,
		Future = impl Future<Output = Result<Response<axum::body::Body>, Infallible>>,
	> + Clone {
		tower::service_fn(|_req: Request<axum::body::Body>| async {
			Ok::<_, Infallible>(Response::builder().status(200).body(axum::body::Body::empty()).unwrap())
		})
	}

	fn req(method: Method, path: &str) -> Request<axum::body::Body> {
		Request::builder().method(method).uri(path).body(axum::body::Body::empty()).unwrap()
	}

	fn req_with_peer(method: Method, path: &str, peer: &str) -> Request<axum::body::Body> {
		let addr: SocketAddr = peer.parse().unwrap();
		let mut r = req(method, path);
		r.extensions_mut().insert(ConnectInfo::<SocketAddr>(addr));
		r
	}

	#[tokio::test]
	async fn whitelist_method_bypasses_exhausted_limiter() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quota_default(Quota::requests_per_second(nz!(1u32)))
			.whitelist_methods([Method::OPTIONS])
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);
		let svc = layer.layer(ok_inner());

		// exhaust the single token
		let r1 = svc.clone().oneshot(req(Method::POST, "/")).await.unwrap();
		assert_eq!(r1.status(), StatusCode::OK);

		// confirm bucket is exhausted
		let r2 = svc.clone().oneshot(req(Method::POST, "/")).await.unwrap();
		assert_eq!(r2.status(), StatusCode::TOO_MANY_REQUESTS);

		// OPTIONS is whitelisted — must pass even though bucket is empty
		let r3 = svc.clone().oneshot(req(Method::OPTIONS, "/")).await.unwrap();
		assert_eq!(r3.status(), StatusCode::OK);
		assert!(r3.headers().get("ratelimit-policy").is_none());
	}

	#[tokio::test]
	async fn whitelist_path_bypasses_exhausted_limiter() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quota_default(Quota::requests_per_second(nz!(1u32)))
			.whitelist_paths(["/health"])
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);
		let svc = layer.layer(ok_inner());

		// exhaust the bucket
		let _ = svc.clone().oneshot(req(Method::GET, "/api")).await.unwrap();

		// whitelisted path passes even though bucket is empty
		let r = svc.clone().oneshot(req(Method::GET, "/health")).await.unwrap();
		assert_eq!(r.status(), StatusCode::OK);
		assert!(r.headers().get("ratelimit-policy").is_none());
	}

	#[tokio::test]
	async fn whitelist_ip_bypasses_exhausted_limiter() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quota_default(Quota::requests_per_second(nz!(1u32)))
			.whitelist_ips(["127.0.0.0/8".parse::<IpNet>().unwrap()])
			.expect_connect_info()
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);
		let svc = layer.layer(ok_inner());

		// exhaust the bucket from a non-whitelisted IP
		let _ = svc.clone().oneshot(req_with_peer(Method::GET, "/", "1.2.3.4:1234")).await.unwrap();

		// localhost is whitelisted
		let r = svc.clone().oneshot(req_with_peer(Method::GET, "/", "127.0.0.1:1234")).await.unwrap();
		assert_eq!(r.status(), StatusCode::OK);
		assert!(r.headers().get("ratelimit-policy").is_none());
	}

	#[tokio::test]
	async fn successful_admit_adds_headers() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quota_default(Quota::requests_per_second(nz!(10u32)))
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);
		let svc = layer.layer(ok_inner());

		let r = svc.oneshot(req(Method::GET, "/")).await.unwrap();
		assert_eq!(r.status(), StatusCode::OK);
		assert!(r.headers().get("ratelimit-policy").is_some());
		assert!(r.headers().get("ratelimit").is_some());
		assert!(r.headers().get("x-ratelimit-limit").is_some());
		assert!(r.headers().get("x-ratelimit-remaining").is_some());
		assert!(r.headers().get("x-ratelimit-reset").is_some());
	}

	#[tokio::test]
	async fn reject_returns_429_with_all_headers() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quota_default(Quota::requests_per_second(nz!(1u32)))
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);
		let svc = layer.layer(ok_inner());

		// first request succeeds
		let r1 = svc.clone().oneshot(req(Method::GET, "/")).await.unwrap();
		assert_eq!(r1.status(), StatusCode::OK);

		// second request is rejected
		let r2 = svc.clone().oneshot(req(Method::GET, "/")).await.unwrap();
		assert_eq!(r2.status(), StatusCode::TOO_MANY_REQUESTS);
		assert!(r2.headers().get("retry-after").is_some());
		assert!(r2.headers().get("ratelimit-policy").is_some());
		assert!(r2.headers().get("ratelimit").is_some());
		assert!(r2.headers().get("x-ratelimit-limit").is_some());
		assert!(r2.headers().get("x-ratelimit-remaining").is_some());
		assert!(r2.headers().get("x-ratelimit-reset").is_some());
	}

	#[tokio::test]
	async fn key_extraction_failed_returns_500_no_rl_headers() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(PeerIp::default())
			.expect_connect_info()
			.quota_default(Quota::requests_per_second(nz!(10u32)))
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);
		let svc = layer.layer(ok_inner());

		// request has no ConnectInfo → PeerIp fails with MissingConnectInfo → 500
		let r = svc.oneshot(req(Method::GET, "/")).await.unwrap();
		assert_eq!(r.status(), StatusCode::INTERNAL_SERVER_ERROR);
		assert!(r.headers().get("ratelimit-policy").is_none());
	}

	#[tokio::test]
	async fn legacy_reset_epoch_produces_epoch_value() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quota_default(Quota::requests_per_second(nz!(1u32)))
			.legacy_reset_epoch(true)
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);
		let svc = layer.layer(ok_inner());

		let r = svc.oneshot(req(Method::GET, "/")).await.unwrap();
		assert_eq!(r.status(), StatusCode::OK);

		let reset_val: u64 =
			r.headers().get("x-ratelimit-reset").unwrap().to_str().unwrap().parse().unwrap();
		// Any value greater than this threshold is clearly an epoch, not a delta.
		assert!(reset_val > 1_700_000_000, "expected epoch seconds, got {reset_val}");
	}

	#[tokio::test]
	async fn error_handler_response_used_with_rl_headers_attached() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quota_default(Quota::requests_per_second(nz!(1u32)))
			.error_handler(|_| {
				Response::builder().status(418).body(axum::body::Body::from("custom-body")).unwrap()
			})
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);
		let svc = layer.layer(ok_inner());

		// exhaust bucket
		let _ = svc.clone().oneshot(req(Method::GET, "/")).await.unwrap();

		// second request triggers handler
		let r = svc.clone().oneshot(req(Method::GET, "/")).await.unwrap();
		assert_eq!(r.status(), StatusCode::from_u16(418).unwrap());
		// rate-limit headers are still injected on top of the custom response
		assert!(r.headers().get("ratelimit-policy").is_some());

		let body_bytes = axum::body::to_bytes(r.into_body(), usize::MAX).await.unwrap();
		assert!(body_bytes.starts_with(b"custom-body"));
	}
}
