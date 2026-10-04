//! Turning a pair of coordinates into a place name, without asking anyone.
//!
//! GeoNames' `cities500`, indexed once into an R-tree and searched for the nearest settlement.
//! Offline on purpose: a reverse geocoding service would make naming a place depend on somebody
//! else's uptime, their rate limit and their opinion about what we may do with the answer -- for a
//! fact that never changes once written. See spec/architecture/media.md.
//!
//! Two consumers: `local`, naming where a photograph was taken, and the `geo` service, answering
//! the same question over HTTP. The data directory is the caller's to know.

use rstar::{AABB, PointDistance, RTree, RTreeObject};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Where a position is, as far as the data can say.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Address {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub continent: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub country: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub country_code: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub region: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub subregion: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub city: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub district: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub postal_code: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub timezone: Option<String>,
}

/// One settlement, as much of it as the address needs.
#[derive(Debug, Clone)]
struct Place {
	lat: f64,
	lon: f64,
	name: String,
	country: String,
	admin1: String,
	admin2: String,
}

/// One postal area, by the point GeoNames gives for it.
///
/// A code is not unique on its own -- 27707 is a district of Eumseong and also a part of
/// Durham -- so these are found by position and never by string.
#[derive(Debug, Clone)]
struct Postal {
	lat: f64,
	lon: f64,
	code: String,
	country: String,
}

impl RTreeObject for Place {
	type Envelope = AABB<[f64; 2]>;
	fn envelope(&self) -> Self::Envelope {
		AABB::from_point([self.lon, self.lat])
	}
}

impl RTreeObject for Postal {
	type Envelope = AABB<[f64; 2]>;
	fn envelope(&self) -> Self::Envelope {
		AABB::from_point([self.lon, self.lat])
	}
}

impl PointDistance for Postal {
	fn distance_2(&self, point: &[f64; 2]) -> f64 {
		let dx = self.lon - point[0];
		let dy = self.lat - point[1];
		dx * dx + dy * dy
	}
}

impl PointDistance for Place {
	/// Squared degrees, which is wrong as a distance and right as an ordering.
	///
	/// A degree of longitude is shorter near the poles, so this is not metres and must never
	/// be reported as one. It only ever decides which of two candidates is closer, and for
	/// that the distortion has to be extreme before it changes the answer -- the nearest town
	/// to a photograph is rarely a close call between two on opposite bearings.
	fn distance_2(&self, point: &[f64; 2]) -> f64 {
		let dx = self.lon - point[0];
		let dy = self.lat - point[1];
		dx * dx + dy * dy
	}
}

pub struct Gazetteer {
	tree: RTree<Place>,
	/// ISO country code to (name, continent).
	countries: HashMap<String, (String, String)>,
	/// `US.NC` to the region's name.
	regions: HashMap<String, String>,
	/// `US.NC.183` to the county's name.
	subregions: HashMap<String, String>,
	/// Postal areas, built the first time a photograph turns out to have a position.
	///
	/// Fourteen seconds for 1.8 million points, and most imports are screenshots with no GPS
	/// at all. Paying that before knowing whether anything will ask is the cost of a guess.
	postal: OnceLock<Option<RTree<Postal>>>,
	root: PathBuf,
	finder: tzf_rs::DefaultFinder,
}

fn continent_of(code: &str) -> &'static str {
	match code {
		"AF" => "Africa",
		"AS" => "Asia",
		"EU" => "Europe",
		"NA" => "North America",
		"OC" => "Oceania",
		"SA" => "South America",
		"AN" => "Antarctica",
		_ => "",
	}
}

impl Gazetteer {
	/// Read the gazetteer from its data directory, or `None` when it has not been fetched.
	pub fn open(root: &Path) -> Option<Self> {
		let root_for_later = root.to_path_buf();
		let cities = std::fs::read_to_string(root.join("cities500.txt")).ok()?;

		let mut countries = HashMap::new();
		if let Ok(text) = std::fs::read_to_string(root.join("countryInfo.txt")) {
			for line in text.lines().filter(|l| !l.starts_with('#')) {
				let f: Vec<&str> = line.split('\t').collect();
				if f.len() > 8 {
					countries.insert(f[0].to_owned(), (f[4].to_owned(), continent_of(f[8]).to_owned()));
				}
			}
		}

		let mut regions = HashMap::new();
		if let Ok(text) = std::fs::read_to_string(root.join("admin1CodesASCII.txt")) {
			for line in text.lines() {
				let f: Vec<&str> = line.split('\t').collect();
				if f.len() > 1 {
					regions.insert(f[0].to_owned(), f[1].to_owned());
				}
			}
		}

		let mut subregions = HashMap::new();
		if let Ok(text) = std::fs::read_to_string(root.join("admin2Codes.txt")) {
			for line in text.lines() {
				let f: Vec<&str> = line.split('\t').collect();
				if f.len() > 1 {
					subregions.insert(f[0].to_owned(), f[1].to_owned());
				}
			}
		}

		// Built once. Two hundred thousand points is a second of work and a lookup that costs
		// nothing after, which matters because a library of photographs is imported in batches.
		let places: Vec<Place> = cities
			.lines()
			.filter_map(|line| {
				let f: Vec<&str> = line.split('\t').collect();
				if f.len() < 18 {
					return None;
				}
				Some(Place {
					lat: f[4].parse().ok()?,
					lon: f[5].parse().ok()?,
					name: f[1].to_owned(),
					country: f[8].to_owned(),
					admin1: f[10].to_owned(),
					admin2: f[11].to_owned(),
				})
			})
			.collect();

		Some(Self {
			tree: RTree::bulk_load(places),
			countries,
			regions,
			subregions,
			postal: OnceLock::new(),
			root: root_for_later,
			finder: tzf_rs::DefaultFinder::new(),
		})
	}

	/// Read the postal index now rather than at the first lookup that needs it.
	///
	/// A service answering many lookups pays this before it reports itself healthy, so no request
	/// is the one that waits fourteen seconds.
	pub fn preload(&self) {
		self.postal();
	}

	/// The postal index, read on first use.
	///
	/// By far the largest file here, and only ever needed by an image that recorded where it
	/// was taken. Absent, postal codes are simply missing -- the same state as before it was
	/// fetched.
	fn postal(&self) -> Option<&RTree<Postal>> {
		self
			.postal
			.get_or_init(|| {
				let text = std::fs::read_to_string(self.root.join("postal.txt")).ok()?;
				let points: Vec<Postal> = text
					.lines()
					.filter_map(|line| {
						let f: Vec<&str> = line.split('\t').collect();
						if f.len() < 11 {
							return None;
						}
						Some(Postal {
							lat: f[9].parse().ok()?,
							lon: f[10].parse().ok()?,
							code: f[1].to_owned(),
							country: f[0].to_owned(),
						})
					})
					.collect();
				Some(RTree::bulk_load(points))
			})
			.as_ref()
	}

	/// The address for a position, as far as the data can say.
	///
	/// `district` stays absent. Naming a neighbourhood needs the full GeoNames dump, which is
	/// an order of magnitude larger than everything here put together, and deriving one from
	/// the nearest town would state something no source claimed.
	pub fn lookup(&self, lat: f64, lon: f64) -> Option<Address> {
		let place = self.tree.nearest_neighbor([lon, lat])?;
		let (country, continent) = self.countries.get(&place.country).cloned().unwrap_or_default();
		let region = self.regions.get(&format!("{}.{}", place.country, place.admin1)).cloned();
		let subregion =
			self.subregions.get(&format!("{}.{}.{}", place.country, place.admin1, place.admin2)).cloned();
		// Found by position, then checked against the country the town is in: a code is not
		// unique on its own, and the nearest point to a border could belong to the other side.
		let postal_code = self
			.postal()
			.and_then(|tree| tree.nearest_neighbor([lon, lat]))
			.filter(|found| found.country == place.country)
			.map(|found| found.code.clone());

		Some(Address {
			continent: (!continent.is_empty()).then_some(continent),
			country: (!country.is_empty()).then_some(country),
			country_code: (!place.country.is_empty()).then(|| place.country.clone()),
			region,
			subregion,
			city: (!place.name.is_empty()).then(|| place.name.clone()),
			district: None,
			postal_code,
			// From the polygon the point falls in, not from the nearest town. A settlement a
			// few miles away can sit on the other side of a zone boundary.
			timezone: Some(self.finder.get_tz_name(lon, lat).to_owned()).filter(|t| !t.is_empty()),
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn nearness_is_an_ordering_and_not_a_measurement() {
		// Squared degrees, which is not metres and must never be reported as any. It decides
		// which of two candidates is closer and nothing else.
		let place = Place {
			lat: 0.0,
			lon: 0.0,
			name: "origin".into(),
			country: "XX".into(),
			admin1: "01".into(),
			admin2: "001".into(),
		};
		assert!(place.distance_2(&[1.0, 0.0]) < place.distance_2(&[2.0, 0.0]));
		assert_eq!(place.distance_2(&[0.0, 0.0]), 0.0);
	}

	#[test]
	fn a_missing_gazetteer_is_absence_rather_than_failure() {
		// The data lives outside git. Not having fetched it should read the same as a photograph
		// that carried no position: no address, no error.
		assert!(Gazetteer::open(Path::new("/nowhere-at-all")).is_none());
	}

	#[test]
	fn continents_come_from_the_code_the_data_uses() {
		assert_eq!(continent_of("NA"), "North America");
		assert_eq!(continent_of("EU"), "Europe");
		assert_eq!(continent_of("ZZ"), "");
	}
}
