//! An IP address to its location and network, from GeoLite2.
//!
//! MaxMind's license asks anything built on GeoLite2 to say so where it shows the data; that is
//! the caller's to do, since only the caller shows anything.

use maxminddb::{Mmap, Reader, geoip2};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::path::Path;

pub use maxminddb::MaxMindDbError as Error;

/// The City database's file name in a data directory.
pub const CITY_FILE: &str = "GeoLite2-City.mmdb";
/// The ASN database's file name in a data directory.
pub const ASN_FILE: &str = "GeoLite2-ASN.mmdb";

/// An address's country, region, city, approximate position, time zone and network: nulls for
/// what the data does not know, rather than fields left out.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Location {
	pub country_code: Option<String>,
	pub country: Option<String>,
	pub region: Option<String>,
	pub city: Option<String>,
	pub latitude: Option<f64>,
	pub longitude: Option<f64>,
	pub timezone: Option<String>,
	pub asn: Option<u32>,
	pub organization: Option<String>,
}

impl Location {
	fn of(city: &geoip2::City, asn: &geoip2::Asn) -> Self {
		Self {
			country_code: city.country.iso_code.map(str::to_owned),
			country: city.country.names.english.map(str::to_owned),
			region: city.subdivisions.first().and_then(|s| s.names.english).map(str::to_owned),
			city: city.city.names.english.map(str::to_owned),
			latitude: city.location.latitude,
			longitude: city.location.longitude,
			timezone: city.location.time_zone.map(str::to_owned),
			asn: asn.autonomous_system_number,
			organization: asn.autonomous_system_organization.map(str::to_owned),
		}
	}
}

/// `geoip2::Asn` derives no `Default`, unlike every other `geoip2` record, so an address the ASN
/// database has nothing for is built by hand instead.
fn empty_asn() -> geoip2::Asn<'static> {
	geoip2::Asn { autonomous_system_number: None, autonomous_system_organization: None }
}

/// The City and ASN databases, read together.
pub struct Databases<S: AsRef<[u8]> = Mmap> {
	city: Reader<S>,
	asn: Reader<S>,
}

impl Databases<Mmap> {
	/// Map [`CITY_FILE`] and [`ASN_FILE`] in `dir`.
	///
	/// # Safety
	///
	/// The files are memory-mapped: one changed in place while mapped is undefined behavior. Put a
	/// new file in place by renaming over the old one, which leaves the mapped one intact.
	pub unsafe fn open(dir: &Path) -> Result<Self, Error> {
		// SAFETY: the caller upholds the contract above.
		let city = unsafe { Reader::open_mmap(dir.join(CITY_FILE)) }?;
		let asn = unsafe { Reader::open_mmap(dir.join(ASN_FILE)) }?;
		Ok(Self { city, asn })
	}
}

impl<S: AsRef<[u8]>> Databases<S> {
	/// From databases already read, by whatever means the caller chose.
	pub fn from_bytes(city: S, asn: S) -> Result<Self, Error> {
		Ok(Self { city: Reader::from_source(city)?, asn: Reader::from_source(asn)? })
	}

	/// Where `address` is. Either database missing the address, or holding nothing useful for
	/// it, leaves that half null; an address the data does not know at all is every field null.
	pub fn lookup(&self, address: IpAddr) -> Location {
		let city: geoip2::City = self
			.city
			.lookup(address)
			.ok()
			.and_then(|result| result.decode::<geoip2::City>().ok())
			.flatten()
			.unwrap_or_default();
		let asn: geoip2::Asn = self
			.asn
			.lookup(address)
			.ok()
			.and_then(|result| result.decode::<geoip2::Asn>().ok())
			.flatten()
			.unwrap_or(empty_asn());
		Location::of(&city, &asn)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use maxminddb::geoip2::{Asn, City, Names, city};

	/// A decoded record built by hand, standing in for one borrowed from a real `.mmdb` file: the
	/// shape the mapping is held to.
	fn city_record() -> City<'static> {
		City {
			city: city::City {
				geoname_id: None,
				names: Names { english: Some("Reykjavik"), ..Default::default() },
			},
			country: city::Country {
				iso_code: Some("IS"),
				names: Names { english: Some("Iceland"), ..Default::default() },
				..Default::default()
			},
			subdivisions: vec![city::Subdivision {
				names: Names { english: Some("Capital Region"), ..Default::default() },
				..Default::default()
			}],
			location: city::Location {
				latitude: Some(64.15),
				longitude: Some(-21.95),
				time_zone: Some("Atlantic/Reykjavik"),
				..Default::default()
			},
			..Default::default()
		}
	}

	fn asn_record() -> Asn<'static> {
		Asn {
			autonomous_system_number: Some(13335),
			autonomous_system_organization: Some("Cloudflare, Inc."),
		}
	}

	#[test]
	fn maps_a_full_record_into_the_location() {
		assert_eq!(
			Location::of(&city_record(), &asn_record()),
			Location {
				country_code: Some("IS".into()),
				country: Some("Iceland".into()),
				region: Some("Capital Region".into()),
				city: Some("Reykjavik".into()),
				latitude: Some(64.15),
				longitude: Some(-21.95),
				timezone: Some("Atlantic/Reykjavik".into()),
				asn: Some(13335),
				organization: Some("Cloudflare, Inc.".into()),
			}
		);
	}

	#[test]
	fn an_empty_record_is_every_field_null() {
		assert_eq!(Location::of(&City::default(), &empty_asn()), Location::default());
	}

	#[test]
	fn a_directory_with_nothing_in_it_fails_rather_than_panics() {
		assert!(unsafe { Databases::open(Path::new("/nowhere-at-all")) }.is_err());
		assert!(Databases::from_bytes(Vec::new(), Vec::new()).is_err());
	}
}
