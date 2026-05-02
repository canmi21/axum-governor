//! Key extraction traits and built-in extractor implementations.

use core::future::Future;
use core::hash::Hash;
use core::pin::Pin;

use http::request::Parts;

use crate::error::ExtractionError;

/// Carrier returned by a successful extraction: the rate-limit key plus an
/// optional per-request quota override for tiered-plan support.
#[derive(Clone, Debug)]
pub struct KeyOutcome<K> {
	pub key: K,
	pub quota_override: Option<crate::Quota>,
}

/// Synchronous key extractor. Object-safe; the layer holds
/// `Arc<dyn KeyExtractor<Key = K>>`.
pub trait KeyExtractor: Send + Sync + 'static {
	type Key: Hash + Eq + Clone + Send + Sync + 'static;
	fn extract(&self, parts: &Parts) -> Result<KeyOutcome<Self::Key>, ExtractionError>;
}

/// Return type of [`AsyncKeyExtractor::extract`].
pub type AsyncExtractFuture<'a, K> =
	Pin<Box<dyn Future<Output = Result<KeyOutcome<K>, ExtractionError>> + Send + 'a>>;

/// Async sibling for extractors that genuinely need to await
/// (e.g. database tier lookup). Hand-rolled `Pin<Box<dyn Future>>` so the
/// trait stays dyn-compatible without pulling in `async-trait`.
pub trait AsyncKeyExtractor: Send + Sync + 'static {
	type Key: Hash + Eq + Clone + Send + Sync + 'static;
	fn extract<'a>(&'a self, parts: &'a Parts) -> AsyncExtractFuture<'a, Self::Key>;
}

mod compound;
mod cookie;
mod extension;
mod global;
mod header;
mod peer_ip;
mod smart_ip;

pub use compound::Compound;
pub use cookie::Cookie;
pub use extension::Extension;
pub use global::Global;
pub use header::Header;
pub use peer_ip::PeerIp;
pub use smart_ip::SmartIp;
