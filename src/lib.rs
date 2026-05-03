//! Rate limiting middleware for axum, powered by governor.
//!
//! See `spec/` in the repository for architecture documentation.

#![forbid(unsafe_code)]

pub mod builder;
pub mod error;
pub mod extractor;
pub mod gc;
mod glob;
pub mod headers;
pub mod layer;
pub mod quota;
pub mod response;
pub mod service;
pub mod snapshot;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

pub use crate::builder::{GovernorConfig, GovernorConfigBuilder};
pub use crate::error::{ConfigError, ExtractionError, RejectionReason};
pub use crate::extractor::{
	AsyncKeyExtractor, Compound, Cookie, Extension, Global, Header, KeyExtractor, KeyOutcome, PeerIp,
	SmartIp,
};
pub use crate::layer::GovernorLayer;
pub use crate::quota::{Quota, nz};
pub use crate::response::{BodyPreset, ErrorHandler};

#[cfg(test)]
mod smoke {
	use super::{ConfigError, ExtractionError, RejectionReason};

	#[test]
	fn error_variants_are_nameable() {
		let _: ExtractionError = ExtractionError::MissingConnectInfo;
		let _: ConfigError = ConfigError::NoExtractor;
	}

	#[test]
	fn rejection_reason_variant_is_nameable() {
		let _: RejectionReason = RejectionReason::KeyExtractionFailed(ExtractionError::UntrustedProxy);
	}
}
