//! Compound combinator that chains two extractors into a tuple key.

use http::request::Parts;

use super::{ExtractionError, KeyExtractor, KeyOutcome};

/// Combines two extractors into a single `(A::Key, B::Key)` key.
///
/// If either extractor fails, extraction short-circuits with its error.
/// `A`'s `quota_override` takes precedence over `B`'s.
#[derive(Clone, Copy, Debug)]
pub struct Compound<A, B>(pub A, pub B);

impl<A, B> KeyExtractor for Compound<A, B>
where
	A: KeyExtractor,
	B: KeyExtractor,
{
	type Key = (A::Key, B::Key);

	fn extract(&self, parts: &Parts) -> Result<KeyOutcome<Self::Key>, ExtractionError> {
		let a = self.0.extract(parts)?;
		let b = self.1.extract(parts)?;
		Ok(KeyOutcome { key: (a.key, b.key), quota_override: a.quota_override.or(b.quota_override) })
	}
}

#[cfg(test)]
mod tests {
	use http::Request;
	use http::header::{AUTHORIZATION, CONTENT_TYPE};

	use super::*;
	use crate::error::ExtractionError;
	use crate::extractor::Header;

	#[test]
	fn both_succeed_returns_tuple_key() {
		let req = Request::builder()
			.header("authorization", "Bearer token")
			.header("content-type", "application/json")
			.body(())
			.unwrap();
		let (parts, _) = req.into_parts();
		let outcome = Compound(Header(&AUTHORIZATION), Header(&CONTENT_TYPE)).extract(&parts).unwrap();
		assert_eq!(outcome.key, ("Bearer token".to_owned(), "application/json".to_owned()));
	}

	#[test]
	fn a_fails_returns_a_error() {
		let (parts, _) = Request::new(()).into_parts();
		assert!(matches!(
			Compound(Header(&AUTHORIZATION), Header(&CONTENT_TYPE)).extract(&parts),
			Err(ExtractionError::MissingHeader(_))
		));
	}

	#[test]
	fn b_fails_when_a_succeeds_returns_b_error() {
		let req = Request::builder().header("authorization", "Bearer token").body(()).unwrap();
		let (parts, _) = req.into_parts();
		assert!(matches!(
			Compound(Header(&AUTHORIZATION), Header(&CONTENT_TYPE)).extract(&parts),
			Err(ExtractionError::MissingHeader(_))
		));
	}
}
