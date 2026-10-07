//! What the index replaced, kept to test it against: GeoNames' text parsed into R-trees, as
//! version 1.0 read it, and the fixture files both are read from.

use super::Address;
use rstar::{AABB, PointDistance, RTree, RTreeObject};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
struct Place {
	lat: f64,
	lon: f64,
	name: String,
	country: String,
	admin1: String,
	admin2: String,
}

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

impl PointDistance for Place {
	fn distance_2(&self, point: &[f64; 2]) -> f64 {
		let dx = self.lon - point[0];
		let dy = self.lat - point[1];
		dx * dx + dy * dy
	}
}

impl PointDistance for Postal {
	fn distance_2(&self, point: &[f64; 2]) -> f64 {
		let dx = self.lon - point[0];
		let dy = self.lat - point[1];
		dx * dx + dy * dy
	}
}

/// Every item as near as the nearest, in the tree's order.
fn tied<T: PointDistance<Envelope = AABB<[f64; 2]>>>(
	tree: &RTree<T>,
	lat: f64,
	lon: f64,
) -> Vec<&T> {
	let mut found = tree.nearest_neighbor_iter_with_distance_2([lon, lat]);
	let Some((first, nearest)) = found.next() else {
		return Vec::new();
	};
	let mut all = vec![first];
	all.extend(found.take_while(|(_, d)| *d == nearest).map(|(item, _)| item));
	all
}

/// How a run of comparisons went: answers equal to the R-tree's, and answers the R-tree would
/// have given had it broken a tie the other way.
#[derive(Debug, Default)]
pub struct Tally {
	pub same: usize,
	pub tie: usize,
}

pub struct Reference {
	tree: RTree<Place>,
	countries: HashMap<String, (String, String)>,
	regions: HashMap<String, String>,
	subregions: HashMap<String, String>,
	postal: Option<RTree<Postal>>,
	finder: tzf_rs::DefaultFinder,
}

impl Reference {
	/// Version 1.0's `Gazetteer::open`, with the postal tree read at once.
	pub fn open(root: &Path) -> Self {
		let cities = std::fs::read_to_string(root.join("cities500.txt")).unwrap();
		let read = |name: &str| std::fs::read_to_string(root.join(name)).unwrap_or_default();
		let mut countries = HashMap::new();
		for line in read("countryInfo.txt").lines().filter(|l| !l.starts_with('#')) {
			let f: Vec<&str> = line.split('\t').collect();
			if f.len() > 8 {
				let continent = super::source::continent_of(f[8]);
				countries.insert(f[0].to_owned(), (f[4].to_owned(), continent.to_owned()));
			}
		}
		let named = |name: &str| {
			let mut map = HashMap::new();
			for line in read(name).lines() {
				let f: Vec<&str> = line.split('\t').collect();
				if f.len() > 1 {
					map.insert(f[0].to_owned(), f[1].to_owned());
				}
			}
			map
		};
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
		let postal = std::fs::read_to_string(root.join("postal.txt")).ok().map(|text| {
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
			RTree::bulk_load(points)
		});
		Self {
			tree: RTree::bulk_load(places),
			countries,
			regions: named("admin1CodesASCII.txt"),
			subregions: named("admin2Codes.txt"),
			postal,
			finder: tzf_rs::DefaultFinder::new(),
		}
	}

	fn address(&self, place: &Place, postal: Option<&Postal>, lat: f64, lon: f64) -> Address {
		let (country, continent) = self.countries.get(&place.country).cloned().unwrap_or_default();
		let region = self.regions.get(&format!("{}.{}", place.country, place.admin1)).cloned();
		let subregion =
			self.subregions.get(&format!("{}.{}.{}", place.country, place.admin1, place.admin2)).cloned();
		Address {
			continent: (!continent.is_empty()).then_some(continent),
			country: (!country.is_empty()).then_some(country),
			country_code: (!place.country.is_empty()).then(|| place.country.clone()),
			region,
			subregion,
			city: (!place.name.is_empty()).then(|| place.name.clone()),
			district: None,
			postal_code: postal.filter(|p| p.country == place.country).map(|p| p.code.clone()),
			timezone: Some(self.finder.get_tz_name(lon, lat).to_owned()).filter(|t| !t.is_empty()),
		}
	}

	/// Version 1.0's `Gazetteer::lookup`.
	pub fn lookup(&self, lat: f64, lon: f64) -> Option<Address> {
		let place = self.tree.nearest_neighbor([lon, lat])?;
		let postal = self.postal.as_ref().and_then(|tree| tree.nearest_neighbor([lon, lat]));
		Some(self.address(place, postal, lat, lon))
	}

	/// Every answer version 1.0 could have given, had its trees broken ties another way.
	fn possible(&self, lat: f64, lon: f64) -> Vec<Address> {
		let postal = self.postal.as_ref().map(|tree| tied(tree, lat, lon)).unwrap_or_default();
		let mut all = Vec::new();
		for place in tied(&self.tree, lat, lon) {
			if postal.is_empty() {
				all.push(self.address(place, None, lat, lon));
			}
			for &code in &postal {
				all.push(self.address(place, Some(code), lat, lon));
			}
		}
		all
	}

	/// Whether `found` is the answer, or one of the tied answers, version 1.0 gave.
	pub fn judge(
		&self,
		lat: f64,
		lon: f64,
		expected: Option<&Address>,
		found: Option<&Address>,
	) -> Option<bool> {
		if expected == found {
			return Some(true);
		}
		let found = found?;
		self.possible(lat, lon).contains(found).then_some(false)
	}

	pub fn assert_same(
		&self,
		lat: f64,
		lon: f64,
		expected: Option<&Address>,
		found: Option<&Address>,
		how: &str,
	) {
		assert!(
			self.judge(lat, lon, expected, found).is_some(),
			"{how} at {lat},{lon}: expected {expected:?}, found {found:?}"
		);
	}

	/// Compares every query, panicking on the first answer that is not a possible one.
	pub fn compare_all(
		&self,
		gazetteer: &super::Gazetteer,
		queries: impl Iterator<Item = (f64, f64)>,
	) -> Tally {
		let mut tally = Tally::default();
		let mut wrong = String::new();
		for (lat, lon) in queries {
			let expected = self.lookup(lat, lon);
			let found = gazetteer.lookup(lat, lon);
			match self.judge(lat, lon, expected.as_ref(), found.as_ref()) {
				Some(true) => tally.same += 1,
				Some(false) => tally.tie += 1,
				None => {
					let _ = writeln!(wrong, "{lat},{lon}: expected {expected:?}, found {found:?}");
				}
			}
		}
		assert!(wrong.is_empty(), "{wrong}");
		tally
	}

	/// Positions a hair from settlements, where a nearest-neighbor answer is hardest.
	pub fn near_places(&self, count: usize) -> impl Iterator<Item = (f64, f64)> + '_ {
		let step = (self.tree.size() / count.max(1)).max(1);
		let mut random = Random(11);
		self.tree.iter().step_by(step).map(move |place| {
			let jitter = |r: &mut Random| (r.unit() - 0.5) * 0.02;
			(place.lat + jitter(&mut random), place.lon + jitter(&mut random))
		})
	}
}

/// splitmix64, so a fixture is the same on every run.
pub struct Random(pub u64);

impl Random {
	pub fn next(&mut self) -> u64 {
		self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
		let mut z = self.0;
		z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
		z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
		z ^ (z >> 31)
	}

	pub fn unit(&mut self) -> f64 {
		(self.next() >> 11) as f64 / (1u64 << 53) as f64
	}

	fn pick<'a>(&mut self, from: &[&'a str]) -> &'a str {
		from[(self.next() % from.len() as u64) as usize]
	}
}

/// Uniform positions over the whole globe.
pub fn random_queries(count: usize, seed: u64) -> impl Iterator<Item = (f64, f64)> {
	let mut random = Random(seed);
	(0..count).map(move |_| (random.unit() * 180.0 - 90.0, random.unit() * 360.0 - 180.0))
}

/// Rounded as GeoNames writes positions.
fn five(value: f64) -> f64 {
	(value * 1e5).round() / 1e5
}

/// Positions the fixture keeps clear of random points, so the tests that ask near them know
/// what is nearest.
const RESERVED: [(f64, f64); 3] = [(40.0, 40.0), (-20.0, -20.0), (10.0, 10.0)];

fn reserved(lat: f64, lon: f64) -> bool {
	RESERVED.iter().any(|&(a, b)| (lat - a).abs() < 1.5 && (lon - b).abs() < 1.5)
}

/// GeoNames-shaped text files in a directory of their own, removed when dropped.
pub struct Fixture {
	dir: PathBuf,
	source: PathBuf,
	places: Vec<(f64, f64)>,
}

impl Fixture {
	pub fn new(name: &str) -> Self {
		let dir = std::env::temp_dir().join(format!("whereabouts-{}-{name}", std::process::id()));
		let source = dir.join("source");
		let _ = std::fs::remove_dir_all(&dir);
		std::fs::create_dir_all(&source).unwrap();
		let mut random = Random(42);

		let countries = "# ISO\tISO3\tnumeric\tfips\tCountry\tCapital\tArea\tPopulation\tContinent\n\
			AA\tAAA\t001\tAA\tAland\tA\t1\t1\tEU\n\
			BB\tBBB\t002\tBB\tBeeland\tB\t1\t1\tAS\n\
			CC\tCCC\t003\tCC\tCeeland\tC\t1\t1\tXX\n";
		std::fs::write(source.join("countryInfo.txt"), countries).unwrap();
		let admin1 = "AA.01\tAa One\tAa One\t1\nAA.02\t\nBB.01\tBb One\tBb One\t2\nshort\n";
		std::fs::write(source.join("admin1CodesASCII.txt"), admin1).unwrap();
		std::fs::write(source.join("admin2Codes.txt"), "AA.01.001\tAa One One\tx\t1\n").unwrap();

		let mut places = Vec::new();
		let mut cities = String::new();
		let mut city = |lat: f64, lon: f64, name: &str, country: &str, admin1: &str, admin2: &str| {
			let _ = writeln!(
				cities,
				"1\t{name}\t{name}\t\t{lat}\t{lon}\tP\tPPL\t{country}\t\t{admin1}\t{admin2}\t\t\t500\t\t1\tTZ\t2026-01-01"
			);
			places.push((lat, lon));
		};
		let countries = ["AA", "BB", "CC", "ZZ", ""];
		for i in 0..3000 {
			let (lat, lon) = (five(random.unit() * 180.0 - 90.0), five(random.unit() * 360.0 - 180.0));
			if !reserved(lat, lon) {
				let (country, admin1) = (random.pick(&countries), random.pick(&["01", "02", "03"]));
				city(lat, lon, &format!("Town {i}"), country, admin1, random.pick(&["001", ""]));
			}
		}
		// Dense clusters, where cells hold more than a leaf's worth.
		for (i, (clat, clon)) in
			[(48.85, 2.35), (35.68, 139.69), (-33.87, 151.2)].into_iter().enumerate()
		{
			for j in 0..700 {
				let lat = five(clat + (random.unit() - 0.5) * 0.4);
				let lon = five(clon + (random.unit() - 0.5) * 0.4);
				city(lat, lon, &format!("Cluster {i}.{j}"), "BB", "01", "");
			}
		}
		// The poles, the antimeridian and the lines cells are cut along, at every level.
		for (lat, lon) in
			[(90.0, 0.0), (-90.0, 0.0), (0.0, 180.0), (0.0, -180.0), (89.99999, 179.99999)]
				.into_iter()
				.chain([(-89.99999, -179.99999), (0.5, 179.99999), (-0.5, -179.99999), (90.0, 180.0)])
		{
			city(lat, lon, &format!("Edge {lat} {lon}"), "CC", "01", "");
		}
		for level in 1..=10 {
			let n = f64::from(1u32 << level);
			for k in [1.0, n / 2.0 - 1.0, n / 2.0 + 1.0, n - 1.0] {
				let (lat, lon) = (five(-90.0 + k * 180.0 / n), five(-180.0 + k * 360.0 / n));
				if !reserved(lat, lon) {
					city(lat, lon, &format!("Line {level} {k}"), "AA", "01", "001");
				}
			}
		}
		city(40.0, 40.0, "Borderton", "AA", "01", "001");
		city(-20.0, -20.0, "First", "AA", "02", "");
		city(-20.0, -20.0, "Second", "AA", "02", "");
		city(10.0, 10.0, "Tenton", "AA", "01", "001");
		cities.push_str(
			"short\tline\n1\tBad\tBad\t\tnorth\t1\tP\tPPL\tAA\t\t01\t\t\t\t500\t\t1\tTZ\t2026\n",
		);
		std::fs::write(source.join("cities500.txt"), cities).unwrap();

		let mut postal = String::new();
		let mut code = |lat: f64, lon: f64, code: &str, country: &str| {
			let _ = writeln!(postal, "{country}\t{code}\tPlace\t\t\t\t\t\t\t{lat}\t{lon}\t4");
		};
		for i in 0..6000 {
			let (lat, lon) = (five(random.unit() * 180.0 - 90.0), five(random.unit() * 360.0 - 180.0));
			if !reserved(lat, lon) {
				code(lat, lon, &format!("P{i}"), random.pick(&["AA", "BB", "ZZ", ""]));
				// Codes sharing one position, as GeoNames gives many.
				if i % 50 == 0 {
					code(lat, lon, &format!("P{i}b"), "AA");
					code(lat, lon, "", "BB");
				}
			}
		}
		code(40.02, 40.02, "B-4040", "BB");
		code(39.98, 39.98, "A-4040", "AA");
		code(10.001, 10.001, "A-1010", "AA");
		postal.push_str("AA\tnowhere\tPlace\t\t\t\t\t\t\t\t\t\nshort\n");
		std::fs::write(source.join("postal.txt"), postal).unwrap();

		Self { dir, source, places }
	}

	pub fn source(&self) -> &Path {
		&self.source
	}

	/// The index for the source, built into a directory holding nothing else.
	pub fn built(&self) -> PathBuf {
		let built = self.dir.join("built");
		super::Gazetteer::build(&self.source, &built).unwrap();
		built
	}

	/// Uniform positions, positions a hair from every settlement and exactly on some, and the
	/// edges of the globe.
	pub fn queries(&self) -> Vec<(f64, f64)> {
		let mut random = Random(7);
		let mut queries: Vec<(f64, f64)> = random_queries(4000, 3).collect();
		for &(lat, lon) in &self.places {
			queries.push((lat + (random.unit() - 0.5) * 0.01, lon + (random.unit() - 0.5) * 0.01));
		}
		queries.extend(self.places.iter().step_by(7).copied());
		for lat in [-90.0, -89.99999, -45.0, 0.0, 45.0, 89.99999, 90.0] {
			for lon in [-180.0, -179.99999, -179.9, 0.0, 179.9, 179.99999, 180.0] {
				queries.push((lat, lon));
			}
		}
		queries
	}
}

impl Drop for Fixture {
	fn drop(&mut self) {
		let _ = std::fs::remove_dir_all(&self.dir);
	}
}
