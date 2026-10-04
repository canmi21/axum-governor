//! Peer IP extractor with configurable IPv6 prefix masking.

use std::net::IpAddr;

use http::request::Parts;

use super::ip::{apply_prefix, peer_ip};
use super::{ExtractionError, KeyExtractor, KeyOutcome};

/// Extracts the peer IP from `ConnectInfo<SocketAddr>`.
///
/// IPv6 addresses are masked to a configurable prefix (default /56) so that
/// addresses within a rotation budget share a bucket.
#[derive(Clone, Copy, Debug)]
pub struct PeerIp {
	ipv6_prefix: u8,
}

impl Default for PeerIp {
	fn default() -> Self {
		Self { ipv6_prefix: 56 }
	}
}

impl PeerIp {
	/// Create a `PeerIp` that masks IPv6 addresses to the given prefix; clamped to 128.
	pub const fn ipv6_prefix(prefix: u8) -> Self {
		let ipv6_prefix = if prefix > 128 { 128 } else { prefix };
		Self { ipv6_prefix }
	}
}

impl KeyExtractor for PeerIp {
	type Key = IpAddr;

	fn requires_connect_info(&self) -> bool {
		true
	}

	fn extract(&self, parts: &Parts) -> Result<KeyOutcome<Self::Key>, ExtractionError> {
		let peer = peer_ip(parts).ok_or(ExtractionError::MissingConnectInfo)?;
		Ok(KeyOutcome { key: apply_prefix(peer, self.ipv6_prefix), quota_override: None })
	}
}

#[cfg(test)]
mod tests {
	use std::net::IpAddr;

	use http::Request;

	use super::*;
	use crate::extractor::ip::test_support::parts_with_peer;

	#[test]
	fn ipv4_peer_returned_unchanged() {
		let parts = parts_with_peer("1.2.3.4:0");
		let key = PeerIp::default().extract(&parts).unwrap().key;
		assert_eq!(key, "1.2.3.4".parse::<IpAddr>().unwrap());
	}

	#[test]
	fn absent_peer_returns_missing_connect_info() {
		let (parts, _) = Request::new(()).into_parts();
		assert!(matches!(PeerIp::default().extract(&parts), Err(ExtractionError::MissingConnectInfo)));
	}

	#[test]
	fn default_prefix_is_56() {
		let parts = parts_with_peer("[2001:db8:1234:5678:9abc:def0:1234:5678]:0");
		let key = PeerIp::default().extract(&parts).unwrap().key;
		assert_eq!(key, "2001:db8:1234:5600::".parse::<IpAddr>().unwrap());
	}

	#[test]
	fn explicit_prefix_is_applied() {
		let parts = parts_with_peer("[2001:db8:1234:5678:9abc:def0:1234:5678]:0");
		let key = PeerIp::ipv6_prefix(64).extract(&parts).unwrap().key;
		assert_eq!(key, "2001:db8:1234:5678::".parse::<IpAddr>().unwrap());
	}
}
