//! Peer IP extractor with configurable IPv6 prefix masking.

use std::net::{IpAddr, Ipv6Addr, SocketAddr};

use axum::extract::ConnectInfo;
use http::request::Parts;

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
		let peer = parts
			.extensions
			.get::<ConnectInfo<SocketAddr>>()
			.map(|ci| ci.0.ip())
			.ok_or(ExtractionError::MissingConnectInfo)?;

		let key = match peer {
			IpAddr::V4(_) => peer,
			IpAddr::V6(v6) => IpAddr::V6(mask_ipv6(v6, self.ipv6_prefix)),
		};
		Ok(KeyOutcome { key, quota_override: None })
	}
}

fn mask_ipv6(addr: Ipv6Addr, prefix: u8) -> Ipv6Addr {
	let prefix = prefix.min(128);
	let bits = u128::from(addr);
	let mask = if prefix == 0 {
		0u128
	} else if prefix == 128 {
		u128::MAX
	} else {
		!((1u128 << (128 - prefix)) - 1)
	};
	Ipv6Addr::from(bits & mask)
}

#[cfg(test)]
mod tests {
	use std::net::{IpAddr, Ipv6Addr, SocketAddr};

	use axum::extract::ConnectInfo;
	use http::Request;

	use super::*;

	fn parts_with_peer(peer: &str) -> http::request::Parts {
		let addr: SocketAddr = peer.parse().unwrap();
		let mut req = Request::new(());
		req.extensions_mut().insert(ConnectInfo::<SocketAddr>(addr));
		req.into_parts().0
	}

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
	fn default_prefix_56_masks_correctly() {
		let parts = parts_with_peer("[2001:db8:1234:5678:9abc:def0:1234:5678]:0");
		let key = PeerIp::default().extract(&parts).unwrap().key;
		let expected: IpAddr = "2001:db8:1234:5600::".parse().unwrap();
		assert_eq!(key, expected);
	}

	#[test]
	fn prefix_64_masks_correctly() {
		let parts = parts_with_peer("[2001:db8:1234:5678:9abc:def0:1234:5678]:0");
		let key = PeerIp::ipv6_prefix(64).extract(&parts).unwrap().key;
		let expected: IpAddr = "2001:db8:1234:5678::".parse().unwrap();
		assert_eq!(key, expected);
	}

	#[test]
	fn prefix_0_masks_to_unspecified() {
		let parts = parts_with_peer("[2001:db8:1234:5678:9abc:def0:1234:5678]:0");
		let key = PeerIp::ipv6_prefix(0).extract(&parts).unwrap().key;
		assert_eq!(key, IpAddr::V6(Ipv6Addr::UNSPECIFIED));
	}

	#[test]
	fn prefix_128_returns_address_unchanged() {
		let parts = parts_with_peer("[2001:db8:1234:5678:9abc:def0:1234:5678]:0");
		let key = PeerIp::ipv6_prefix(128).extract(&parts).unwrap().key;
		let expected: IpAddr = "2001:db8:1234:5678:9abc:def0:1234:5678".parse().unwrap();
		assert_eq!(key, expected);
	}
}
