//! Default 429 response bodies and the `BodyPreset` enum.

// Functions in this module are called by the Service layer (Stage 6); suppress
// dead_code until that wiring exists.
#![allow(dead_code)]

use http::{HeaderValue, StatusCode};

use crate::error::{ExtractionError, RejectionReason};

/// Default body format selected on the builder.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BodyPreset {
	/// `text/plain; charset=utf-8` — the default.
	#[default]
	Text,
	/// `application/json` — `{"error":"...","retry_after_seconds":N}`.
	#[cfg(feature = "json")]
	Json,
	/// `application/problem+json` per RFC 9457.
	#[cfg(feature = "json")]
	ProblemJson,
}

/// User-supplied response builder; overrides the default body.
///
/// Wrapped in `Arc` so the cloned `Service` shares one handler.
pub type ErrorHandler = std::sync::Arc<
	dyn Fn(crate::RejectionReason) -> http::Response<axum::body::Body> + Send + Sync + 'static,
>;

pub(crate) fn default_status(reason: &RejectionReason) -> StatusCode {
	match reason {
		RejectionReason::QuotaExceeded { .. } => StatusCode::TOO_MANY_REQUESTS,
		RejectionReason::KeyExtractionFailed(err) => match err {
			ExtractionError::MissingHeader(_)
			| ExtractionError::MalformedHeader(_)
			| ExtractionError::UntrustedProxy => StatusCode::BAD_REQUEST,
			ExtractionError::MissingConnectInfo | ExtractionError::Other(_) => {
				StatusCode::INTERNAL_SERVER_ERROR
			}
		},
	}
}

pub(crate) fn default_body(preset: BodyPreset, reason: &RejectionReason) -> (HeaderValue, Vec<u8>) {
	match preset {
		BodyPreset::Text => text_body(reason),
		#[cfg(feature = "json")]
		BodyPreset::Json => json_body(reason),
		#[cfg(feature = "json")]
		BodyPreset::ProblemJson => problem_json_body(reason),
	}
}

fn text_body(reason: &RejectionReason) -> (HeaderValue, Vec<u8>) {
	let ct = HeaderValue::from_static("text/plain; charset=utf-8");
	let body = match reason {
		RejectionReason::QuotaExceeded { wait, .. } => {
			let secs = wait.as_secs().max(1);
			format!("Too Many Requests, retry in {secs}s").into_bytes()
		}
		RejectionReason::KeyExtractionFailed(err) => {
			let detail = err.to_string();
			if default_status(reason) == StatusCode::BAD_REQUEST {
				format!("Bad Request: {detail}").into_bytes()
			} else {
				format!("Internal Server Error: {detail}").into_bytes()
			}
		}
	};
	(ct, body)
}

#[cfg(feature = "json")]
fn json_body(reason: &RejectionReason) -> (HeaderValue, Vec<u8>) {
	let ct = HeaderValue::from_static("application/json");
	let body = match reason {
		RejectionReason::QuotaExceeded { wait, .. } => {
			let secs = wait.as_secs().max(1);
			let v = serde_json::json!({
				"error": "too_many_requests",
				"retry_after_seconds": secs
			});
			serde_json::to_vec(&v).expect("static JSON shape serializes")
		}
		RejectionReason::KeyExtractionFailed(err) => {
			let detail = err.to_string();
			let error_code = if default_status(reason) == StatusCode::BAD_REQUEST {
				"bad_request"
			} else {
				"internal_server_error"
			};
			let v = serde_json::json!({ "error": error_code, "detail": detail });
			serde_json::to_vec(&v).expect("static JSON shape serializes")
		}
	};
	(ct, body)
}

#[cfg(feature = "json")]
fn problem_json_body(reason: &RejectionReason) -> (HeaderValue, Vec<u8>) {
	let ct = HeaderValue::from_static("application/problem+json");
	let body = match reason {
		RejectionReason::QuotaExceeded { wait, .. } => {
			let secs = wait.as_secs().max(1);
			let v = serde_json::json!({
				"type": "about:blank",
				"title": "Too Many Requests",
				"status": 429u16,
				"detail": format!("Rate limit exceeded; retry in {} seconds", secs)
			});
			serde_json::to_vec(&v).expect("static JSON shape serializes")
		}
		RejectionReason::KeyExtractionFailed(err) => {
			let detail = err.to_string();
			let status = default_status(reason);
			let v = serde_json::json!({
				"type": "about:blank",
				"title": status.canonical_reason().unwrap_or("Error"),
				"status": status.as_u16(),
				"detail": detail
			});
			serde_json::to_vec(&v).expect("static JSON shape serializes")
		}
	};
	(ct, body)
}

#[cfg(test)]
mod tests {
	use std::time::Duration;

	use super::*;
	use crate::{ExtractionError, RejectionReason};

	fn fake_snapshot() -> governor::middleware::StateSnapshot {
		use governor::middleware::StateInformationMiddleware;
		use nonzero_ext::nonzero;
		let lim = governor::RateLimiter::direct(governor::Quota::per_second(nonzero!(1u32)))
			.with_middleware::<StateInformationMiddleware>();
		lim.check().unwrap()
	}

	fn quota_exceeded_5s() -> RejectionReason {
		RejectionReason::QuotaExceeded {
			wait: Duration::from_secs(5),
			snapshot: fake_snapshot(),
			key: Box::new(()) as Box<dyn std::any::Any + Send>,
			policy_name: "default",
		}
	}

	#[test]
	fn status_quota_exceeded() {
		assert_eq!(default_status(&quota_exceeded_5s()), StatusCode::TOO_MANY_REQUESTS);
	}

	#[test]
	fn status_missing_header() {
		let r = RejectionReason::KeyExtractionFailed(ExtractionError::MissingHeader("x-api-key"));
		assert_eq!(default_status(&r), StatusCode::BAD_REQUEST);
	}

	#[test]
	fn status_malformed_header() {
		let r = RejectionReason::KeyExtractionFailed(ExtractionError::MalformedHeader("x-api-key"));
		assert_eq!(default_status(&r), StatusCode::BAD_REQUEST);
	}

	#[test]
	fn status_missing_connect_info() {
		let r = RejectionReason::KeyExtractionFailed(ExtractionError::MissingConnectInfo);
		assert_eq!(default_status(&r), StatusCode::INTERNAL_SERVER_ERROR);
	}

	#[test]
	fn status_untrusted_proxy() {
		let r = RejectionReason::KeyExtractionFailed(ExtractionError::UntrustedProxy);
		assert_eq!(default_status(&r), StatusCode::BAD_REQUEST);
	}

	#[test]
	fn status_other() {
		let r = RejectionReason::KeyExtractionFailed(ExtractionError::Other(Box::new(
			std::io::Error::other("oops"),
		)));
		assert_eq!(default_status(&r), StatusCode::INTERNAL_SERVER_ERROR);
	}

	#[test]
	fn text_body_quota_exceeded() {
		let (ct, body) = default_body(BodyPreset::Text, &quota_exceeded_5s());
		assert_eq!(ct, "text/plain; charset=utf-8");
		assert_eq!(body, b"Too Many Requests, retry in 5s");
	}

	#[test]
	fn text_body_key_extraction_400() {
		let r = RejectionReason::KeyExtractionFailed(ExtractionError::MissingHeader("x-api-key"));
		let (ct, body) = default_body(BodyPreset::Text, &r);
		assert_eq!(ct, "text/plain; charset=utf-8");
		assert_eq!(body, b"Bad Request: required header 'x-api-key' is missing");
	}

	#[test]
	fn text_body_key_extraction_500() {
		let r = RejectionReason::KeyExtractionFailed(ExtractionError::MissingConnectInfo);
		let (ct, body) = default_body(BodyPreset::Text, &r);
		assert_eq!(ct, "text/plain; charset=utf-8");
		assert_eq!(body, b"Internal Server Error: connect info extension is absent");
	}

	#[cfg(feature = "json")]
	#[test]
	fn json_body_quota_exceeded() {
		let (ct, body) = default_body(BodyPreset::Json, &quota_exceeded_5s());
		assert_eq!(ct, "application/json");
		let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
		assert_eq!(v["error"], "too_many_requests");
		assert_eq!(v["retry_after_seconds"], 5);
	}

	#[cfg(feature = "json")]
	#[test]
	fn json_body_key_extraction_400() {
		let r = RejectionReason::KeyExtractionFailed(ExtractionError::MissingHeader("x-api-key"));
		let (ct, body) = default_body(BodyPreset::Json, &r);
		assert_eq!(ct, "application/json");
		let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
		assert_eq!(v["error"], "bad_request");
		assert!(v["detail"].is_string());
	}

	#[cfg(feature = "json")]
	#[test]
	fn json_body_key_extraction_500() {
		let r = RejectionReason::KeyExtractionFailed(ExtractionError::MissingConnectInfo);
		let (ct, body) = default_body(BodyPreset::Json, &r);
		assert_eq!(ct, "application/json");
		let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
		assert_eq!(v["error"], "internal_server_error");
		assert!(v["detail"].is_string());
	}

	#[cfg(feature = "json")]
	#[test]
	fn problem_json_body_quota_exceeded() {
		let (ct, body) = default_body(BodyPreset::ProblemJson, &quota_exceeded_5s());
		assert_eq!(ct, "application/problem+json");
		let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
		assert_eq!(v["type"], "about:blank");
		assert_eq!(v["title"], "Too Many Requests");
		assert_eq!(v["status"], 429);
		assert!(v["detail"].as_str().unwrap().contains("5"));
	}

	#[cfg(feature = "json")]
	#[test]
	fn problem_json_body_key_extraction_400() {
		let r = RejectionReason::KeyExtractionFailed(ExtractionError::MissingHeader("x-api-key"));
		let (ct, body) = default_body(BodyPreset::ProblemJson, &r);
		assert_eq!(ct, "application/problem+json");
		let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
		assert_eq!(v["status"], 400);
		assert_eq!(v["title"], "Bad Request");
	}

	#[cfg(feature = "json")]
	#[test]
	fn problem_json_body_key_extraction_500() {
		let r = RejectionReason::KeyExtractionFailed(ExtractionError::MissingConnectInfo);
		let (ct, body) = default_body(BodyPreset::ProblemJson, &r);
		assert_eq!(ct, "application/problem+json");
		let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
		assert_eq!(v["status"], 500);
		assert_eq!(v["title"], "Internal Server Error");
	}
}
