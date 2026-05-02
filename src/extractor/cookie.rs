//! Cookie-value key extractor.

use http::request::Parts;

use super::{ExtractionError, KeyExtractor, KeyOutcome};

/// Extracts a named cookie value as the rate-limit key.
///
/// Values are returned as-is (no URL-decoding); cookies are treated as opaque tokens.
#[derive(Clone, Copy, Debug)]
pub struct Cookie(pub &'static str);

impl KeyExtractor for Cookie {
	type Key = String;

	fn extract(&self, parts: &Parts) -> Result<KeyOutcome<Self::Key>, ExtractionError> {
		for header_value in parts.headers.get_all(http::header::COOKIE) {
			let Ok(s) = header_value.to_str() else {
				continue;
			};
			for pair in s.split(';') {
				let pair = pair.trim();
				if let Some((name, value)) = pair.split_once('=')
					&& name.trim() == self.0
				{
					return Ok(KeyOutcome { key: value.trim().to_owned(), quota_override: None });
				}
			}
		}
		// consider promoting MissingCookie to a first-class ExtractionError variant when the
		// upstream error type design is settled
		Err(ExtractionError::Other(Box::new(MissingCookie(self.0))))
	}
}

#[derive(Debug)]
struct MissingCookie(&'static str);

impl std::fmt::Display for MissingCookie {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "cookie '{}' not found in request", self.0)
	}
}

impl std::error::Error for MissingCookie {}

#[cfg(test)]
mod tests {
	use http::Request;

	use super::*;

	fn parts_with_cookie(value: &str) -> http::request::Parts {
		Request::builder().header("cookie", value).body(()).unwrap().into_parts().0
	}

	#[test]
	fn present_cookie_returns_value() {
		let parts = parts_with_cookie("session=abc123");
		assert_eq!(Cookie("session").extract(&parts).unwrap().key, "abc123");
	}

	#[test]
	fn quoted_value_preserved() {
		let parts = parts_with_cookie("token=\"v\"");
		assert_eq!(Cookie("token").extract(&parts).unwrap().key, "\"v\"");
	}

	#[test]
	fn second_cookie_in_header_matched() {
		let parts = parts_with_cookie("foo=bar; session=abc123");
		assert_eq!(Cookie("session").extract(&parts).unwrap().key, "abc123");
	}

	#[test]
	fn absent_cookie_returns_other() {
		let (parts, _) = Request::new(()).into_parts();
		assert!(matches!(Cookie("session").extract(&parts), Err(ExtractionError::Other(_))));
	}
}
