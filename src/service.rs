//! `tower::Service` per-request flow.

use std::future::Future;
use std::hash::Hash;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use http::{HeaderMap, HeaderName, Request, Response};
use pin_project_lite::pin_project;

use crate::Quota;
use crate::builder::{ExtractorSlot, Settings};
use crate::error::RejectionReason;
use crate::extractor::KeyOutcome;
use crate::headers::{
	PolicyDescriptor, render_policy_value, write_ietf_rate_limit, write_legacy_rate_limit,
	write_retry_after,
};
use crate::layer::{KeyedRateLimiter, LimiterShared, PolicyEntry};
use crate::limiters::StackedResult;
use crate::response::{default_body, default_status};
use crate::tracker::{EvictionReason, KeyTracker};

#[derive(Clone)]
pub struct Governor<S, K>
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	pub(crate) inner: S,
	pub(crate) shared: Arc<LimiterShared<K>>,
}

pin_project! {
	pub struct GovernorFuture<F, E> {
		#[pin] state: GovernorFutureState<F, E>,
	}
}

pin_project! {
	#[project = StateProj]
	enum GovernorFutureState<F, E> {
		Admit {
			#[pin] inner: F,
			headers: Option<HeaderMap>,
		},
		Reject {
			response: Option<Response<axum::body::Body>>,
		},
		// Boxed state for the async extractor branch.
		// Pin<Box<...>> is Unpin itself, so no #[pin] attribute is needed.
		Boxed {
			fut: Option<Pin<Box<dyn Future<Output = Result<Response<axum::body::Body>, E>> + Send>>>,
		},
	}
}

impl<S, K, ReqBody> tower::Service<Request<ReqBody>> for Governor<S, K>
where
	S: tower::Service<Request<ReqBody>, Response = Response<axum::body::Body>>
		+ Clone
		+ Send
		+ 'static,
	S::Future: Send + 'static,
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
	ReqBody: Send + 'static,
{
	type Response = Response<axum::body::Body>;
	type Error = S::Error;
	type Future = GovernorFuture<S::Future, S::Error>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), S::Error>> {
		self.inner.poll_ready(cx)
	}

	fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
		match &self.shared.extractor {
			ExtractorSlot::Async(_) => {
				call_async_dispatch(Arc::clone(&self.shared), self.inner.clone(), req)
			}
			_ => call_sync(&mut self.inner, &self.shared, req),
		}
	}
}

fn call_sync<S, K, ReqBody>(
	inner: &mut S,
	shared: &Arc<LimiterShared<K>>,
	req: Request<ReqBody>,
) -> GovernorFuture<S::Future, S::Error>
where
	S: tower::Service<Request<ReqBody>, Response = Response<axum::body::Body>>,
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	let (parts, body) = req.into_parts();

	#[cfg(feature = "tracing")]
	let _span_guard = crate::trace::span_for(&parts.method, parts.uri.path()).entered();

	if is_whitelisted(&shared.settings, &parts) {
		return GovernorFuture::admit(inner.call(Request::from_parts(parts, body)), HeaderMap::new());
	}

	let outcome = match &shared.extractor {
		ExtractorSlot::Sync(e) => e.extract(&parts),
		ExtractorSlot::Async(_) => unreachable!("call_sync dispatched for sync extractor only"),
		ExtractorSlot::None => unreachable!("guarded at GovernorConfigBuilder::finish"),
	};

	match decide(shared, &parts, outcome) {
		Decision::Reject(response) => GovernorFuture::reject(response),
		Decision::Admit(headers) => {
			GovernorFuture::admit(inner.call(Request::from_parts(parts, body)), headers)
		}
	}
}

fn call_async_dispatch<S, K, ReqBody>(
	shared: Arc<LimiterShared<K>>,
	mut inner: S,
	req: Request<ReqBody>,
) -> GovernorFuture<S::Future, S::Error>
where
	S: tower::Service<Request<ReqBody>, Response = Response<axum::body::Body>> + Send + 'static,
	S::Future: Send + 'static,
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
	ReqBody: Send + 'static,
{
	let (parts, body) = req.into_parts();

	type BoxedFut<E> = Pin<Box<dyn Future<Output = Result<Response<axum::body::Body>, E>> + Send>>;

	#[cfg(feature = "tracing")]
	let _async_span = crate::trace::span_for(&parts.method, parts.uri.path());

	let inner_fut = async move {
		if is_whitelisted(&shared.settings, &parts) {
			return inner.call(Request::from_parts(parts, body)).await;
		}

		let outcome = match &shared.extractor {
			ExtractorSlot::Async(e) => e.extract(&parts).await,
			_ => unreachable!("call_async_dispatch dispatched for async extractor only"),
		};

		match decide(&shared, &parts, outcome) {
			Decision::Reject(response) => Ok(response),
			Decision::Admit(headers) => {
				let mut resp = inner.call(Request::from_parts(parts, body)).await?;
				merge_headers(&mut resp, headers);
				Ok(resp)
			}
		}
	};

	#[cfg(feature = "tracing")]
	let fut: BoxedFut<S::Error> = {
		use tracing::Instrument as _;
		Box::pin(inner_fut.instrument(_async_span))
	};
	#[cfg(not(feature = "tracing"))]
	let fut: BoxedFut<S::Error> = Box::pin(inner_fut);

	GovernorFuture { state: GovernorFutureState::Boxed { fut: Some(fut) } }
}

/// What the limiter decided for one request, once the key is known. Shared by the sync
/// and async paths so the precedence rules exist in exactly one place.
enum Decision {
	Reject(Response<axum::body::Body>),
	/// Headers to copy onto the inner response; empty when no policy applies.
	Admit(HeaderMap),
}

/// A whitelist hit bypasses the limiter with no header injection.
fn is_whitelisted(settings: &Settings, parts: &http::request::Parts) -> bool {
	settings.whitelist_methods.contains(&parts.method)
		|| settings.whitelist_paths.iter().any(|p| crate::glob::path_matches(p, parts.uri.path()))
		|| crate::extractor::ip::peer_ip(parts)
			.is_some_and(|ip| settings.whitelist_ips.iter().any(|n| n.contains(&ip)))
}

fn merge_headers(resp: &mut Response<axum::body::Body>, extra: HeaderMap) {
	let dst = resp.headers_mut();
	for (name, value) in extra {
		if let Some(n) = name {
			dst.insert(n, value);
		}
	}
}

fn decide<K>(
	shared: &LimiterShared<K>,
	parts: &http::request::Parts,
	outcome: Result<KeyOutcome<K>, crate::ExtractionError>,
) -> Decision
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	let cfg = &shared.settings;

	let outcome = match outcome {
		Ok(o) => o,
		Err(err) => {
			crate::trace::extraction_failed(&err);
			return Decision::Reject(build_base_response(cfg, RejectionReason::KeyExtractionFailed(err)));
		}
	};

	let policy_entry = pick_policy_entry(shared, &parts.method, outcome.quota_override.is_some());
	let dispatch = pick_primary(shared, &parts.method, outcome.quota_override);
	let primary_result = dispatch.as_ref().map(|d| d.run(&outcome.key));

	if let Some(LimiterOutcome::Reject { wait, quota }) = primary_result {
		let key_repr = crate::util::format_key(&outcome.key, cfg.redact_keys);
		crate::trace::reject(&key_repr, quota.inner().burst_size().get(), wait.as_millis(), "default");
		let label = Arc::clone(&shared.default_label);
		let reason = RejectionReason::QuotaExceeded {
			wait,
			snapshot: synthetic_snapshot(quota),
			key: Box::new(outcome.key.clone()) as Box<dyn std::any::Any + Send>,
			policy_name: Arc::clone(&label),
		};
		return Decision::Reject(build_reject_response(
			cfg,
			quota,
			wait,
			reason,
			&label,
			policy_entry,
			shared,
		));
	}

	let primary_remaining = match primary_result {
		Some(LimiterOutcome::Admit { remaining }) => Some(remaining),
		_ => None,
	};
	let mut lowest_remaining = primary_remaining.unwrap_or(u32::MAX);

	// Walk the stack in order; the first reject wins.
	for entry in shared.stack.iter() {
		match entry.check(parts, cfg.redact_keys) {
			StackedResult::ExtractionFailed(err) => {
				crate::trace::extraction_failed(&err);
				return Decision::Reject(build_base_response(
					cfg,
					RejectionReason::KeyExtractionFailed(err),
				));
			}
			StackedResult::Reject { wait, key_repr } => {
				let quota = entry.quota();
				let name = entry.name_arc();
				crate::trace::reject(&key_repr, quota.inner().burst_size().get(), wait.as_millis(), &name);
				let reason = RejectionReason::QuotaExceeded {
					wait,
					snapshot: synthetic_snapshot(quota),
					// Stack entries are type-erased, so the key itself cannot be carried
					// here; the formatted key is in the tracing event.
					key: Box::new(()) as Box<dyn std::any::Any + Send>,
					policy_name: Arc::clone(&name),
				};
				return Decision::Reject(build_reject_response(
					cfg,
					quota,
					wait,
					reason,
					&name,
					policy_entry,
					shared,
				));
			}
			StackedResult::Admit { remaining } => lowest_remaining = lowest_remaining.min(remaining),
		}
	}

	if dispatch.is_none() && shared.stack.is_empty() {
		return Decision::Admit(HeaderMap::new());
	}

	// The RateLimit: header names one policy: the primary when there is one, otherwise the
	// first advertised stack entry.
	let (admit_quota, admit_name) = if let Some(d) = dispatch.as_ref() {
		(d.quota, Arc::clone(&shared.default_label))
	} else if let Some(first) = policy_entry.and_then(|e| e.descriptors.first()) {
		(first.quota, Arc::clone(&first.name))
	} else {
		return Decision::Admit(HeaderMap::new());
	};

	let final_remaining = if lowest_remaining == u32::MAX { 0 } else { lowest_remaining };
	Decision::Admit(build_admit_headers(
		cfg.legacy_reset_epoch,
		admit_quota,
		&admit_name,
		final_remaining,
		policy_entry,
		shared,
		outcome.quota_override,
	))
}

enum LimiterOutcome {
	Admit { remaining: u32 },
	Reject { wait: Duration, quota: Quota },
}

/// Bundle the limiter, tracker and quota chosen by the precedence rules so the
/// dispatch site doesn't have to thread three options through the rest of the
/// flow. `Dispatch::run` performs the check and the tracker bookkeeping in a
/// single call so the hot path stays linear.
struct Dispatch<'a, K>
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	limiter: LimiterRef<'a, K>,
	tracker: Option<&'a KeyTracker<K>>,
	quota: Quota,
}

enum LimiterRef<'a, K>
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	Borrowed(&'a KeyedRateLimiter<K>),
	Owned(Arc<KeyedRateLimiter<K>>),
}

impl<K> LimiterRef<'_, K>
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	fn as_ref(&self) -> &KeyedRateLimiter<K> {
		match self {
			Self::Borrowed(l) => l,
			Self::Owned(arc) => arc.as_ref(),
		}
	}
}

impl<K> Dispatch<'_, K>
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	fn run(&self, key: &K) -> LimiterOutcome {
		let outcome = check_limiter(self.limiter.as_ref(), key, self.quota);
		if let Some(tracker) = self.tracker
			&& tracker.touch(key) == Some(EvictionReason::MaxKeys)
		{
			crate::trace::eviction("default");
			self.limiter.as_ref().retain_recent();
		}
		outcome
	}
}

fn pick_primary<'a, K>(
	shared: &'a LimiterShared<K>,
	method: &http::Method,
	quota_override: Option<Quota>,
) -> Option<Dispatch<'a, K>>
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	if let Some(method_quota) =
		shared.settings.quota_methods.iter().find(|(m, _)| m == method).map(|(_, q)| *q)
	{
		let limiter = shared
			.method_limiters
			.iter()
			.find(|(m, _)| m == method)
			.map(|(_, l)| l)
			.expect("method_limiters and quota_methods are built in parallel");
		let tracker = shared.method_trackers.iter().find(|(m, _)| m == method).map(|(_, t)| t);
		return Some(Dispatch { limiter: LimiterRef::Borrowed(limiter), tracker, quota: method_quota });
	}

	if let Some(override_quota) = quota_override {
		let limiter = shared.tier_cache.get_or_insert(override_quota);
		// Tier-cache buckets have their own state stores per quota; we don't track
		// the override path through the per-limiter LRU since hit counts would split
		// across (key, quota) combinations. The default tracker still sees the key,
		// so top_n reflects activity faithfully when the user opts into max_keys.
		return Some(Dispatch {
			limiter: LimiterRef::Owned(limiter),
			tracker: shared.default_tracker.as_ref(),
			quota: override_quota,
		});
	}

	if let Some(limiter) = shared.default_limiter.as_ref() {
		let q = shared.settings.quota_default.expect("default_limiter present => quota_default Some");
		return Some(Dispatch {
			limiter: LimiterRef::Borrowed(limiter),
			tracker: shared.default_tracker.as_ref(),
			quota: q,
		});
	}

	None
}

fn pick_policy_entry<'a, K>(
	shared: &'a LimiterShared<K>,
	method: &http::Method,
	has_tier_override: bool,
) -> Option<&'a PolicyEntry>
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	// Per-method overrides shadow the default-quota policy entry.
	if let Some(entry) = shared.policy_per_method.iter().find(|(m, _)| m == method).map(|(_, e)| e) {
		return Some(entry);
	}
	// Tier overrides change the "default" entry's quota at runtime, so the
	// pre-rendered default header is not accurate for that case. Fall back to
	// inline rendering at the call site instead of returning a stale entry.
	if has_tier_override {
		return None;
	}
	shared.policy_default.as_ref()
}

fn check_limiter<K: Hash + Eq + Clone + Send + Sync + 'static>(
	limiter: &KeyedRateLimiter<K>,
	key: &K,
	quota: Quota,
) -> LimiterOutcome {
	use governor::clock::Clock as _;
	match limiter.check_key(key) {
		Ok(snapshot) => LimiterOutcome::Admit { remaining: snapshot.remaining_burst_capacity() },
		Err(not_until) => {
			let now = governor::clock::DefaultClock::default().now();
			LimiterOutcome::Reject { wait: not_until.wait_time_from(now), quota }
		}
	}
}

fn synthetic_snapshot(quota: Quota) -> governor::middleware::StateSnapshot {
	use governor::middleware::StateInformationMiddleware;
	governor::RateLimiter::direct(quota.inner())
		.with_middleware::<StateInformationMiddleware>()
		.check()
		.expect("fresh direct limiter always allows first check")
}

fn build_admit_headers<K>(
	legacy_reset_epoch: bool,
	quota: Quota,
	policy_name: &str,
	remaining: u32,
	policy_entry: Option<&PolicyEntry>,
	shared: &LimiterShared<K>,
	tier_override: Option<Quota>,
) -> HeaderMap
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	let mut headers = HeaderMap::new();
	let burst = quota.inner().burst_size().get();

	let replenish_nanos = quota.inner().replenish_interval().as_nanos();
	let consumed = (burst - remaining.min(burst)) as u128;
	let t = ((consumed * replenish_nanos) / 1_000_000_000) as u64;

	let reset = if legacy_reset_epoch {
		SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() + t
	} else {
		t
	};

	insert_policy_header(&mut headers, policy_entry, shared, tier_override, quota, policy_name);
	write_ietf_rate_limit(&mut headers, policy_name, remaining, t);
	write_legacy_rate_limit(&mut headers, burst, remaining, reset);

	headers
}

fn build_reject_response<K>(
	cfg: &crate::builder::Settings,
	quota: Quota,
	wait: Duration,
	reason: RejectionReason,
	policy_name: &str,
	policy_entry: Option<&PolicyEntry>,
	shared: &LimiterShared<K>,
) -> Response<axum::body::Body>
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
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

	insert_policy_header(h, policy_entry, shared, None, quota, policy_name);
	write_ietf_rate_limit(h, policy_name, 0, delta);
	write_legacy_rate_limit(h, burst, 0, reset);

	response
}

/// Insert the `RateLimit-Policy` header. Prefers the precomputed value held by
/// `LimiterShared`; falls back to inline rendering only when a tier override has
/// changed the default-quota policy at request time.
fn insert_policy_header<K>(
	headers: &mut HeaderMap,
	policy_entry: Option<&PolicyEntry>,
	shared: &LimiterShared<K>,
	tier_override: Option<Quota>,
	fallback_quota: Quota,
	fallback_name: &str,
) where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	if let Some(entry) = policy_entry {
		// Hot path: clone the precomputed HeaderValue (cheap; backed by Bytes).
		headers.insert(HeaderName::from_static("ratelimit-policy"), entry.header.clone());
		return;
	}

	// Slow path: tier override forced inline rendering, or there is no precomputed
	// entry at all (e.g. caller has neither a default quota nor a stack and we still
	// got here for some reason). Build a transient view from the override (if any)
	// and the static stack descriptors.
	let mut view: Vec<PolicyDescriptor<'_>> = Vec::new();
	if let Some(q) = tier_override {
		view.push(PolicyDescriptor { name: &shared.default_label, quota: q });
	} else {
		view.push(PolicyDescriptor { name: fallback_name, quota: fallback_quota });
	}
	for entry in shared.stack.iter() {
		view.push(PolicyDescriptor { name: entry.name(), quota: entry.quota() });
	}
	if let Some(value) = render_policy_value(&view) {
		headers.insert(HeaderName::from_static("ratelimit-policy"), value);
	}
}

fn build_base_response(
	cfg: &crate::builder::Settings,
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

impl<F, E> GovernorFuture<F, E> {
	fn admit(inner: F, headers: HeaderMap) -> Self {
		Self { state: GovernorFutureState::Admit { inner, headers: Some(headers) } }
	}

	fn reject(response: Response<axum::body::Body>) -> Self {
		Self { state: GovernorFutureState::Reject { response: Some(response) } }
	}
}

impl<F, E> Future for GovernorFuture<F, E>
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
						merge_headers(&mut resp, extra);
					}
					Poll::Ready(Ok(resp))
				}
				Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
				Poll::Pending => Poll::Pending,
			},
			StateProj::Reject { response } => {
				Poll::Ready(Ok(response.take().expect("polled GovernorFuture::Reject after completion")))
			}
			// Pin<Box<dyn Future>> is Unpin; poll via as_mut().
			StateProj::Boxed { fut } => {
				let pinned = fut.as_mut().expect("polled GovernorFuture::Boxed after completion");
				pinned.as_mut().poll(cx)
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use http::{Method, Request, Response, StatusCode, header::AUTHORIZATION};
	use ipnet::IpNet;

	use crate::builder::GovernorConfigBuilder;
	use crate::extractor::{
		AsyncExtractFuture, AsyncKeyExtractor, Global, Header, KeyExtractor, KeyOutcome, PeerIp,
	};
	use crate::layer::GovernorLayer;
	use crate::test_utils::{drive_response, request, request_with_peer};
	use crate::{Quota, nz};

	fn req(method: Method, path: &str) -> Request<axum::body::Body> {
		request(method, path)
	}

	fn req_with_peer(method: Method, path: &str, peer: &str) -> Request<axum::body::Body> {
		request_with_peer(method, path, peer.parse().unwrap())
	}

	fn policy_header(resp: &Response<axum::body::Body>) -> Option<&str> {
		resp.headers().get("ratelimit-policy").map(|v| v.to_str().unwrap())
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

		// exhaust the single token
		let r1 = drive_response(&layer, req(Method::POST, "/")).await;
		assert_eq!(r1.status(), StatusCode::OK);

		// confirm bucket is exhausted
		let r2 = drive_response(&layer, req(Method::POST, "/")).await;
		assert_eq!(r2.status(), StatusCode::TOO_MANY_REQUESTS);

		// OPTIONS is whitelisted — must pass even though bucket is empty
		let r3 = drive_response(&layer, req(Method::OPTIONS, "/")).await;
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

		// exhaust the bucket
		let _ = drive_response(&layer, req(Method::GET, "/api")).await;

		// whitelisted path passes even though bucket is empty
		let r = drive_response(&layer, req(Method::GET, "/health")).await;
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

		// exhaust the bucket from a non-whitelisted IP
		let _ = drive_response(&layer, req_with_peer(Method::GET, "/", "1.2.3.4:1234")).await;

		// localhost is whitelisted
		let r = drive_response(&layer, req_with_peer(Method::GET, "/", "127.0.0.1:1234")).await;
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

		let r = drive_response(&layer, req(Method::GET, "/")).await;
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

		// first request succeeds
		let r1 = drive_response(&layer, req(Method::GET, "/")).await;
		assert_eq!(r1.status(), StatusCode::OK);

		// second request is rejected
		let r2 = drive_response(&layer, req(Method::GET, "/")).await;
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

		// request has no ConnectInfo → PeerIp fails with MissingConnectInfo → 500
		let r = drive_response(&layer, req(Method::GET, "/")).await;
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

		let r = drive_response(&layer, req(Method::GET, "/")).await;
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

		// exhaust bucket
		let _ = drive_response(&layer, req(Method::GET, "/")).await;

		// second request triggers handler
		let r = drive_response(&layer, req(Method::GET, "/")).await;
		assert_eq!(r.status(), StatusCode::from_u16(418).unwrap());
		// rate-limit headers are still injected on top of the custom response
		assert!(r.headers().get("ratelimit-policy").is_some());

		let body_bytes = axum::body::to_bytes(r.into_body(), usize::MAX).await.unwrap();
		assert!(body_bytes.starts_with(b"custom-body"));
	}

	#[tokio::test]
	async fn per_method_quota_get_exhausted_post_passes() {
		// GET has 1/s quota; POST falls through to default 100/s.
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quota_for(Method::GET, Quota::requests_per_second(nz!(1u32)))
			.quota_default(Quota::requests_per_second(nz!(100u32)))
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);

		// First GET passes.
		let r1 = drive_response(&layer, req(Method::GET, "/")).await;
		assert_eq!(r1.status(), StatusCode::OK);

		// Second GET is rejected.
		let r2 = drive_response(&layer, req(Method::GET, "/")).await;
		assert_eq!(r2.status(), StatusCode::TOO_MANY_REQUESTS);

		// POST uses default quota (100/s) — passes.
		let r3 = drive_response(&layer, req(Method::POST, "/")).await;
		assert_eq!(r3.status(), StatusCode::OK);

		// GET still rejected.
		let r4 = drive_response(&layer, req(Method::GET, "/")).await;
		assert_eq!(r4.status(), StatusCode::TOO_MANY_REQUESTS);
	}

	#[tokio::test]
	async fn stacked_peer_and_auth_first_reject_wins() {
		// Stack: peer 1/s (Global key), auth 600/min (Global key).
		// Exhaust peer on 2nd request; rejection names "peer" and lists both policies.
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.stack("peer", Global, Quota::requests_per_second(nz!(1u32)))
			.stack("auth", Global, Quota::requests_per_minute(nz!(600u32)))
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);

		// First request passes.
		let r1 = drive_response(&layer, req(Method::GET, "/")).await;
		assert_eq!(r1.status(), StatusCode::OK);

		// Second: peer (1/s) exhausted — 429.
		let r2 = drive_response(&layer, req(Method::GET, "/")).await;
		assert_eq!(r2.status(), StatusCode::TOO_MANY_REQUESTS);

		// RateLimit: header names the triggering entry.
		let rl = r2.headers().get("ratelimit").unwrap().to_str().unwrap();
		assert!(rl.contains("\"peer\""), "expected 'peer' in RateLimit header, got: {rl}");

		// RateLimit-Policy: lists both.
		let policy = r2.headers().get("ratelimit-policy").unwrap().to_str().unwrap();
		assert!(policy.contains("\"peer\""), "expected 'peer' in policy header");
		assert!(policy.contains("\"auth\""), "expected 'auth' in policy header");
	}

	#[tokio::test]
	async fn multi_window_quotas_first_rejects_on_per_second() {
		// quotas("peer", Global, [1/s, 60/m]) expands to peer:1s and peer:1m.
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quotas(
				"peer",
				Global,
				[Quota::requests_per_second(nz!(1u32)), Quota::requests_per_minute(nz!(60u32))],
			)
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);

		// First request passes.
		let r1 = drive_response(&layer, req(Method::GET, "/")).await;
		assert_eq!(r1.status(), StatusCode::OK);

		// Second request: peer:1s exhausted → 429.
		let r2 = drive_response(&layer, req(Method::GET, "/")).await;
		assert_eq!(r2.status(), StatusCode::TOO_MANY_REQUESTS);

		// Policy header lists both windows.
		let policy = r2.headers().get("ratelimit-policy").unwrap().to_str().unwrap();
		assert!(policy.contains("\"peer:1s\""), "expected peer:1s in policy, got: {policy}");
		assert!(policy.contains("\"peer:1m\""), "expected peer:1m in policy, got: {policy}");
	}

	#[derive(Clone)]
	struct TierExtractor;
	impl KeyExtractor for TierExtractor {
		type Key = ();
		fn extract(
			&self,
			_parts: &http::request::Parts,
		) -> Result<KeyOutcome<()>, crate::ExtractionError> {
			Ok(KeyOutcome { key: (), quota_override: Some(Quota::requests_per_second(nz!(100u32))) })
		}
	}

	#[tokio::test]
	async fn per_tier_quota_override_admits_burst() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(TierExtractor)
			// Default very tight; override allows more.
			.quota_default(Quota::requests_per_second(nz!(1u32)))
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);

		// With default 1/s the second would fail; with override 100/s it passes.
		let r1 = drive_response(&layer, req(Method::GET, "/")).await;
		assert_eq!(r1.status(), StatusCode::OK);
		let r2 = drive_response(&layer, req(Method::GET, "/")).await;
		assert_eq!(r2.status(), StatusCode::OK, "override quota should admit second request");
	}

	#[tokio::test]
	async fn async_extractor_admits_and_rejects() {
		#[derive(Clone, Debug)]
		struct SimpleAsync;
		impl AsyncKeyExtractor for SimpleAsync {
			type Key = ();
			fn extract<'a>(&'a self, _parts: &'a http::request::Parts) -> AsyncExtractFuture<'a, ()> {
				Box::pin(async {
					tokio::task::yield_now().await;
					Ok(KeyOutcome { key: (), quota_override: None })
				})
			}
		}

		let cfg = GovernorConfigBuilder::default()
			.with_async_extractor(SimpleAsync)
			.quota_default(Quota::requests_per_second(nz!(1u32)))
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);

		// First request: 200.
		let r1 = drive_response(&layer, req(Method::GET, "/")).await;
		assert_eq!(r1.status(), StatusCode::OK);

		// Second request: 429.
		let r2 = drive_response(&layer, req(Method::GET, "/")).await;
		assert_eq!(r2.status(), StatusCode::TOO_MANY_REQUESTS);
	}

	#[tokio::test]
	async fn stacked_reject_names_correct_entry_in_ratelimit_header() {
		// peer 100/s (never exhausted), auth (via Header) 1/s.
		// On the second request: peer passes (100 burst), auth rejects.
		// The RateLimit: header should name "auth".
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.stack("peer", Global, Quota::requests_per_second(nz!(100u32)))
			.stack("auth", Header(&AUTHORIZATION), Quota::requests_per_second(nz!(1u32)))
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);

		let make_req = || {
			Request::builder()
				.method(Method::GET)
				.uri("/")
				.header(AUTHORIZATION, "Bearer token123")
				.body(axum::body::Body::empty())
				.unwrap()
		};

		// First request passes.
		let r1 = drive_response(&layer, make_req()).await;
		assert_eq!(r1.status(), StatusCode::OK);

		// Second request: auth (1/s) exhausted → 429 naming "auth".
		let r2 = drive_response(&layer, make_req()).await;
		assert_eq!(r2.status(), StatusCode::TOO_MANY_REQUESTS);
		let rl = r2.headers().get("ratelimit").unwrap().to_str().unwrap();
		assert!(rl.contains("\"auth\""), "expected 'auth' in RateLimit header, got: {rl}");
	}

	#[tokio::test]
	async fn max_keys_evicts_oldest_under_pressure() {
		// max_keys=2 with PeerIp: after 3 distinct IPs, the oldest should have been
		// dropped from the sidecar tracker. governor's internal keyed state is nudged
		// via retain_recent(), but it cannot delete that exact fresh key in governor 0.10.
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(PeerIp::default())
			.expect_connect_info()
			.quota_default(Quota::requests_per_second(nz!(10u32)))
			.max_keys(2)
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);

		for ip in ["1.1.1.1:1", "2.2.2.2:1", "3.3.3.3:1"] {
			let r = drive_response(&layer, req_with_peer(Method::GET, "/", ip)).await;
			assert_eq!(r.status(), StatusCode::OK);
		}

		let snap = layer.limiter().snapshot();
		assert_eq!(snap.top_n.len(), 2, "tracker-backed top_n should be capped by max_keys");
		assert!(
			!snap.top_n.iter().any(|(k, _)| k.contains("1.1.1.1")),
			"oldest key should have been evicted from tracker: {:?}",
			snap.top_n
		);
	}

	#[test]
	fn format_key_redact_off_uses_debug() {
		let k: String = "alice".into();
		assert_eq!(crate::util::format_key(&k, false), "\"alice\"");
	}

	#[test]
	fn format_key_redact_on_hashes() {
		let s = crate::util::format_key(&String::from("alice"), true);
		assert!(s.starts_with("hash:"), "got {s}");
		assert_eq!(s.len(), "hash:".len() + 16);
	}

	#[tokio::test]
	async fn tier_override_renders_its_own_quota_in_the_policy_header() {
		// The precomputed "default" policy header carries the default quota; a tier
		// override must fall back to inline rendering so the header tells the truth.
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(TierExtractor)
			.quota_default(Quota::requests_per_second(nz!(1u32)))
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);

		let r = drive_response(&layer, req(Method::GET, "/")).await;
		assert_eq!(r.status(), StatusCode::OK);
		assert_eq!(policy_header(&r), Some("\"default\";q=100;w=1"));
		assert_eq!(r.headers()["x-ratelimit-limit"], "100");
	}

	#[tokio::test]
	async fn stack_only_admit_names_the_first_entry() {
		// No primary quota: the RateLimit: header still has to name one policy, and the
		// first advertised stack entry is the one a client would want to pace against.
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.stack("peer", Global, Quota::requests_per_second(nz!(10u32)))
			.stack("auth", Global, Quota::requests_per_minute(nz!(600u32)))
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);

		let r = drive_response(&layer, req(Method::GET, "/")).await;
		assert_eq!(r.status(), StatusCode::OK);
		let rl = r.headers()["ratelimit"].to_str().unwrap();
		assert!(rl.starts_with("\"peer\";r=9;"), "got {rl}");
		assert_eq!(policy_header(&r), Some("\"peer\";q=10;w=1, \"auth\";q=600;w=60"));
	}

	#[tokio::test]
	async fn stack_extraction_failure_rejects_with_extraction_status() {
		// A stack entry whose extractor fails is a request problem, not a quota problem:
		// 400 for a missing header, and no rate-limit headers because nothing was counted.
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quota_default(Quota::requests_per_second(nz!(10u32)))
			.stack("auth", Header(&AUTHORIZATION), Quota::requests_per_second(nz!(10u32)))
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);

		let r = drive_response(&layer, req(Method::GET, "/")).await;
		assert_eq!(r.status(), StatusCode::BAD_REQUEST);
		assert!(policy_header(&r).is_none());
	}

	#[tokio::test]
	async fn error_handler_sees_key_extraction_failed() {
		use std::sync::Arc;
		use std::sync::Mutex;

		let seen: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
		let sink = Arc::clone(&seen);
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(PeerIp::default())
			.expect_connect_info()
			.quota_default(Quota::requests_per_second(nz!(10u32)))
			.error_handler(move |reason| {
				*sink.lock().unwrap() = Some(format!("{reason:?}"));
				Response::builder().status(503).body(axum::body::Body::empty()).unwrap()
			})
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);

		let r = drive_response(&layer, req(Method::GET, "/")).await;
		assert_eq!(r.status(), StatusCode::SERVICE_UNAVAILABLE);
		let seen = seen.lock().unwrap().clone().expect("handler was called");
		assert!(seen.contains("KeyExtractionFailed(MissingConnectInfo)"), "got {seen}");
	}

	#[tokio::test]
	async fn whitelist_ip_is_judged_on_the_peer_not_forwarded_headers() {
		// The whitelist runs before extraction and only ever sees ConnectInfo, so an
		// X-Forwarded-For naming a whitelisted address must not bypass the limiter.
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quota_default(Quota::requests_per_second(nz!(1u32)))
			.whitelist_ips(["127.0.0.0/8".parse::<IpNet>().unwrap()])
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);

		let spoofed = || {
			let mut r = req_with_peer(Method::GET, "/", "8.8.8.8:1");
			r.headers_mut().insert("x-forwarded-for", "127.0.0.1".parse().unwrap());
			r
		};
		drive_response(&layer, spoofed()).await;
		let r = drive_response(&layer, spoofed()).await;
		assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
	}

	#[cfg(feature = "tracing")]
	#[tokio::test]
	async fn tracing_smoke_compiles_and_runs() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quota_default(Quota::requests_per_second(nz!(1u32)))
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);
		drive_response(&layer, req(Method::GET, "/")).await;
		drive_response(&layer, req(Method::GET, "/")).await;
	}
}
