//! Header-value key extractor.

use http::HeaderName;
use http::request::Parts;

use super::{ExtractionError, KeyExtractor, KeyOutcome};

/// Extracts a named header value as the rate-limit key.
///
/// Missing headers return `MissingHeader`; non-UTF-8 values return `MalformedHeader`.
#[derive(Clone, Copy, Debug)]
pub struct Header(pub &'static HeaderName);

impl KeyExtractor for Header {
	type Key = String;

	fn extract(&self, parts: &Parts) -> Result<KeyOutcome<Self::Key>, ExtractionError> {
		// as_str() on &'static HeaderName yields &'static str, matching the variant's lifetime.
		let name: &'static str = self.0.as_str();
		let value = parts.headers.get(self.0).ok_or(ExtractionError::MissingHeader(name))?;
		let key = value.to_str().map_err(|_| ExtractionError::MalformedHeader(name))?.to_owned();
		Ok(KeyOutcome { key, quota_override: None })
	}
}

#[cfg(test)]
mod tests {
	use http::Request;
	use http::header::AUTHORIZATION;

	use super::*;

	#[test]
	fn present_header_returns_value() {
		let req = Request::builder().header("authorization", "Bearer token123").body(()).unwrap();
		let (parts, _) = req.into_parts();
		assert_eq!(Header(&AUTHORIZATION).extract(&parts).unwrap().key, "Bearer token123");
	}

	#[test]
	fn absent_header_returns_missing() {
		let (parts, _) = Request::new(()).into_parts();
		assert!(matches!(
			Header(&AUTHORIZATION).extract(&parts),
			Err(ExtractionError::MissingHeader(_))
		));
	}

	#[test]
	fn non_utf8_header_returns_malformed() {
		let value = http::HeaderValue::from_bytes(b"\x80\x81").unwrap();
		let mut req = Request::new(());
		req.headers_mut().insert(AUTHORIZATION, value);
		let (parts, _) = req.into_parts();
		assert!(matches!(
			Header(&AUTHORIZATION).extract(&parts),
			Err(ExtractionError::MalformedHeader(_))
		));
	}
}
