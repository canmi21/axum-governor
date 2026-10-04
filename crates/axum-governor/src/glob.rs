//! Path glob matcher for `whitelist_paths`.
//!
//! `*` matches a single segment; `**` matches zero or more segments.

pub(crate) fn path_matches(pattern: &str, path: &str) -> bool {
	let pat: Vec<&str> = pattern.split('/').collect();
	let pth: Vec<&str> = path.split('/').collect();
	matches_segments(&pat, &pth)
}

fn matches_segments(pattern: &[&str], path: &[&str]) -> bool {
	match (pattern.first(), path.first()) {
		(None, None) => true,
		(Some(&"**"), _) => {
			for i in 0..=path.len() {
				if matches_segments(&pattern[1..], &path[i..]) {
					return true;
				}
			}
			false
		}
		(Some(_), None) | (None, Some(_)) => false,
		(Some(&"*"), Some(_)) => matches_segments(&pattern[1..], &path[1..]),
		(Some(p), Some(q)) if p == q => matches_segments(&pattern[1..], &path[1..]),
		_ => false,
	}
}

#[cfg(test)]
mod tests {
	use super::path_matches;

	#[test]
	fn exact_match() {
		assert!(path_matches("/health", "/health"));
	}

	#[test]
	fn no_extra_segments() {
		assert!(!path_matches("/health", "/health/foo"));
	}

	#[test]
	fn no_prefix_match() {
		assert!(!path_matches("/health", "/healthx"));
	}

	#[test]
	fn single_star_one_segment() {
		assert!(path_matches("/internal/*", "/internal/admin"));
	}

	#[test]
	fn single_star_no_multi_segment() {
		assert!(!path_matches("/internal/*", "/internal/admin/users"));
	}

	#[test]
	fn double_star_one_segment() {
		assert!(path_matches("/internal/**", "/internal/admin"));
	}

	#[test]
	fn double_star_multi_segment() {
		assert!(path_matches("/internal/**", "/internal/admin/users"));
	}

	#[test]
	fn double_star_empty_tail() {
		assert!(path_matches("/internal/**", "/internal"));
	}

	#[test]
	fn double_star_middle() {
		assert!(path_matches("/api/**/admin", "/api/v1/admin"));
	}

	#[test]
	fn double_star_middle_empty() {
		assert!(path_matches("/api/**/admin", "/api/admin"));
	}

	#[test]
	fn leading_double_star() {
		assert!(path_matches("**", "/anything/at/all"));
	}
}
