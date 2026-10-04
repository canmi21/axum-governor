//! The shape every API here answers in, from Rust and from TypeScript alike: `src/index.ts` is the
//! other half, and both read `codes.json` and are tested against `src/fixtures.json`. See
//! spec/architecture/services.md, "Every answer is one envelope".

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// Success carries what the route answers; failure carries a code for a program and a message for
/// a person.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum Envelope<T> {
	Success { data: T },
	Error { code: String, message: String },
}

fn catalogue() -> &'static BTreeMap<String, String> {
	static CODES: OnceLock<BTreeMap<String, String>> = OnceLock::new();
	CODES.get_or_init(|| {
		serde_json::from_str(include_str!("../../../pkgs/response/codes.json")).unwrap_or_default()
	})
}

/// The message a code carries when the moment has nothing more exact to say.
pub fn message_of(code: &str) -> Option<&'static str> {
	catalogue().get(code).map(String::as_str)
}

/// Every code a Rust source names beside a status -- `StatusCode::X, "code"` -- so a program's
/// tests can hold what it answers with to the catalogue, which TypeScript does with a type.
pub fn codes_named(source: &str) -> Vec<&str> {
	source
		.split("StatusCode::")
		.skip(1)
		.filter_map(|after| {
			let rest = after.trim_start_matches(|c: char| c.is_ascii_uppercase() || c == '_');
			let rest = rest.strip_prefix(',')?.trim_start().strip_prefix('"')?;
			rest.split('"').next()
		})
		.filter(|named| !named.is_empty() && named.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'))
		.collect()
}

impl Envelope<()> {
	/// A failure with the code's own message. A code missing from the catalogue answers with the
	/// code itself, and the tests hold every code the programs use to the catalogue.
	pub fn error(code: &str) -> Self {
		Self::error_with(code, message_of(code).unwrap_or(code))
	}

	pub fn error_with(code: &str, message: impl Into<String>) -> Self {
		Self::Error { code: code.into(), message: message.into() }
	}
}

#[cfg(feature = "axum")]
mod web {
	use super::Envelope;
	use axum::Json;
	use axum::http::StatusCode;
	use axum::response::{IntoResponse, Response};
	use serde::Serialize;

	pub fn success<T: Serialize>(status: StatusCode, data: T) -> Response {
		(status, Json(Envelope::Success { data })).into_response()
	}

	pub fn failure(status: StatusCode, code: &str) -> Response {
		(status, Json(Envelope::error(code))).into_response()
	}

	pub fn failure_with(status: StatusCode, code: &str, message: impl ToString) -> Response {
		(status, Json(Envelope::error_with(code, message.to_string()))).into_response()
	}
}

#[cfg(feature = "axum")]
pub use web::{failure, failure_with, success};

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::{Value, json};

	fn fixtures() -> Value {
		serde_json::from_str(include_str!("../../../pkgs/response/src/fixtures.json")).unwrap()
	}

	#[test]
	fn reads_and_writes_the_shape_typescript_does() {
		let fixtures = fixtures();
		let success: Envelope<Value> = serde_json::from_value(fixtures["success"].clone()).unwrap();
		assert_eq!(success, Envelope::Success { data: json!({ "count": 3 }) });
		assert_eq!(
			serde_json::to_value(Envelope::error("no_such_route")).unwrap(),
			fixtures["failure"]
		);
	}

	#[test]
	fn finds_the_codes_a_source_names() {
		let source = r#"failure(StatusCode::NOT_FOUND, "no_such_place"), Reply(StatusCode::BAD_GATEWAY,
			"app_unavailable", e), (StatusCode::OK, Json(x)), StatusCode::OK, assert_eq!(s,
			StatusCode::OK, "{path}")"#;
		assert_eq!(codes_named(source), ["no_such_place", "app_unavailable"]);
	}

	#[test]
	fn every_code_has_a_message() {
		assert!(catalogue().len() > 10);
		assert_eq!(message_of("no_such_scope"), Some("No API is published under this scope"));
		assert_eq!(message_of("not_a_code"), None);
	}
}
