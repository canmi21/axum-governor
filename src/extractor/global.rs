//! Single-bucket extractor that puts all traffic into one rate-limit pool.

use http::request::Parts;

use super::{ExtractionError, KeyExtractor, KeyOutcome};

/// Extracts a unit key — every request shares the same rate-limit bucket.
#[derive(Clone, Copy, Debug, Default)]
pub struct Global;

impl KeyExtractor for Global {
	type Key = ();

	fn extract(&self, _parts: &Parts) -> Result<KeyOutcome<Self::Key>, ExtractionError> {
		Ok(KeyOutcome { key: (), quota_override: None })
	}
}

#[cfg(test)]
mod tests {
	use http::Request;

	use super::*;

	#[test]
	fn extract_returns_unit_key() {
		let (parts, _) = Request::new(()).into_parts();
		assert!(Global.extract(&parts).is_ok());
	}
}
