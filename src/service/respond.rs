//! The headers an admitted request carries out and the response a rejected one gets.

use std::hash::Hash;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use http::{HeaderMap, HeaderName, Response};

use crate::Quota;
use crate::builder::Settings;
use crate::error::RejectionReason;
use crate::headers::{
	PolicyDescriptor, render_policy_value, write_ietf_rate_limit, write_legacy_rate_limit,
	write_retry_after,
};
use crate::layer::{LimiterShared, PolicyEntry};
use crate::response::{default_body, default_status};

pub(super) fn build_admit_headers<K>(
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

pub(super) fn build_reject_response<K>(
	cfg: &Settings,
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

pub(super) fn build_base_response(
	cfg: &Settings,
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
