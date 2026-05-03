//! Pure header writers for rate-limit response headers.

// Functions in this module are called by the Service layer (Stage 6); suppress
// dead_code until that wiring exists.
#![allow(dead_code)]

use http::header::{HeaderMap, HeaderName, HeaderValue};

use crate::Quota;

/// One advertised rate-limit policy (name + quota), used to fill `RateLimit-Policy:`.
#[derive(Clone, Copy, Debug)]
pub struct PolicyDescriptor {
	pub name: &'static str,
	pub quota: Quota,
}

pub(crate) fn write_retry_after(headers: &mut HeaderMap, delta_seconds: u64) {
	headers.insert(
		http::header::RETRY_AFTER,
		HeaderValue::from_str(&delta_seconds.to_string())
			.expect("decimal integer is a valid header value"),
	);
}

pub(crate) fn write_ietf_rate_limit(
	headers: &mut HeaderMap,
	policy_name: &'static str,
	remaining: u32,
	t_seconds: u64,
) {
	let value = format!("\"{policy_name}\";r={remaining};t={t_seconds}");
	headers.insert(
		HeaderName::from_static("ratelimit"),
		HeaderValue::from_str(&value).expect("ratelimit header value is always valid"),
	);
}

pub(crate) fn write_ietf_policy_set(headers: &mut HeaderMap, policies: &[PolicyDescriptor]) {
	if policies.is_empty() {
		return;
	}
	let parts: Vec<String> = policies
		.iter()
		.map(|p| {
			let q = p.quota.inner().burst_size().get();
			let w = quota_window_seconds(&p.quota);
			format!("\"{}\";q={};w={}", p.name, q, w)
		})
		.collect();
	let value = parts.join(", ");
	headers.insert(
		HeaderName::from_static("ratelimit-policy"),
		HeaderValue::from_str(&value).expect("ratelimit-policy header value is always valid"),
	);
}

pub(crate) fn write_legacy_rate_limit(
	headers: &mut HeaderMap,
	limit: u32,
	remaining: u32,
	reset_seconds: u64,
) {
	headers.insert(
		HeaderName::from_static("x-ratelimit-limit"),
		HeaderValue::from_str(&limit.to_string()).expect("x-ratelimit-limit value is always valid"),
	);
	headers.insert(
		HeaderName::from_static("x-ratelimit-remaining"),
		HeaderValue::from_str(&remaining.to_string())
			.expect("x-ratelimit-remaining value is always valid"),
	);
	headers.insert(
		HeaderName::from_static("x-ratelimit-reset"),
		HeaderValue::from_str(&reset_seconds.to_string())
			.expect("x-ratelimit-reset value is always valid"),
	);
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
