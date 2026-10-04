//! Where something is, answered from files on disk rather than by asking a service.
//!
//! Two lookups, each behind the feature of its name:
//!
//! - `coordinates`: a latitude and longitude to the place it is in, from GeoNames' `cities500`
//!   and the files beside it -- see [`coordinates::Gazetteer`].
//! - `ip`: an IP address to its approximate location and network, from MaxMind's GeoLite2 City
//!   and ASN databases -- see [`ip::Databases`].
//!
//! Offline on purpose: a lookup service would make naming a place depend on somebody else's
//! uptime, their rate limit and their terms. The data directory is the caller's, and so is
//! fetching what goes in it; this crate only reads it.

#[cfg(feature = "coordinates")]
pub mod coordinates;
#[cfg(feature = "ip")]
pub mod ip;
