//! SmartIp extractor: header-walk with trusted-proxy gating.

use std::net::{IpAddr, Ipv6Addr, SocketAddr};

use axum::extract::ConnectInfo;
use forwarded_header_value::ForwardedHeaderValue;
use http::request::Parts;
use ipnet::IpNet;

use super::{ExtractionError, KeyExtractor, KeyOutcome};

/// Extracts the originating IP by walking `X-Forwarded-For` → `X-Real-IP` →
/// `Forwarded` → peer, gated by a trusted-proxy CIDR allowlist.
///
/// With an empty trust list the peer IP is always used and headers are ignored.
#[derive(Clone, Debug)]
pub struct SmartIp {
	trusted: Vec<IpNet>,
	ipv6_prefix: u8,
}

impl Default for SmartIp {
	fn default() -> Self {
		Self::new()
	}
}

impl SmartIp {
	/// Create a `SmartIp` with no trusted proxies and the default /56 IPv6 prefix.
	pub fn new() -> Self {
		Self { trusted: vec![], ipv6_prefix: 56 }
	}

	/// Append trusted-proxy CIDRs; chainable.
	pub fn with_trusted_proxies(mut self, nets: impl IntoIterator<Item = IpNet>) -> Self {
		self.trusted.extend(nets);
		self
	}

	/// Override the IPv6 masking prefix; clamped to 128.
	pub fn ipv6_prefix(mut self, prefix: u8) -> Self {
		self.ipv6_prefix = prefix.min(128);
		self
	}
}

impl KeyExtractor for SmartIp {
	type Key = IpAddr;

	fn extract(&self, parts: &Parts) -> Result<KeyOutcome<Self::Key>, ExtractionError> {
		let peer = parts
			.extensions
			.get::<ConnectInfo<SocketAddr>>()
			.map(|ci| ci.0.ip())
			.ok_or(ExtractionError::MissingConnectInfo)?;

		if self.trusted.is_empty() {
			return Ok(KeyOutcome { key: apply_prefix(peer, self.ipv6_prefix), quota_override: None });
		}

		let peer_trusted = self.trusted.iter().any(|net| net.contains(&peer));

		let has_xff = parts.headers.contains_key("x-forwarded-for");
		let has_real_ip = parts.headers.contains_key("x-real-ip");
		let has_forwarded = parts.headers.contains_key("forwarded");

		if !peer_trusted && (has_xff || has_real_ip || has_forwarded) {
			return Err(ExtractionError::UntrustedProxy);
		}

		let selected = if peer_trusted {
			self.try_xff(parts).or_else(|| self.try_real_ip(parts)).or_else(|| self.try_forwarded(parts))
		} else {
			None
		};

		let ip = apply_prefix(selected.unwrap_or(peer), self.ipv6_prefix);
		Ok(KeyOutcome { key: ip, quota_override: None })
	}
}

impl SmartIp {
	fn is_trusted(&self, ip: &IpAddr) -> bool {
		self.trusted.iter().any(|net| net.contains(ip))
	}

	fn try_xff(&self, parts: &Parts) -> Option<IpAddr> {
		let mut last_untrusted: Option<IpAddr> = None;
		for value in parts.headers.get_all("x-forwarded-for") {
			let Ok(s) = value.to_str() else {
				continue;
			};
			for entry in s.split(',') {
				if let Ok(ip) = entry.trim().parse::<IpAddr>()
					&& !self.is_trusted(&ip)
				{
					last_untrusted = Some(ip);
				}
			}
		}
		last_untrusted
	}

	fn try_real_ip(&self, parts: &Parts) -> Option<IpAddr> {
		let value = parts.headers.get("x-real-ip")?;
		let ip: IpAddr = value.to_str().ok()?.trim().parse().ok()?;
		(!self.is_trusted(&ip)).then_some(ip)
	}

	fn try_forwarded(&self, parts: &Parts) -> Option<IpAddr> {
		for value in parts.headers.get_all("forwarded") {
			let Ok(s) = value.to_str() else {
				continue;
			};
			let Ok(fhv) = ForwardedHeaderValue::from_forwarded(s) else {
				continue;
			};
			if let Some(ip) = fhv.iter().find_map(|stanza| {
				let ip = stanza.forwarded_for_ip()?;
				(!self.is_trusted(&ip)).then_some(ip)
			}) {
				return Some(ip);
			}
		}
		None
	}
}

fn apply_prefix(ip: IpAddr, prefix: u8) -> IpAddr {
	match ip {
		IpAddr::V4(_) => ip,
		IpAddr::V6(v6) => IpAddr::V6(mask_ipv6(v6, prefix)),
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
	use std::net::{IpAddr, SocketAddr};
	use std::str::FromStr;

	use axum::extract::ConnectInfo;
	use http::Request;
	use ipnet::IpNet;

	use super::*;

	fn parts_with_peer(peer: &str) -> http::request::Parts {
		let addr: SocketAddr = peer.parse().unwrap();
		let mut req = Request::new(());
		req.extensions_mut().insert(ConnectInfo::<SocketAddr>(addr));
		req.into_parts().0
	}

	fn parts_with_peer_and_header(peer: &str, name: &str, value: &str) -> http::request::Parts {
		let addr: SocketAddr = peer.parse().unwrap();
		let req = Request::builder().header(name, value).body(()).unwrap();
		let (mut parts, _) = req.into_parts();
		parts.extensions.insert(ConnectInfo::<SocketAddr>(addr));
		parts
	}

	fn net(s: &str) -> IpNet {
		IpNet::from_str(s).unwrap()
	}

	#[test]
	fn empty_trust_list_uses_peer_ignores_headers() {
		let parts = parts_with_peer_and_header("8.8.8.8:0", "x-forwarded-for", "1.2.3.4");
		let key = SmartIp::new().extract(&parts).unwrap().key;
		assert_eq!(key, "8.8.8.8".parse::<IpAddr>().unwrap());
	}

	#[test]
	fn peer_in_trusted_xff_single_entry_returned() {
		let parts = parts_with_peer_and_header("10.0.0.1:0", "x-forwarded-for", "8.8.8.8");
		let key = SmartIp::new().with_trusted_proxies([net("10.0.0.0/8")]).extract(&parts).unwrap().key;
		assert_eq!(key, "8.8.8.8".parse::<IpAddr>().unwrap());
	}

	#[test]
	fn peer_in_trusted_xff_last_untrusted_returned() {
		let parts = parts_with_peer_and_header("10.0.0.1:0", "x-forwarded-for", "8.8.8.8, 10.0.0.2");
		let key = SmartIp::new().with_trusted_proxies([net("10.0.0.0/8")]).extract(&parts).unwrap().key;
		assert_eq!(key, "8.8.8.8".parse::<IpAddr>().unwrap());
	}

	#[test]
	fn peer_in_trusted_real_ip_used_when_no_xff() {
		let parts = parts_with_peer_and_header("10.0.0.1:0", "x-real-ip", "8.8.8.8");
		let key = SmartIp::new().with_trusted_proxies([net("10.0.0.0/8")]).extract(&parts).unwrap().key;
		assert_eq!(key, "8.8.8.8".parse::<IpAddr>().unwrap());
	}

	#[test]
	fn untrusted_peer_with_xff_returns_untrusted_proxy() {
		let parts = parts_with_peer_and_header("8.8.8.8:0", "x-forwarded-for", "1.2.3.4");
		assert!(matches!(
			SmartIp::new().with_trusted_proxies([net("10.0.0.0/8")]).extract(&parts),
			Err(ExtractionError::UntrustedProxy)
		));
	}

	#[test]
	fn trusted_non_empty_no_connect_info_returns_missing() {
		let (parts, _) = Request::new(()).into_parts();
		assert!(matches!(
			SmartIp::new().with_trusted_proxies([net("10.0.0.0/8")]).extract(&parts),
			Err(ExtractionError::MissingConnectInfo)
		));
	}

	#[test]
	fn ipv6_peer_masked_to_default_prefix() {
		let parts = parts_with_peer("[2001:db8::1]:0");
		let key = SmartIp::new().extract(&parts).unwrap().key;
		let expected: IpAddr = "2001:db8::".parse().unwrap();
		assert_eq!(key, expected);
	}
}
