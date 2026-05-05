//! `tower::Service` per-request flow.

use std::future::Future;
use std::hash::Hash;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Tracing helpers (feature-gated)
// ---------------------------------------------------------------------------

#[cfg(feature = "tracing")]
fn span_for(method: &http::Method, path: &str) -> tracing::Span {
	tracing::debug_span!(target: "axum_governor::layer",
        "axum_governor::layer", method = %method, path = %path)
}

#[cfg(feature = "tracing")]
fn emit_reject_event(key_str: &str, quota_burst: u32, wait_ms: u128, policy: &str) {
	tracing::info!(target: "axum_governor",
        key = %key_str, quota_burst, wait_ms, policy,
        "rate limit exceeded");
}

#[cfg(not(feature = "tracing"))]
fn emit_reject_event(_key_str: &str, _quota_burst: u32, _wait_ms: u128, _policy: &str) {}

#[cfg(feature = "tracing")]
fn emit_extraction_failed_event(reason: &crate::ExtractionError) {
	tracing::warn!(target: "axum_governor", error = %reason,
        "rate-limit key extraction failed");
}

#[cfg(not(feature = "tracing"))]
fn emit_extraction_failed_event(_reason: &crate::ExtractionError) {}

#[cfg(feature = "tracing")]
fn emit_eviction_warn(name: &str) {
	tracing::warn!(target: "axum_governor", policy = %name,
        "max_keys exceeded; evicted oldest key from tracker and forced retain_recent");
}

#[cfg(not(feature = "tracing"))]
fn emit_eviction_warn(_name: &str) {}

use http::{HeaderMap, HeaderName, Request, Response};
use pin_project_lite::pin_project;

use crate::Quota;
use crate::builder::ExtractorSlot;
use crate::error::RejectionReason;
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
		match &self.shared.config.extractor {
			ExtractorSlot::Async(_) => {
				call_async_dispatch(Arc::clone(&self.shared), self.inner.clone(), req)
			}
			_ => call_sync(&mut self.inner, &self.shared, req),
		}
	}
}

// ---------------------------------------------------------------------------
// Sync dispatch
// ---------------------------------------------------------------------------

fn call_sync<S, K, ReqBody>(
	inner: &mut S,
	shared: &Arc<LimiterShared<K>>,
	req: Request<ReqBody>,
) -> GovernorFuture<S::Future, S::Error>
where
	S: tower::Service<Request<ReqBody>, Response = Response<axum::body::Body>>
		+ Clone
		+ Send
		+ 'static,
	S::Future: Send + 'static,
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
	ReqBody: Send + 'static,
{
	let (parts, body) = req.into_parts();

	#[cfg(feature = "tracing")]
	let _span_guard = span_for(&parts.method, parts.uri.path()).entered();

	let cfg = &shared.config;

	// 1. Whitelist precedence — any hit bypasses the limiter with no header injection.
	if cfg.whitelist_methods.contains(&parts.method)
		|| cfg.whitelist_paths.iter().any(|p| crate::glob::path_matches(p, parts.uri.path()))
		|| peer_ip_from(&parts).is_some_and(|ip| cfg.whitelist_ips.iter().any(|n| n.contains(&ip)))
	{
		let req = Request::from_parts(parts, body);
		return GovernorFuture::admit(inner.call(req), HeaderMap::new());
	}

	// 2. Extract rate-limit key.
	let outcome = match &cfg.extractor {
		ExtractorSlot::Sync(e) => e.extract(&parts),
		ExtractorSlot::Async(_) => unreachable!("call_sync dispatched for sync extractor only"),
		ExtractorSlot::None => unreachable!("guarded at GovernorConfigBuilder::finish"),
	};
	let outcome = match outcome {
		Ok(o) => o,
		Err(err) => {
			emit_extraction_failed_event(&err);
			let reason = RejectionReason::KeyExtractionFailed(err);
			return GovernorFuture::reject(build_reject_no_headers(cfg, reason));
		}
	};

	let dispatch = pick_primary(shared, &parts.method, outcome.quota_override);
	let primary_result = dispatch.as_ref().map(|d| d.run(&outcome.key));

	let active_quota: Option<Quota> = dispatch.as_ref().map(|d| d.quota);

	if let Some(LimiterOutcome::Reject { wait, quota }) = primary_result {
		let key_repr = crate::util::format_key(&outcome.key, cfg.redact_keys);
		emit_reject_event(&key_repr, quota.inner().burst_size().get(), wait.as_millis(), "default");
		let policy_entry = pick_policy_entry(shared, &parts.method, outcome.quota_override.is_some());
		let label = Arc::clone(&shared.default_label);
		let reason = RejectionReason::QuotaExceeded {
			wait,
			snapshot: synthetic_snapshot(quota),
			key: Box::new(outcome.key.clone()) as Box<dyn std::any::Any + Send>,
			policy_name: Arc::clone(&label),
		};
		return GovernorFuture::reject(build_reject_response(
			cfg,
			quota,
			wait,
			reason,
			&label,
			policy_entry,
			shared,
		));
	}

	let primary_remaining = primary_result.as_ref().and_then(|r| {
		if let LimiterOutcome::Admit { remaining } = r { Some(*remaining) } else { None }
	});

	// 4. Walk the stack in order; first reject wins.
	let policy_entry = pick_policy_entry(shared, &parts.method, outcome.quota_override.is_some());
	let mut lowest_remaining = primary_remaining.unwrap_or(u32::MAX);

	for entry in shared.stack.iter() {
		match entry.check(&parts, cfg.redact_keys) {
			StackedResult::ExtractionFailed(err) => {
				emit_extraction_failed_event(&err);
				let reason = RejectionReason::KeyExtractionFailed(err);
				return GovernorFuture::reject(build_reject_no_headers(cfg, reason));
			}
			StackedResult::Reject { wait, key_repr } => {
				let quota = entry.quota();
				let name = entry.name_arc();
				emit_reject_event(&key_repr, quota.inner().burst_size().get(), wait.as_millis(), &name);
				let reason = RejectionReason::QuotaExceeded {
					wait,
					snapshot: synthetic_snapshot(quota),
					key: Box::new(()) as Box<dyn std::any::Any + Send>,
					policy_name: Arc::clone(&name),
				};
				return GovernorFuture::reject(build_reject_response(
					cfg,
					quota,
					wait,
					reason,
					&name,
					policy_entry,
					shared,
				));
			}
			StackedResult::Admit { remaining } => {
				if remaining < lowest_remaining {
					lowest_remaining = remaining;
				}
			}
		}
	}

	// 5. Everything admitted.
	if dispatch.is_none() && shared.stack.is_empty() {
		let req = Request::from_parts(parts, body);
		return GovernorFuture::admit(inner.call(req), HeaderMap::new());
	}

	let (admit_quota, admit_name) = if let Some(q) = active_quota {
		(q, Arc::clone(&shared.default_label))
	} else if let Some(entry) = policy_entry.as_ref().filter(|e| !e.descriptors.is_empty()) {
		let first = &entry.descriptors[0];
		(first.quota, Arc::clone(&first.name))
	} else {
		let req = Request::from_parts(parts, body);
		return GovernorFuture::admit(inner.call(req), HeaderMap::new());
	};

	let final_remaining = if lowest_remaining == u32::MAX { 0 } else { lowest_remaining };
	let headers = build_admit_headers(
		cfg.legacy_reset_epoch,
		admit_quota,
		&admit_name,
		final_remaining,
		policy_entry,
		shared,
		outcome.quota_override,
	);
	let req = Request::from_parts(parts, body);
	GovernorFuture::admit(inner.call(req), headers)
}

// ---------------------------------------------------------------------------
// Async dispatch
// ---------------------------------------------------------------------------

fn call_async_dispatch<S, K, ReqBody>(
	shared: Arc<LimiterShared<K>>,
	mut inner: S,
	req: Request<ReqBody>,
) -> GovernorFuture<S::Future, S::Error>
where
	S: tower::Service<Request<ReqBody>, Response = Response<axum::body::Body>>
		+ Clone
		+ Send
		+ 'static,
	S::Future: Send + 'static,
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
	ReqBody: Send + 'static,
{
	let (parts, body) = req.into_parts();

	type BoxedFut<E> = Pin<Box<dyn Future<Output = Result<Response<axum::body::Body>, E>> + Send>>;

	#[cfg(feature = "tracing")]
	let _async_span = span_for(&parts.method, parts.uri.path());

	let inner_fut = async move {
		let cfg = &shared.config;

		// 1. Whitelist check.
		if cfg.whitelist_methods.contains(&parts.method)
			|| cfg.whitelist_paths.iter().any(|p| crate::glob::path_matches(p, parts.uri.path()))
			|| peer_ip_from(&parts).is_some_and(|ip| cfg.whitelist_ips.iter().any(|n| n.contains(&ip)))
		{
			let req = Request::from_parts(parts, body);
			return inner.call(req).await;
		}

		// 2. Extract key asynchronously.
		let outcome = match &cfg.extractor {
			ExtractorSlot::Async(e) => e.extract(&parts).await,
			_ => unreachable!("call_async_dispatch dispatched for async extractor only"),
		};
		let outcome = match outcome {
			Ok(o) => o,
			Err(err) => {
				emit_extraction_failed_event(&err);
				let reason = RejectionReason::KeyExtractionFailed(err);
				return Ok(build_reject_no_headers(cfg, reason));
			}
		};

		let dispatch = pick_primary(&shared, &parts.method, outcome.quota_override);
		let primary_result = dispatch.as_ref().map(|d| d.run(&outcome.key));
		let active_quota: Option<Quota> = dispatch.as_ref().map(|d| d.quota);

		if let Some(LimiterOutcome::Reject { wait, quota }) = primary_result {
			let key_repr = crate::util::format_key(&outcome.key, cfg.redact_keys);
			emit_reject_event(&key_repr, quota.inner().burst_size().get(), wait.as_millis(), "default");
			let policy_entry =
				pick_policy_entry(&shared, &parts.method, outcome.quota_override.is_some());
			let label = Arc::clone(&shared.default_label);
			let reason = RejectionReason::QuotaExceeded {
				wait,
				snapshot: synthetic_snapshot(quota),
				key: Box::new(outcome.key.clone()) as Box<dyn std::any::Any + Send>,
				policy_name: Arc::clone(&label),
			};
			return Ok(build_reject_response(cfg, quota, wait, reason, &label, policy_entry, &shared));
		}

		let primary_remaining = primary_result.as_ref().and_then(|r| {
			if let LimiterOutcome::Admit { remaining } = r { Some(*remaining) } else { None }
		});

		let policy_entry = pick_policy_entry(&shared, &parts.method, outcome.quota_override.is_some());
		let mut lowest_remaining = primary_remaining.unwrap_or(u32::MAX);

		for entry in shared.stack.iter() {
			match entry.check(&parts, cfg.redact_keys) {
				StackedResult::ExtractionFailed(err) => {
					emit_extraction_failed_event(&err);
					let reason = RejectionReason::KeyExtractionFailed(err);
					return Ok(build_reject_no_headers(cfg, reason));
				}
				StackedResult::Reject { wait, key_repr } => {
					let quota = entry.quota();
					let name = entry.name_arc();
					emit_reject_event(&key_repr, quota.inner().burst_size().get(), wait.as_millis(), &name);
					let reason = RejectionReason::QuotaExceeded {
						wait,
						snapshot: synthetic_snapshot(quota),
						key: Box::new(()) as Box<dyn std::any::Any + Send>,
						policy_name: Arc::clone(&name),
					};
					return Ok(build_reject_response(cfg, quota, wait, reason, &name, policy_entry, &shared));
				}
				StackedResult::Admit { remaining } => {
					if remaining < lowest_remaining {
						lowest_remaining = remaining;
					}
				}
			}
		}

		if dispatch.is_none() && shared.stack.is_empty() {
			let req = Request::from_parts(parts, body);
			return inner.call(req).await;
		}

		let (admit_quota, admit_name) = if let Some(q) = active_quota {
			(q, Arc::clone(&shared.default_label))
		} else if let Some(entry) = policy_entry.as_ref().filter(|e| !e.descriptors.is_empty()) {
			let first = &entry.descriptors[0];
			(first.quota, Arc::clone(&first.name))
		} else {
			let req = Request::from_parts(parts, body);
			return inner.call(req).await;
		};

		let final_remaining = if lowest_remaining == u32::MAX { 0 } else { lowest_remaining };
		let headers = build_admit_headers(
			cfg.legacy_reset_epoch,
			admit_quota,
			&admit_name,
			final_remaining,
			policy_entry,
			&shared,
			outcome.quota_override,
		);
		let req = Request::from_parts(parts, body);
		let mut resp = inner.call(req).await?;
		for (name, value) in headers {
			if let Some(n) = name {
				resp.headers_mut().insert(n, value);
			}
		}
		Ok(resp)
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

// ---------------------------------------------------------------------------
// Helper types and functions
// ---------------------------------------------------------------------------

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
	policy_label: &'a str,
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
		if let Some(tracker) = self.tracker {
			let touch = tracker.touch(key);
			if touch.reason == Some(EvictionReason::MaxKeys) {
				emit_eviction_warn(self.policy_label);
				self.limiter.as_ref().retain_recent();
			}
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
		shared.config.quota_methods.iter().find(|(m, _)| m == method).map(|(_, q)| *q)
	{
		let limiter = shared
			.method_limiters
			.iter()
			.find(|(m, _)| m == method)
			.map(|(_, l)| l)
			.expect("method_limiters and quota_methods are built in parallel");
		let tracker = shared.method_trackers.iter().find(|(m, _)| m == method).map(|(_, t)| t);
		return Some(Dispatch {
			limiter: LimiterRef::Borrowed(limiter),
			tracker,
			policy_label: &shared.default_label,
			quota: method_quota,
		});
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
			policy_label: &shared.default_label,
			quota: override_quota,
		});
	}

	if let Some(limiter) = shared.default_limiter.as_ref() {
		let q = shared.config.quota_default.expect("default_limiter present => quota_default Some");
		return Some(Dispatch {
			limiter: LimiterRef::Borrowed(limiter),
			tracker: shared.default_tracker.as_ref(),
			policy_label: &shared.default_label,
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

fn peer_ip_from(parts: &http::request::Parts) -> Option<std::net::IpAddr> {
	parts.extensions.get::<axum::extract::ConnectInfo<SocketAddr>>().map(|ci| ci.0.ip())
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
	cfg: &crate::builder::GovernorConfig<K>,
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

// ---------------------------------------------------------------------------
// GovernorFuture constructors and Future impl
// ---------------------------------------------------------------------------

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
	use std::convert::Infallible;
	use std::net::SocketAddr;

	use axum::extract::ConnectInfo;
	use http::{Method, Request, Response, StatusCode, header::AUTHORIZATION};
	use ipnet::IpNet;
	use tower::ServiceExt as _;

	use tower::Layer as _;

	use super::*;
	use crate::builder::GovernorConfigBuilder;
	use crate::extractor::{
		AsyncExtractFuture, AsyncKeyExtractor, Global, Header, KeyOutcome, PeerIp,
	};
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

	// Stage 6b tests

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
		let svc = layer.layer(ok_inner());

		// First GET passes.
		let r1 = svc.clone().oneshot(req(Method::GET, "/")).await.unwrap();
		assert_eq!(r1.status(), StatusCode::OK);

		// Second GET is rejected.
		let r2 = svc.clone().oneshot(req(Method::GET, "/")).await.unwrap();
		assert_eq!(r2.status(), StatusCode::TOO_MANY_REQUESTS);

		// POST uses default quota (100/s) — passes.
		let r3 = svc.clone().oneshot(req(Method::POST, "/")).await.unwrap();
		assert_eq!(r3.status(), StatusCode::OK);

		// GET still rejected.
		let r4 = svc.clone().oneshot(req(Method::GET, "/")).await.unwrap();
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
		let svc = layer.layer(ok_inner());

		// First request passes.
		let r1 = svc.clone().oneshot(req(Method::GET, "/")).await.unwrap();
		assert_eq!(r1.status(), StatusCode::OK);

		// Second: peer (1/s) exhausted — 429.
		let r2 = svc.clone().oneshot(req(Method::GET, "/")).await.unwrap();
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
		let svc = layer.layer(ok_inner());

		// First request passes.
		let r1 = svc.clone().oneshot(req(Method::GET, "/")).await.unwrap();
		assert_eq!(r1.status(), StatusCode::OK);

		// Second request: peer:1s exhausted → 429.
		let r2 = svc.clone().oneshot(req(Method::GET, "/")).await.unwrap();
		assert_eq!(r2.status(), StatusCode::TOO_MANY_REQUESTS);

		// Policy header lists both windows.
		let policy = r2.headers().get("ratelimit-policy").unwrap().to_str().unwrap();
		assert!(policy.contains("\"peer:1s\""), "expected peer:1s in policy, got: {policy}");
		assert!(policy.contains("\"peer:1m\""), "expected peer:1m in policy, got: {policy}");
	}

	#[tokio::test]
	async fn per_tier_quota_override_admits_burst() {
		use crate::extractor::KeyExtractor;
		use http::request::Parts;

		// Extractor returns quota_override = Some(100/s), bypassing the default 1/s.
		#[derive(Clone)]
		struct TierExtractor;
		impl KeyExtractor for TierExtractor {
			type Key = ();
			fn extract(&self, _parts: &Parts) -> Result<KeyOutcome<()>, crate::ExtractionError> {
				Ok(KeyOutcome { key: (), quota_override: Some(Quota::requests_per_second(nz!(100u32))) })
			}
		}

		let cfg = GovernorConfigBuilder::default()
			.with_extractor(TierExtractor)
			// Default very tight; override allows more.
			.quota_default(Quota::requests_per_second(nz!(1u32)))
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);
		let svc = layer.layer(ok_inner());

		// With default 1/s the second would fail; with override 100/s it passes.
		let r1 = svc.clone().oneshot(req(Method::GET, "/")).await.unwrap();
		assert_eq!(r1.status(), StatusCode::OK);
		let r2 = svc.clone().oneshot(req(Method::GET, "/")).await.unwrap();
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
		let svc = layer.layer(ok_inner());

		// First request: 200.
		let r1 = svc.clone().oneshot(req(Method::GET, "/")).await.unwrap();
		assert_eq!(r1.status(), StatusCode::OK);

		// Second request: 429.
		let r2 = svc.clone().oneshot(req(Method::GET, "/")).await.unwrap();
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
		let svc = layer.layer(ok_inner());

		let make_req = || {
			Request::builder()
				.method(Method::GET)
				.uri("/")
				.header(AUTHORIZATION, "Bearer token123")
				.body(axum::body::Body::empty())
				.unwrap()
		};

		// First request passes.
		let r1 = svc.clone().oneshot(make_req()).await.unwrap();
		assert_eq!(r1.status(), StatusCode::OK);

		// Second request: auth (1/s) exhausted → 429 naming "auth".
		let r2 = svc.clone().oneshot(make_req()).await.unwrap();
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
		let svc = layer.layer(ok_inner());

		for ip in ["1.1.1.1:1", "2.2.2.2:1", "3.3.3.3:1"] {
			let r = svc.clone().oneshot(req_with_peer(Method::GET, "/", ip)).await.unwrap();
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

	#[cfg(feature = "tracing")]
	#[tokio::test]
	async fn tracing_smoke_compiles_and_runs() {
		let cfg = GovernorConfigBuilder::default()
			.with_extractor(Global)
			.quota_default(Quota::requests_per_second(nz!(1u32)))
			.finish()
			.unwrap();
		let layer = GovernorLayer::new(cfg);
		let svc = layer.layer(ok_inner());
		let _ = svc.clone().oneshot(req(Method::GET, "/")).await;
		let _ = svc.clone().oneshot(req(Method::GET, "/")).await;
	}
}
