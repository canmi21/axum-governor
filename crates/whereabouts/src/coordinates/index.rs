//! The place index: points laid out to be read where they lie, from a mapped file or a buffer.
//!
//! One file per kind of point. A header, a table of offsets, records of one width and the strings
//! they point into. Records are sorted along a Z-order curve over a grid of the globe, so any cell
//! at any coarser level is one contiguous run of records and the table finds it in two reads; a
//! nearest lookup walks those cells best first and touches the few pages around the point. See
//! platform's spec/architecture/geo.md, "Both lookups are files laid out for asking".

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::fs::File;
use std::io;
use std::path::Path;

const MAGIC: [u8; 4] = *b"WHRB";
/// Raised on any change a reader of the previous version would misread.
const VERSION: u32 = 1;
const HEADER: usize = 32;

/// Degrees to fixed point. GeoNames writes at most five decimals, so a position read back is the
/// same `f64` the text parses to, and a distance computed from it is the same number.
const SCALE: f64 = 1e7;
const LON_HALF: i64 = 1_800_000_000;
const LAT_HALF: i64 = 900_000_000;
/// One more than the range, so 180 and 90 fall inside the last cell rather than past it.
const LON_SPAN: u64 = 3_600_000_001;
const LAT_SPAN: u64 = 1_800_000_001;

/// The deepest level a file is cut to: a million cells, cells a third of a degree wide, and a
/// 4 MB offset table of which a lookup reads a few pages.
const MAX_LEVEL: u32 = 10;
/// A run this short is scanned rather than split further.
const LEAF: usize = 16;

/// A string field that holds nothing, which is not the same as one holding `""`.
pub(super) const ABSENT: u32 = u32::MAX;

/// What a file holds, so one kind is never read as the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
	Places = 1,
	Postal = 2,
}

impl Kind {
	/// String fields per record: a settlement's name, country code, country, continent, region
	/// and subregion; a postal area's code and country code.
	pub const fn fields(self) -> usize {
		match self {
			Kind::Places => 6,
			Kind::Postal => 2,
		}
	}
}

/// One point to be written: a position and `N` string fields, each a handle from [`Strings`].
pub(super) struct Entry<const N: usize> {
	pub lat: f64,
	pub lon: f64,
	pub fields: [u32; N],
}

/// The strings a file's records point into. A handle is a byte offset to a little-endian `u16`
/// length and the bytes after it.
#[derive(Default)]
pub(super) struct Strings {
	bytes: Vec<u8>,
	seen: HashMap<String, u32>,
}

impl Strings {
	/// A string few records share, such as a name: written as it comes.
	pub fn add(&mut self, text: &str) -> u32 {
		let handle = u32::try_from(self.bytes.len()).unwrap_or(ABSENT);
		let mut end = text.len().min(usize::from(u16::MAX));
		while !text.is_char_boundary(end) {
			end -= 1;
		}
		self.bytes.extend_from_slice(&(end as u16).to_le_bytes());
		self.bytes.extend_from_slice(&text.as_bytes()[..end]);
		handle
	}

	/// A string many records share, such as a country: written once.
	pub fn shared(&mut self, text: &str) -> u32 {
		if let Some(&handle) = self.seen.get(text) {
			return handle;
		}
		let handle = self.add(text);
		self.seen.insert(text.to_owned(), handle);
		handle
	}
}

/// The level whose cell count first reaches the number of points, within `1..=MAX_LEVEL`.
pub(super) fn level_for(count: usize) -> u32 {
	let mut level = 1;
	while level < MAX_LEVEL && (1usize << (2 * level)) < count {
		level += 1;
	}
	level
}

/// Degrees to fixed point; `as` saturates, and nothing GeoNames writes is out of range.
fn fixed(degrees: f64) -> i32 {
	(degrees * SCALE).round() as i32
}

fn degrees(fixed: i32) -> f64 {
	f64::from(fixed) / SCALE
}

/// The column or row a fixed-point value falls in at `level`.
fn slot(value: i32, half: i64, span: u64, level: u32) -> u32 {
	let shifted = (i64::from(value) + half).clamp(0, span as i64 - 1) as u64;
	((shifted << level) / span) as u32
}

/// The bounds of a column or row, in degrees, never narrower than the values assigned to it; the
/// edge slots are open, since `slot` clamps whatever lies past them into them.
fn bounds(index: u32, half: i64, span: u64, level: u32) -> (f64, f64) {
	let last = (1u32 << level) - 1;
	let low = if index == 0 {
		f64::NEG_INFINITY
	} else {
		degrees((((u64::from(index) * span) >> level) as i64 - half) as i32)
	};
	let high = if index == last {
		f64::INFINITY
	} else {
		degrees(((u64::from(index + 1) * span).div_ceil(1 << level) as i64 - half) as i32)
	};
	(low, high)
}

/// Interleaves `x` and `y` into a Z-order key, `x` on the even bits.
fn morton(x: u32, y: u32) -> u64 {
	fn spread(v: u32) -> u64 {
		let mut v = u64::from(v);
		v = (v | (v << 16)) & 0x0000_ffff_0000_ffff;
		v = (v | (v << 8)) & 0x00ff_00ff_00ff_00ff;
		v = (v | (v << 4)) & 0x0f0f_0f0f_0f0f_0f0f;
		v = (v | (v << 2)) & 0x3333_3333_3333_3333;
		(v | (v << 1)) & 0x5555_5555_5555_5555
	}
	spread(x) | (spread(y) << 1)
}

/// Writes `entries` as a file of `kind` cut to `level`, records sorted by cell and, within a cell,
/// kept in the order given.
pub(super) fn encode<const N: usize>(
	kind: Kind,
	level: u32,
	entries: Vec<Entry<N>>,
	strings: Strings,
) -> Vec<u8> {
	debug_assert_eq!(N, kind.fields());
	let level = level.clamp(1, MAX_LEVEL);
	let width = 8 + 4 * N;
	let mut keyed: Vec<(u64, i32, i32, [u32; N])> = entries
		.into_iter()
		.map(|entry| {
			let (lat, lon) = (fixed(entry.lat), fixed(entry.lon));
			let x = slot(lon, LON_HALF, LON_SPAN, level);
			let y = slot(lat, LAT_HALF, LAT_SPAN, level);
			(morton(x, y), lat, lon, entry.fields)
		})
		.collect();
	keyed.sort_by_key(|entry| entry.0);

	let cells = 1usize << (2 * level);
	let mut table = vec![0u32; cells + 1];
	for entry in &keyed {
		table[entry.0 as usize + 1] += 1;
	}
	for i in 1..table.len() {
		table[i] += table[i - 1];
	}

	let count = keyed.len();
	let mut out = Vec::with_capacity(HEADER + 4 * table.len() + width * count + strings.bytes.len());
	out.extend_from_slice(&MAGIC);
	for word in
		[VERSION, kind as u32, level, count as u32, width as u32, strings.bytes.len() as u32, 0]
	{
		out.extend_from_slice(&word.to_le_bytes());
	}
	for offset in table {
		out.extend_from_slice(&offset.to_le_bytes());
	}
	for (_, lat, lon, fields) in keyed {
		out.extend_from_slice(&lat.to_le_bytes());
		out.extend_from_slice(&lon.to_le_bytes());
		for field in fields {
			out.extend_from_slice(&field.to_le_bytes());
		}
	}
	out.extend_from_slice(&strings.bytes);
	out
}

enum Bytes {
	Mapped(memmap2::Mmap),
	Owned(Vec<u8>),
}

impl Bytes {
	fn as_slice(&self) -> &[u8] {
		match self {
			Bytes::Mapped(map) => map,
			Bytes::Owned(vec) => vec,
		}
	}
}

/// A file of one kind, checked once on opening so every read after is in bounds.
pub(super) struct Index {
	bytes: Bytes,
	level: u32,
	count: usize,
	width: usize,
	records: usize,
	strings: usize,
}

impl Index {
	/// Maps the file at `path`.
	///
	/// The file must not be written over while it is mapped: a new index is renamed into place,
	/// which leaves the mapped one whole until it is dropped.
	pub fn map(path: &Path, kind: Kind) -> io::Result<Self> {
		let file = File::open(path)?;
		// SAFETY: see above; the files are replaced by rename and never truncated or rewritten.
		let map = unsafe { memmap2::Mmap::map(&file)? };
		Self::new(Bytes::Mapped(map), kind)
	}

	pub fn from_vec(bytes: Vec<u8>, kind: Kind) -> io::Result<Self> {
		Self::new(Bytes::Owned(bytes), kind)
	}

	fn new(bytes: Bytes, kind: Kind) -> io::Result<Self> {
		let invalid =
			|what: &str| io::Error::new(io::ErrorKind::InvalidData, format!("place index: {what}"));
		let raw = bytes.as_slice();
		if raw.len() < HEADER || raw[..4] != MAGIC {
			return Err(invalid("not an index"));
		}
		let word =
			|i: usize| u32::from_le_bytes(raw[4 + 4 * i..8 + 4 * i].try_into().unwrap_or_default());
		if word(0) != VERSION {
			return Err(invalid("format version"));
		}
		if word(1) != kind as u32 {
			return Err(invalid("kind"));
		}
		let (level, count, width, strings_len) =
			(word(2), word(3) as usize, word(4) as usize, word(5) as usize);
		if !(1..=MAX_LEVEL).contains(&level) || width != 8 + 4 * kind.fields() {
			return Err(invalid("header"));
		}
		let records = HEADER + 4 * ((1usize << (2 * level)) + 1);
		let strings = records + width * count;
		if raw.len() != strings + strings_len {
			return Err(invalid("length"));
		}
		let index = Self { level, count, width, records, strings, bytes };
		if index.offset(0) != 0 || index.offset(1usize << (2 * level)) as usize != count {
			return Err(invalid("offset table"));
		}
		Ok(index)
	}

	fn raw(&self) -> &[u8] {
		self.bytes.as_slice()
	}

	fn u32_at(&self, at: usize) -> u32 {
		let raw = self.raw();
		u32::from_le_bytes([raw[at], raw[at + 1], raw[at + 2], raw[at + 3]])
	}

	fn offset(&self, cell: usize) -> u32 {
		self.u32_at(HEADER + 4 * cell)
	}

	/// The position of record `i`, as the `f64` the source text gave.
	pub fn position(&self, i: usize) -> (f64, f64) {
		let at = self.records + i * self.width;
		(degrees(self.u32_at(at) as i32), degrees(self.u32_at(at + 4) as i32))
	}

	/// String field `field` of record `i`: `None` for [`ABSENT`] or a handle that reads as nothing.
	pub fn text(&self, i: usize, field: usize) -> Option<&str> {
		let handle = self.u32_at(self.records + i * self.width + 8 + 4 * field);
		if handle == ABSENT {
			return None;
		}
		let raw = &self.raw()[self.strings..];
		let at = handle as usize;
		let len = u16::from_le_bytes(raw.get(at..at + 2)?.try_into().ok()?) as usize;
		std::str::from_utf8(raw.get(at + 2..at + 2 + len)?).ok()
	}

	/// The records of cell `(x, y)` at `level`, as a range.
	fn run(&self, level: u32, x: u32, y: u32) -> (usize, usize) {
		let shift = 2 * (self.level - level);
		let first = (morton(x, y) << shift) as usize;
		let last = ((morton(x, y) + 1) << shift) as usize;
		let start = (self.offset(first) as usize).min(self.count);
		let end = (self.offset(last) as usize).min(self.count);
		(start, end.max(start))
	}

	/// The record nearest `(lat, lon)` in squared degrees, the ordering the R-tree before this
	/// used; see `Gazetteer::lookup`. Ties go to the earlier record, which for points at one
	/// position is the earlier line of the source. `None` when empty or the position is not finite.
	pub fn nearest(&self, lat: f64, lon: f64) -> Option<usize> {
		if !lat.is_finite() || !lon.is_finite() || self.count == 0 {
			return None;
		}
		let mut best: Option<(f64, usize)> = None;
		let mut queue = BinaryHeap::new();
		queue.push(Cell { distance: 0.0, level: 0, x: 0, y: 0 });
		while let Some(cell) = queue.pop() {
			// Strictly greater: a cell exactly as far as the best may still hold an earlier tie.
			if best.is_some_and(|(distance, _)| cell.distance > distance) {
				break;
			}
			let (start, end) = self.run(cell.level, cell.x, cell.y);
			if start == end {
				continue;
			}
			if cell.level == self.level || end - start <= LEAF {
				for i in start..end {
					let (plat, plon) = self.position(i);
					let dx = plon - lon;
					let dy = plat - lat;
					let distance = dx * dx + dy * dy;
					if best.is_none_or(|(d, at)| distance < d || (distance == d && i < at)) {
						best = Some((distance, i));
					}
				}
				continue;
			}
			let level = cell.level + 1;
			for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
				let (x, y) = (2 * cell.x + dx, 2 * cell.y + dy);
				let distance = self.gap(level, x, y, lat, lon);
				if best.is_none_or(|(d, _)| distance <= d) {
					queue.push(Cell { distance, level, x, y });
				}
			}
		}
		best.map(|(_, i)| i)
	}

	/// The least squared-degree distance from `(lat, lon)` to any point cell `(x, y)` can hold,
	/// computed with the same subtractions a point's distance is, so it is never larger than one.
	fn gap(&self, level: u32, x: u32, y: u32, lat: f64, lon: f64) -> f64 {
		fn along(value: f64, (low, high): (f64, f64)) -> f64 {
			if value < low {
				low - value
			} else if value > high {
				value - high
			} else {
				0.0
			}
		}
		let dx = along(lon, bounds(x, LON_HALF, LON_SPAN, level));
		let dy = along(lat, bounds(y, LAT_HALF, LAT_SPAN, level));
		dx * dx + dy * dy
	}
}

/// A cell waiting to be searched, ordered nearest first in a max-heap.
struct Cell {
	distance: f64,
	level: u32,
	x: u32,
	y: u32,
}

impl PartialEq for Cell {
	fn eq(&self, other: &Self) -> bool {
		self.cmp(other) == Ordering::Equal
	}
}

impl Eq for Cell {}

impl PartialOrd for Cell {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for Cell {
	fn cmp(&self, other: &Self) -> Ordering {
		other.distance.total_cmp(&self.distance)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_coarse_cell_is_the_run_of_its_fine_cells() {
		for level in 1..=4u32 {
			for x in 0..(1u32 << level) {
				for y in 0..(1u32 << level) {
					let key = morton(x, y);
					for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
						assert_eq!(morton(2 * x + dx, 2 * y + dy) >> 2, key);
					}
				}
			}
		}
	}

	#[test]
	fn a_position_reads_back_as_the_number_the_text_parsed_to() {
		for text in
			["42.50729", "-179.99999", "0.00001", "89.9999", "-33.86785", "151.20732", "180", "-90"]
		{
			let parsed: f64 = text.parse().unwrap();
			assert_eq!(degrees(fixed(parsed)), parsed, "{text}");
		}
	}

	#[test]
	fn every_value_lies_inside_the_bounds_of_its_slot() {
		for level in [1, 5, 10, MAX_LEVEL] {
			let n = 1u32 << level;
			for index in [0, 1, n / 2 - 1, n / 2, n - 2, n - 1] {
				let (low, high) = bounds(index, LON_HALF, LON_SPAN, level);
				// The first and last fixed-point values the slot is given, and their neighbors.
				let first = ((u64::from(index) * LON_SPAN).div_ceil(1 << level) as i64 - LON_HALF) as i32;
				for value in [first - 1, first, first + 1] {
					let inside = slot(value, LON_HALF, LON_SPAN, level) == index;
					if inside {
						assert!(low <= degrees(value) && degrees(value) <= high, "{level} {index} {value}");
					}
				}
			}
		}
		assert_eq!(slot(fixed(180.0), LON_HALF, LON_SPAN, 10), 1023);
		assert_eq!(slot(fixed(-180.0), LON_HALF, LON_SPAN, 10), 0);
		assert_eq!(slot(fixed(90.0), LAT_HALF, LAT_SPAN, 10), 1023);
	}

	#[test]
	fn the_nearest_is_the_nearest_at_every_level() {
		let mut random = super::super::reference::Random(5);
		let mut degrees = |span: f64| ((random.unit() - 0.5) * span * 1e5).round() / 1e5;
		let points: Vec<(f64, f64)> = (0..1500).map(|_| (degrees(180.0), degrees(360.0))).collect();
		let queries: Vec<(f64, f64)> = (0..1500).map(|_| (degrees(180.0), degrees(360.0))).collect();
		let distance =
			|(a, b): (f64, f64), (lat, lon): (f64, f64)| (b - lon) * (b - lon) + (a - lat) * (a - lat);
		for level in [1, 2, 4, 7, MAX_LEVEL] {
			let entries =
				points.iter().map(|&(lat, lon)| Entry { lat, lon, fields: [ABSENT; 2] }).collect();
			let index =
				Index::from_vec(encode(Kind::Postal, level, entries, Strings::default()), Kind::Postal)
					.unwrap();
			for &query in queries.iter().chain(&points) {
				let best = points.iter().map(|&p| distance(p, query)).fold(f64::INFINITY, f64::min);
				let found = index.nearest(query.0, query.1).unwrap();
				assert_eq!(distance(index.position(found), query), best, "level {level} at {query:?}");
			}
		}
	}

	#[test]
	fn a_damaged_file_is_refused_rather_than_read() {
		let mut strings = Strings::default();
		let name = strings.add("here");
		let entries = vec![Entry { lat: 1.0, lon: 2.0, fields: [name, ABSENT] }];
		let bytes = encode(Kind::Postal, 3, entries, strings);
		assert!(Index::from_vec(bytes.clone(), Kind::Places).is_err());
		assert!(Index::from_vec(bytes[..bytes.len() - 1].to_vec(), Kind::Postal).is_err());
		let index = Index::from_vec(bytes, Kind::Postal).unwrap();
		assert_eq!((index.text(0, 0), index.text(0, 1)), (Some("here"), None));
		assert_eq!(index.nearest(f64::NAN, 0.0), None);
	}
}
