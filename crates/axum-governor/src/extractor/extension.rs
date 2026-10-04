//! Request-extension key extractor.

use core::hash::Hash;
use core::marker::PhantomData;

use http::request::Parts;

use super::{ExtractionError, KeyExtractor, KeyOutcome};

/// Extracts a typed value from request extensions as the rate-limit key.
///
/// The conventional shape for "upstream auth has inserted a `UserId`; rate-limit by that".
pub struct Extension<T>(PhantomData<fn() -> T>);

impl<T> Extension<T> {
	/// Create a new `Extension` extractor.
	pub const fn new() -> Self {
		Self(PhantomData)
	}
}

impl<T> Default for Extension<T> {
	fn default() -> Self {
		Self::new()
	}
}

// PhantomData<fn() -> T> is Clone/Copy/Debug regardless of T's own auto-trait status.
impl<T> Clone for Extension<T> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<T> Copy for Extension<T> {}

impl<T> std::fmt::Debug for Extension<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("Extension").finish()
	}
}

impl<T: Clone + Hash + Eq + std::fmt::Debug + Send + Sync + 'static> KeyExtractor for Extension<T> {
	type Key = T;

	fn extract(&self, parts: &Parts) -> Result<KeyOutcome<Self::Key>, ExtractionError> {
		let key = parts
			.extensions
			.get::<T>()
			.cloned()
			// consider promoting MissingExtension to a first-class ExtractionError variant
			// when the upstream error type design is settled
			.ok_or_else(|| {
				ExtractionError::Other(Box::new(MissingExtension {
					type_name: core::any::type_name::<T>(),
				}))
			})?;
		Ok(KeyOutcome { key, quota_override: None })
	}
}

#[derive(Debug)]
struct MissingExtension {
	type_name: &'static str,
}

impl std::fmt::Display for MissingExtension {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "extension '{}' not found in request", self.type_name)
	}
}

impl std::error::Error for MissingExtension {}

#[cfg(test)]
mod tests {
	use http::Request;

	use super::*;

	#[test]
	fn present_extension_returns_value() {
		let mut req = Request::new(());
		req.extensions_mut().insert(String::from("alice"));
		let (parts, _) = req.into_parts();
		assert_eq!(Extension::<String>::new().extract(&parts).unwrap().key, "alice");
	}

	#[test]
	fn absent_extension_returns_other() {
		let (parts, _) = Request::new(()).into_parts();
		assert!(matches!(Extension::<String>::new().extract(&parts), Err(ExtractionError::Other(_))));
	}
}
