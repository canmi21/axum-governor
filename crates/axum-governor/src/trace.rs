//! Every tracing call site, with a no-op twin behind `not(feature = "tracing")` so the
//! rest of the crate never carries a cfg.

#[cfg(feature = "tracing")]
pub(crate) fn span_for(method: &http::Method, path: &str) -> tracing::Span {
	tracing::debug_span!(target: "axum_governor::layer",
		"axum_governor::layer", method = %method, path = %path)
}

#[cfg(feature = "tracing")]
pub(crate) fn reject(key_str: &str, quota_burst: u32, wait_ms: u128, policy: &str) {
	tracing::info!(target: "axum_governor",
		key = %key_str, quota_burst, wait_ms, policy,
		"rate limit exceeded");
}

#[cfg(not(feature = "tracing"))]
pub(crate) fn reject(_key_str: &str, _quota_burst: u32, _wait_ms: u128, _policy: &str) {}

#[cfg(feature = "tracing")]
pub(crate) fn extraction_failed(reason: &crate::ExtractionError) {
	tracing::warn!(target: "axum_governor", error = %reason,
		"rate-limit key extraction failed");
}

#[cfg(not(feature = "tracing"))]
pub(crate) fn extraction_failed(_reason: &crate::ExtractionError) {}

#[cfg(feature = "tracing")]
pub(crate) fn eviction(policy: &str) {
	tracing::warn!(target: "axum_governor", policy = %policy,
		"max_keys exceeded; evicted oldest key from tracker and forced retain_recent");
}

#[cfg(not(feature = "tracing"))]
pub(crate) fn eviction(_policy: &str) {}
