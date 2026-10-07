//! A latitude and longitude to the place it is in.
//!
//! GeoNames' `cities500`, searched for the nearest settlement; `countryInfo.txt`,
//! `admin1CodesASCII.txt` and `admin2Codes.txt` name its country and regions, and `postal.txt`,
//! read on first need, its postal code. All of them sit in one directory, the caller's, and are
//! read from the place index [`Gazetteer::build`] writes there, or turned into one in memory when
//! it has not been written. The time zone comes from the polygon the point falls in, not from the
//! settlement.

mod index;
mod source;

use index::{Index, Kind};
use serde::{Deserialize, Serialize};
use std::io;
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

/// The settlements' index file in a data directory, written by [`Gazetteer::build`].
pub const PLACES_FILE: &str = "places.index";
/// The postal areas' index file in a data directory, written by [`Gazetteer::build`].
pub const POSTAL_FILE: &str = "postal.index";

pub struct Gazetteer {
	places: Index,
	/// Postal areas, opened the first time a lookup needs them.
	///
	/// Mapping the index costs nothing; building it from `postal.txt`, when only the text is
	/// there, costs seconds for 1.8 million points, and most imports are screenshots with no GPS
	/// at all. Paying that before knowing whether anything will ask is the cost of a guess.
	postal: OnceLock<Option<Index>>,
	root: PathBuf,
	/// Queries tzf's own file where it lies, in the binary, rather than expanding its polygons
	/// onto the heap; the answers are `DefaultFinder`'s.
	finder: tzf_rs::EmbeddedFinder,
}

/// The index of `kind` in `root`: the file `name` mapped when it is there and readable, or else
/// one built in memory by `build` from the text beside it.
fn open_index(
	root: &Path,
	name: &str,
	kind: Kind,
	build: fn(&Path) -> io::Result<Vec<u8>>,
) -> Option<Index> {
	Index::map(&root.join(name), kind).ok().or_else(|| Index::from_vec(build(root).ok()?, kind).ok())
}

/// Writes `bytes` to `name` in `target` by way of a file beside it, so a reader with the old one
/// mapped keeps reading a whole file.
fn replace(target: &Path, name: &str, bytes: &[u8]) -> io::Result<()> {
	let staged = target.join(format!(".{name}.new"));
	std::fs::write(&staged, bytes)?;
	std::fs::rename(&staged, target.join(name))
}

impl Gazetteer {
	/// Read the gazetteer from its data directory, or `None` when it has not been fetched.
	///
	/// [`PLACES_FILE`] is mapped when it is there, and the heap holds next to nothing; without
	/// it, the same index is built in memory from `cities500.txt` and the files beside it. A
	/// mapped index must be replaced by renaming a new file over it, never written in place --
	/// [`Gazetteer::build`] does so.
	pub fn open(root: &Path) -> Option<Self> {
		let places = open_index(root, PLACES_FILE, Kind::Places, source::places)?;
		Some(Self {
			places,
			postal: OnceLock::new(),
			root: root.to_path_buf(),
			finder: tzf_rs::EmbeddedFinder::new(),
		})
	}

	/// Map the index [`Gazetteer::build`] wrote into `root`, and never read the text.
	///
	/// Fails when [`PLACES_FILE`] is missing or unreadable, or when [`POSTAL_FILE`] is there and
	/// unreadable; without it, postal codes are missing. Everything is mapped before this returns,
	/// so the heap holds next to nothing and no lookup waits. The same rule as [`Gazetteer::open`]
	/// holds: a mapped file is replaced by renaming over it, never written in place.
	pub fn map(root: &Path) -> io::Result<Self> {
		let places = Index::map(&root.join(PLACES_FILE), Kind::Places)?;
		let postal = match Index::map(&root.join(POSTAL_FILE), Kind::Postal) {
			Ok(index) => Some(index),
			Err(error) if error.kind() == io::ErrorKind::NotFound => None,
			Err(error) => return Err(error),
		};
		Ok(Self {
			places,
			postal: OnceLock::from(postal),
			root: root.to_path_buf(),
			finder: tzf_rs::EmbeddedFinder::new(),
		})
	}

	/// Write the place index for the GeoNames text in `source` into `target`, creating it.
	///
	/// Reads `cities500.txt`, which must be there, and `countryInfo.txt`, `admin1CodesASCII.txt`,
	/// `admin2Codes.txt` and `postal.txt`, each of which may be missing; writes [`PLACES_FILE`],
	/// and [`POSTAL_FILE`] when there was a `postal.txt`. Each is renamed into place, so a
	/// gazetteer with the old files open keeps answering from them. `target` then answers
	/// [`Gazetteer::open`] alone, without the text.
	pub fn build(source: &Path, target: &Path) -> io::Result<()> {
		std::fs::create_dir_all(target)?;
		replace(target, PLACES_FILE, &source::places(source)?)?;
		match source::postal(source) {
			Ok(bytes) => replace(target, POSTAL_FILE, &bytes),
			Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
			Err(error) => Err(error),
		}
	}

	/// Open the postal index now rather than at the first lookup that needs it.
	///
	/// With [`POSTAL_FILE`] present this maps it and costs nothing to speak of. With only
	/// `postal.txt`, it builds the index in memory, so a service paying it before it reports
	/// itself healthy makes no request the one that waits.
	pub fn preload(&self) {
		self.postal();
	}

	/// The postal index, opened on first use.
	///
	/// By far the largest file here, and only ever needed by an image that recorded where it
	/// was taken. Absent, postal codes are simply missing -- the same state as before it was
	/// fetched.
	fn postal(&self) -> Option<&Index> {
		self
			.postal
			.get_or_init(|| open_index(&self.root, POSTAL_FILE, Kind::Postal, source::postal))
			.as_ref()
	}

	/// The address for a position, as far as the data can say; none for one that is not finite.
	///
	/// The nearest settlement is nearest in squared degrees: an ordering, never reported as a
	/// distance. `district` stays absent. Naming a neighborhood needs the full GeoNames dump, an
	/// order of magnitude larger than everything here, and deriving one from the nearest town
	/// would state something no source claimed.
	pub fn lookup(&self, lat: f64, lon: f64) -> Option<Address> {
		let place = self.places.nearest(lat, lon)?;
		let text = |field| self.places.text(place, field);
		let country_code = text(1).unwrap_or_default();
		// Found by position, then checked against the country the town is in: a code is not
		// unique on its own, and the nearest point to a border could belong to the other side.
		let postal_code = self.postal().and_then(|postal| {
			let found = postal.nearest(lat, lon)?;
			(postal.text(found, 1).unwrap_or_default() == country_code)
				.then(|| postal.text(found, 0).unwrap_or_default().to_owned())
		});
		let present = |field| text(field).filter(|t| !t.is_empty()).map(str::to_owned);

		Some(Address {
			continent: present(3),
			country: present(2),
			country_code: present(1),
			region: text(4).map(str::to_owned),
			subregion: text(5).map(str::to_owned),
			city: present(0),
			district: None,
			postal_code,
			// From the polygon the point falls in, not from the nearest town. A settlement a
			// few miles away can sit on the other side of a zone boundary.
			timezone: Some(self.finder.get_tz_name(lon, lat).to_owned()).filter(|t| !t.is_empty()),
		})
	}
}

#[cfg(test)]
mod reference;

#[cfg(test)]
mod tests {
	use super::*;
	use reference::{Fixture, Reference};

	#[test]
	fn a_missing_gazetteer_is_absence_rather_than_failure() {
		// The data lives outside git. Not having fetched it should read the same as a photograph
		// that carried no position: no address, no error.
		assert!(Gazetteer::open(Path::new("/nowhere-at-all")).is_none());
	}

	#[test]
	fn is_shared_across_threads() {
		fn shared<T: Send + Sync>() {}
		shared::<Gazetteer>();
	}

	#[test]
	fn the_index_answers_what_the_r_tree_answered() {
		let fixture = Fixture::new("answers");
		let reference = Reference::open(fixture.source());
		let built = fixture.built();
		let mapped = Gazetteer::map(&built).unwrap();
		let in_memory = Gazetteer::open(fixture.source()).unwrap();
		for (lat, lon) in fixture.queries() {
			let expected = reference.lookup(lat, lon);
			for (how, gazetteer) in [("mapped", &mapped), ("in memory", &in_memory)] {
				let found = gazetteer.lookup(lat, lon);
				reference.assert_same(lat, lon, expected.as_ref(), found.as_ref(), how);
			}
		}
	}

	#[test]
	fn the_index_alone_is_enough() {
		let fixture = Fixture::new("alone");
		let built = fixture.built();
		let names: Vec<_> =
			std::fs::read_dir(&built).unwrap().map(|e| e.unwrap().file_name()).collect();
		assert_eq!(names.len(), 2, "{names:?}");
		let gazetteer = Gazetteer::map(&built).unwrap();
		let address = gazetteer.lookup(10.0, 10.0).unwrap();
		assert_eq!(address.country_code.as_deref(), Some("AA"));
		assert!(address.postal_code.is_some());
	}

	#[test]
	fn a_postal_code_across_the_border_is_left_out() {
		let fixture = Fixture::new("border");
		let gazetteer = Gazetteer::open(&fixture.built()).unwrap();
		// The town at 40,40 is AA's; the postal point nearest 40.01,40.01 is BB's.
		let address = gazetteer.lookup(40.01, 40.01).unwrap();
		assert_eq!((address.city.as_deref(), address.postal_code), (Some("Borderton"), None));
		let address = gazetteer.lookup(39.99, 39.99).unwrap();
		assert_eq!(address.postal_code.as_deref(), Some("A-4040"));
	}

	#[test]
	fn the_same_position_twice_answers_with_the_earlier_line() {
		let fixture = Fixture::new("twice");
		let gazetteer = Gazetteer::open(&fixture.built()).unwrap();
		assert_eq!(gazetteer.lookup(-20.0, -20.0).unwrap().city.as_deref(), Some("First"));
	}

	#[test]
	fn a_build_without_postal_text_answers_without_postal_codes() {
		let fixture = Fixture::new("nopostal");
		std::fs::remove_file(fixture.source().join("postal.txt")).unwrap();
		let built = fixture.built();
		assert!(!built.join(POSTAL_FILE).exists());
		let address = Gazetteer::open(&built).unwrap().lookup(10.0, 10.0).unwrap();
		assert_eq!((address.country_code.as_deref(), address.postal_code), (Some("AA"), None));
	}

	#[test]
	fn mapping_never_reads_the_text() {
		let fixture = Fixture::new("maponly");
		let refused = Gazetteer::map(fixture.source()).err().map(|error| error.kind());
		assert_eq!(refused, Some(std::io::ErrorKind::NotFound));
		let built = fixture.built();
		std::fs::write(built.join(POSTAL_FILE), b"not an index").unwrap();
		assert!(Gazetteer::map(&built).is_err());
		std::fs::remove_file(built.join(POSTAL_FILE)).unwrap();
		let address = Gazetteer::map(&built).unwrap().lookup(10.0, 10.0).unwrap();
		assert_eq!((address.city.as_deref(), address.postal_code), (Some("Tenton"), None));
	}

	#[test]
	fn the_time_zone_is_the_one_the_expanded_finder_gives() {
		let embedded = tzf_rs::EmbeddedFinder::new();
		let expanded = tzf_rs::DefaultFinder::new();
		for lat in (-900..=900).step_by(7) {
			for lon in (-1800..=1800).step_by(11) {
				let (lat, lon) = (f64::from(lat) / 10.0, f64::from(lon) / 10.0);
				assert_eq!(embedded.get_tz_name(lon, lat), expanded.get_tz_name(lon, lat), "{lat},{lon}");
			}
		}
	}

	/// Against the real GeoNames files in `WHEREABOUTS_GEONAMES`, when it is set:
	/// `cargo test -p whereabouts --all-features -- --ignored real`.
	#[test]
	#[ignore = "needs GeoNames' files on disk"]
	fn the_index_answers_what_the_r_tree_answered_over_real_data() {
		let Some(root) = std::env::var_os("WHEREABOUTS_GEONAMES") else {
			return;
		};
		let root = PathBuf::from(root);
		let reference = Reference::open(&root);
		let gazetteer = Gazetteer::open(&root).unwrap();
		gazetteer.preload();
		let queries = reference::random_queries(20_000, 7).chain(reference.near_places(20_000));
		let ties = reference.compare_all(&gazetteer, queries);
		eprintln!("real data: {ties:?}");
	}
}
