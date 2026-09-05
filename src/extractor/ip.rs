//! IP helpers shared by the address-based extractors and the whitelist check.

use std::net::{IpAddr, Ipv6Addr, SocketAddr};

use axum::extract::ConnectInfo;
use http::request::Parts;

/// Peer address from `ConnectInfo<SocketAddr>`, when the router was built with
/// `into_make_service_with_connect_info`.
pub(crate) fn peer_ip(parts: &Parts) -> Option<IpAddr> {
	parts.extensions.get::<ConnectInfo<SocketAddr>>().map(|ci| ci.0.ip())
}

/// Mask an IPv6 address to `prefix` bits so addresses inside one rotation budget share a
/// bucket; IPv4 passes through untouched.
pub(crate) fn apply_prefix(ip: IpAddr, prefix: u8) -> IpAddr {
	match ip {
		IpAddr::V4(_) => ip,
		IpAddr::V6(v6) => IpAddr::V6(mask_ipv6(v6, prefix)),
	}
}

fn mask_ipv6(addr: Ipv6Addr, prefix: u8) -> Ipv6Addr {
	let mask = match prefix.min(128) {
		0 => 0u128,
		128 => u128::MAX,
		p => !((1u128 << (128 - p)) - 1),
	};
	Ipv6Addr::from(u128::from(addr) & mask)
}

#[cfg(test)]
pub(crate) mod test_support {
	use super::*;
	use http::Request;

	pub(crate) fn parts_with_peer(peer: &str) -> Parts {
		let addr: SocketAddr = peer.parse().unwrap();
		let mut req = Request::new(());
		req.extensions_mut().insert(ConnectInfo::<SocketAddr>(addr));
		req.into_parts().0
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn v6(s: &str) -> IpAddr {
		s.parse().unwrap()
	}

	#[test]
	fn prefix_56_clears_the_host_bits() {
		let full = v6("2001:db8:1234:5678:9abc:def0:1234:5678");
		assert_eq!(apply_prefix(full, 56), v6("2001:db8:1234:5600::"));
	}

	#[test]
	fn prefix_0_and_128_are_the_two_extremes() {
		let full = v6("2001:db8:1234:5678:9abc:def0:1234:5678");
		assert_eq!(apply_prefix(full, 0), IpAddr::V6(Ipv6Addr::UNSPECIFIED));
		assert_eq!(apply_prefix(full, 128), full);
	}

	#[test]
	fn prefix_above_128_is_clamped() {
		let full = v6("2001:db8::1");
		assert_eq!(apply_prefix(full, 200), full);
	}

	#[test]
	fn ipv4_is_untouched() {
		let v4: IpAddr = "1.2.3.4".parse().unwrap();
		assert_eq!(apply_prefix(v4, 0), v4);
	}

	#[test]
	fn peer_ip_absent_without_connect_info() {
		let (parts, _) = http::Request::new(()).into_parts();
		assert!(peer_ip(&parts).is_none());
	}
}
