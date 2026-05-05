//! Pure header writers for rate-limit response headers.
//!
//! Two design notes worth preserving:
//!
//! 1. `PolicyDescriptor` borrows its `name`. Stack entries can carry dynamic labels
//!    (e.g. `peer:1s`), so we can no longer pin them to `&'static str` without
//!    leaking. The lifetime is tied to the layer, which the request future holds via
//!    `Arc<LimiterShared>` — so per-request descriptors borrow without overhead.
//! 2. Integer-only header values use `HeaderValue::from::<u32>` / `From::<u64>`,
//!    which itoa-format into a small buffer. Compared to `format!("{n}")` +
//!    `from_str` this avoids both the formatter and a `String` allocation.

use std::fmt::Write as _;

use http::header::{HeaderMap, HeaderName, HeaderValue};

use crate::Quota;

/// One advertised rate-limit policy (name + quota). Stored in headers and used to
/// fill `RateLimit-Policy:`.
#[derive(Clone, Copy, Debug)]
pub struct PolicyDescriptor<'a> {
	pub name: &'a str,
	pub quota: Quota,
}

pub(crate) fn write_retry_after(headers: &mut HeaderMap, delta_seconds: u64) {
	headers.insert(http::header::RETRY_AFTER, HeaderValue::from(delta_seconds));
}

pub(crate) fn write_ietf_rate_limit(
	headers: &mut HeaderMap,
	policy_name: &str,
	remaining: u32,
	t_seconds: u64,
) {
	let mut buf = String::with_capacity(policy_name.len() + 24);
	buf.push('"');
	buf.push_str(policy_name);
	buf.push('"');
	buf.push_str(";r=");
	let _ = write!(&mut buf, "{remaining}");
	buf.push_str(";t=");
	let _ = write!(&mut buf, "{t_seconds}");
	headers.insert(
		HeaderName::from_static("ratelimit"),
		HeaderValue::from_str(&buf).expect("ratelimit header value is always valid"),
	);
}

/// Render and insert a `RateLimit-Policy` header from descriptors. Currently only
/// exercised in tests — the hot path uses `render_policy_value` once at layer
/// construction and then clones the cached `HeaderValue`.
#[cfg(test)]
pub(crate) fn write_ietf_policy_set(headers: &mut HeaderMap, policies: &[PolicyDescriptor<'_>]) {
	if policies.is_empty() {
		return;
	}
	let value = render_policy_set(policies);
	headers.insert(
		HeaderName::from_static("ratelimit-policy"),
		HeaderValue::from_str(&value).expect("ratelimit-policy header value is always valid"),
	);
}

/// Pre-render a policy set to a `HeaderValue`. Layer construction calls this once
/// per static configuration so the hot path can `.clone()` the resulting
/// `HeaderValue` (cheap — internally a `Bytes`-backed buffer) instead of rebuilding
/// the string per request.
pub(crate) fn render_policy_value(policies: &[PolicyDescriptor<'_>]) -> Option<HeaderValue> {
	if policies.is_empty() {
		return None;
	}
	let s = render_policy_set(policies);
	Some(HeaderValue::from_str(&s).expect("ratelimit-policy header value is always valid"))
}

fn render_policy_set(policies: &[PolicyDescriptor<'_>]) -> String {
	// Estimate: each entry is `"name";q=NNNN;w=NNNN, ` ≈ name.len() + 16. Pre-size to
	// avoid the realloc churn that a naive `String::new()` gets on a 3-stack policy.
	let cap: usize = policies.iter().map(|p| p.name.len() + 16).sum::<usize>() + 4;
	let mut buf = String::with_capacity(cap);
	let mut first = true;
	for p in policies {
		if !first {
			buf.push_str(", ");
		}
		first = false;
		let q = p.quota.inner().burst_size().get();
		let w = quota_window_seconds(&p.quota);
		buf.push('"');
		buf.push_str(p.name);
		buf.push('"');
		buf.push_str(";q=");
		let _ = write!(&mut buf, "{q}");
		buf.push_str(";w=");
		let _ = write!(&mut buf, "{w}");
	}
	buf
}

pub(crate) fn write_legacy_rate_limit(
	headers: &mut HeaderMap,
	limit: u32,
	remaining: u32,
	reset_seconds: u64,
) {
	headers.insert(HeaderName::from_static("x-ratelimit-limit"), HeaderValue::from(limit));
	headers.insert(HeaderName::from_static("x-ratelimit-remaining"), HeaderValue::from(remaining));
	headers.insert(HeaderName::from_static("x-ratelimit-reset"), HeaderValue::from(reset_seconds));
}

/// `w=` value for IETF policy: `(burst * replenish_interval)` in whole seconds, min 1.
pub(crate) fn quota_window_seconds(q: &Quota) -> u64 {
	let inner = q.inner();
	let total_nanos = inner.burst_size().get() as u128 * inner.replenish_interval().as_nanos();
	let secs = (total_nanos / 1_000_000_000) as u64;
	secs.max(1)
}

#[cfg(test)]
mod tests {
	use http::HeaderMap;

	use super::*;
	use crate::nz;

	#[test]
	fn write_retry_after_emits_delta_seconds() {
		let mut headers = HeaderMap::new();
		write_retry_after(&mut headers, 5);
		assert_eq!(headers["retry-after"], "5");
	}

	#[test]
	fn write_ietf_rate_limit_format() {
		let mut headers = HeaderMap::new();
		write_ietf_rate_limit(&mut headers, "default", 0, 5);
		assert_eq!(headers["ratelimit"], "\"default\";r=0;t=5");
	}

	#[test]
	fn write_ietf_policy_set_one_policy() {
		let mut headers = HeaderMap::new();
		write_ietf_policy_set(
			&mut headers,
			&[PolicyDescriptor { name: "peer", quota: crate::Quota::requests_per_second(nz!(10u32)) }],
		);
		assert_eq!(headers["ratelimit-policy"], "\"peer\";q=10;w=1");
	}

	#[test]
	fn write_ietf_policy_set_two_policies() {
		let mut headers = HeaderMap::new();
		write_ietf_policy_set(
			&mut headers,
			&[
				PolicyDescriptor { name: "peer", quota: crate::Quota::requests_per_second(nz!(10u32)) },
				PolicyDescriptor { name: "auth", quota: crate::Quota::requests_per_minute(nz!(600u32)) },
			],
		);
		assert_eq!(headers["ratelimit-policy"], "\"peer\";q=10;w=1, \"auth\";q=600;w=60");
	}

	#[test]
	fn write_ietf_policy_set_empty_writes_nothing() {
		let mut headers = HeaderMap::new();
		write_ietf_policy_set(&mut headers, &[]);
		assert!(headers.get("ratelimit-policy").is_none());
	}

	#[test]
	fn render_policy_value_round_trips_through_set_writer() {
		let policies = [
			PolicyDescriptor { name: "peer", quota: crate::Quota::requests_per_second(nz!(10u32)) },
			PolicyDescriptor { name: "auth", quota: crate::Quota::requests_per_minute(nz!(600u32)) },
		];
		let pre = render_policy_value(&policies).expect("non-empty policies render to a value");
		let mut headers = HeaderMap::new();
		write_ietf_policy_set(&mut headers, &policies);
		assert_eq!(headers["ratelimit-policy"], pre);
	}

	#[test]
	fn render_policy_value_empty_returns_none() {
		assert!(render_policy_value(&[]).is_none());
	}

	#[test]
	fn write_legacy_rate_limit_three_headers() {
		let mut headers = HeaderMap::new();
		write_legacy_rate_limit(&mut headers, 100, 0, 5);
		assert_eq!(headers["x-ratelimit-limit"], "100");
		assert_eq!(headers["x-ratelimit-remaining"], "0");
		assert_eq!(headers["x-ratelimit-reset"], "5");
	}

	#[test]
	fn quota_window_seconds_per_second() {
		assert_eq!(quota_window_seconds(&crate::Quota::requests_per_second(nz!(50u32))), 1);
	}

	#[test]
	fn quota_window_seconds_per_minute() {
		assert_eq!(quota_window_seconds(&crate::Quota::requests_per_minute(nz!(60u32))), 60);
	}

	#[test]
	fn quota_window_seconds_per_hour() {
		assert_eq!(quota_window_seconds(&crate::Quota::requests_per_hour(nz!(3600u32))), 3600);
	}

	#[test]
	fn quota_window_seconds_burst() {
		assert_eq!(
			quota_window_seconds(&crate::Quota::requests_per_second(nz!(1u32)).burst(nz!(1000u32))),
			1000
		);
	}
}
