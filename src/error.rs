//! Error and reason types shared across the middleware.

use std::time::Duration;

/// Failure modes when extracting a rate-limit key from a request.
#[derive(Debug)]
pub enum ExtractionError {
	MissingConnectInfo,
	MissingHeader(&'static str),
	MalformedHeader(&'static str),
	UntrustedProxy,
	Other(Box<dyn std::error::Error + Send + Sync>),
}

impl std::fmt::Display for ExtractionError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::MissingConnectInfo => write!(f, "connect info extension is absent"),
			Self::MissingHeader(name) => write!(f, "required header '{}' is missing", name),
			Self::MalformedHeader(name) => write!(f, "header '{}' contains invalid data", name),
			Self::UntrustedProxy => write!(f, "request originated from an untrusted proxy"),
			Self::Other(e) => e.fmt(f),
		}
	}
}

impl std::error::Error for ExtractionError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::Other(e) => Some(e.as_ref()),
			_ => None,
		}
	}
}

/// Failure modes surfaced by `GovernorConfigBuilder::finish`.
#[derive(Debug)]
pub enum ConfigError {
	ZeroBurst,
	EmptyChain,
	ContradictoryWhitelist,
	NoExtractor,
	MissingConnectInfoAcknowledgement,
}

impl std::fmt::Display for ConfigError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::ZeroBurst => write!(f, "burst capacity must be non-zero"),
			Self::EmptyChain => write!(f, "stacked limiter chain has no entries"),
			Self::ContradictoryWhitelist => {
				write!(f, "whitelist contradicts the configured extractor")
			}
			Self::NoExtractor => write!(f, "no key extractor was configured"),
			Self::MissingConnectInfoAcknowledgement => {
				write!(f, "PeerIp or SmartIp requires expect_connect_info() before finish()")
			}
		}
	}
}

impl std::error::Error for ConfigError {}

/// The reason the middleware rejected or could not process a request.
///
/// Passed to `error_handler` so callers can distinguish quota failure from extraction failure
/// without status-code sniffing.
#[derive(Debug)]
pub enum RejectionReason {
	QuotaExceeded {
		wait: Duration,
		snapshot: governor::middleware::StateSnapshot,
		key: Box<dyn std::any::Any + Send>,
		policy_name: &'static str,
	},
	KeyExtractionFailed(ExtractionError),
}
