//! GeoNames' text files, read into the place index.
//!
//! Each line is read as the R-tree before the index read it: the same columns, the same lines
//! skipped, and a file that is not UTF-8 read as absent. A settlement's region, subregion,
//! country and continent are looked up here, once, rather than at every lookup.

use super::index::{ABSENT, Entry, Kind, Strings, encode, level_for};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

pub(super) const CITIES: &str = "cities500.txt";
pub(super) const POSTAL: &str = "postal.txt";

/// Every line of a file, failing on the first that is not UTF-8.
fn lines(path: &Path) -> io::Result<impl Iterator<Item = io::Result<String>>> {
	Ok(BufReader::new(File::open(path)?).lines())
}

pub(super) fn continent_of(code: &str) -> &'static str {
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

/// A small file read whole, or nothing when it is missing or unreadable.
fn small(path: &Path) -> String {
	std::fs::read_to_string(path).unwrap_or_default()
}

/// ISO country code to (name, continent).
fn countries(root: &Path) -> HashMap<String, (String, String)> {
	let mut countries = HashMap::new();
	for line in small(&root.join("countryInfo.txt")).lines().filter(|l| !l.starts_with('#')) {
		let f: Vec<&str> = line.split('\t').collect();
		if f.len() > 8 {
			countries.insert(f[0].to_owned(), (f[4].to_owned(), continent_of(f[8]).to_owned()));
		}
	}
	countries
}

/// A code to a name, from `admin1CodesASCII.txt` or `admin2Codes.txt`.
fn names(path: &Path) -> HashMap<String, String> {
	let mut names = HashMap::new();
	for line in small(path).lines() {
		let f: Vec<&str> = line.split('\t').collect();
		if f.len() > 1 {
			names.insert(f[0].to_owned(), f[1].to_owned());
		}
	}
	names
}

/// The settlements' index, from `cities500.txt` and the files that name its codes.
pub(super) fn places(root: &Path) -> io::Result<Vec<u8>> {
	let countries = countries(root);
	let regions = names(&root.join("admin1CodesASCII.txt"));
	let subregions = names(&root.join("admin2Codes.txt"));
	let mut strings = Strings::default();
	let mut entries = Vec::new();
	for line in lines(&root.join(CITIES))? {
		let line = line?;
		let f: Vec<&str> = line.split('\t').collect();
		if f.len() < 18 {
			continue;
		}
		let (Ok(lat), Ok(lon)) = (f[4].parse(), f[5].parse()) else {
			continue;
		};
		let (country, admin1, admin2) = (f[8], f[10], f[11]);
		let (name, continent) = countries.get(country).cloned().unwrap_or_default();
		let present = |text: &str, strings: &mut Strings| {
			if text.is_empty() { ABSENT } else { strings.shared(text) }
		};
		let region = regions.get(&format!("{country}.{admin1}"));
		let subregion = subregions.get(&format!("{country}.{admin1}.{admin2}"));
		let fields = [
			strings.add(f[1]),
			strings.shared(country),
			present(&name, &mut strings),
			present(&continent, &mut strings),
			region.map_or(ABSENT, |r| strings.shared(r)),
			subregion.map_or(ABSENT, |s| strings.shared(s)),
		];
		entries.push(Entry { lat, lon, fields });
	}
	Ok(encode(Kind::Places, level_for(entries.len()), entries, strings))
}

/// The postal areas' index, from `postal.txt`.
pub(super) fn postal(root: &Path) -> io::Result<Vec<u8>> {
	let mut strings = Strings::default();
	let mut entries = Vec::new();
	for line in lines(&root.join(POSTAL))? {
		let line = line?;
		let f: Vec<&str> = line.split('\t').collect();
		if f.len() < 11 {
			continue;
		}
		let (Ok(lat), Ok(lon)) = (f[9].parse(), f[10].parse()) else {
			continue;
		};
		let fields = [strings.add(f[1]), strings.shared(f[0])];
		entries.push(Entry { lat, lon, fields });
	}
	Ok(encode(Kind::Postal, level_for(entries.len()), entries, strings))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn continents_come_from_the_code_the_data_uses() {
		assert_eq!(continent_of("NA"), "North America");
		assert_eq!(continent_of("EU"), "Europe");
		assert_eq!(continent_of("ZZ"), "");
	}
}
