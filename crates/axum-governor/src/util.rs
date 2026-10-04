//! Internal utility functions shared across modules.

/// Format a rate-limit key for logging.
///
/// When `redact` is `false`, the key's `Debug` representation is returned.
/// When `redact` is `true`, the key is hashed with a random SipHash state and
/// returned as `hash:<16-hex-digits>` so that sensitive values (API keys,
/// session tokens) do not appear verbatim in tracing events.
pub(crate) fn format_key<K: std::hash::Hash + std::fmt::Debug>(key: &K, redact: bool) -> String {
	if redact {
		use std::hash::BuildHasher;
		let state = std::collections::hash_map::RandomState::new();
		format!("hash:{:016x}", state.hash_one(key))
	} else {
		format!("{key:?}")
	}
}
