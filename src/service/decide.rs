//! Which limiter a request meets and what it said: the precedence rules shared by the sync
//! and async paths, so they exist in exactly one place.

use std::hash::Hash;
use std::sync::Arc;
use std::time::Duration;

use http::{HeaderMap, Response};

use crate::Quota;
use crate::builder::Settings;
use crate::error::RejectionReason;
use crate::extractor::KeyOutcome;
use crate::layer::{KeyedRateLimiter, LimiterShared, PolicyEntry};
use crate::limiters::StackedResult;
use crate::tracker::{EvictionReason, KeyTracker};

use super::respond::{build_admit_headers, build_base_response, build_reject_response};

/// What the limiter decided for one request, once the key is known.
pub(super) enum Decision {
	Reject(Response<axum::body::Body>),
	/// Headers to copy onto the inner response; empty when no policy applies.
	Admit(HeaderMap),
}

/// A whitelist hit bypasses the limiter with no header injection.
pub(super) fn is_whitelisted(settings: &Settings, parts: &http::request::Parts) -> bool {
	settings.whitelist_methods.contains(&parts.method)
		|| settings.whitelist_paths.iter().any(|p| crate::glob::path_matches(p, parts.uri.path()))
		|| crate::extractor::ip::peer_ip(parts)
			.is_some_and(|ip| settings.whitelist_ips.iter().any(|n| n.contains(&ip)))
}

pub(super) fn decide<K>(
	shared: &LimiterShared<K>,
	parts: &http::request::Parts,
	outcome: Result<KeyOutcome<K>, crate::ExtractionError>,
) -> Decision
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	let cfg = &shared.settings;

	let outcome = match outcome {
		Ok(o) => o,
		Err(err) => {
			crate::trace::extraction_failed(&err);
			return Decision::Reject(build_base_response(cfg, RejectionReason::KeyExtractionFailed(err)));
		}
	};

	let policy_entry = pick_policy_entry(shared, &parts.method, outcome.quota_override.is_some());
	let dispatch = pick_primary(shared, &parts.method, outcome.quota_override);
	let primary_result = dispatch.as_ref().map(|d| d.run(&outcome.key));

	if let Some(LimiterOutcome::Reject { wait, quota }) = primary_result {
		let key_repr = crate::util::format_key(&outcome.key, cfg.redact_keys);
		crate::trace::reject(&key_repr, quota.inner().burst_size().get(), wait.as_millis(), "default");
		let label = Arc::clone(&shared.default_label);
		let reason = RejectionReason::QuotaExceeded {
			wait,
			snapshot: synthetic_snapshot(quota),
			key: Box::new(outcome.key.clone()) as Box<dyn std::any::Any + Send>,
			policy_name: Arc::clone(&label),
		};
		return Decision::Reject(build_reject_response(
			cfg,
			quota,
			wait,
			reason,
			&label,
			policy_entry,
			shared,
		));
	}

	let primary_remaining = match primary_result {
		Some(LimiterOutcome::Admit { remaining }) => Some(remaining),
		_ => None,
	};
	let mut lowest_remaining = primary_remaining.unwrap_or(u32::MAX);

	// Walk the stack in order; the first reject wins.
	for entry in shared.stack.iter() {
		match entry.check(parts, cfg.redact_keys) {
			StackedResult::ExtractionFailed(err) => {
				crate::trace::extraction_failed(&err);
				return Decision::Reject(build_base_response(
					cfg,
					RejectionReason::KeyExtractionFailed(err),
				));
			}
			StackedResult::Reject { wait, key_repr } => {
				let quota = entry.quota();
				let name = entry.name_arc();
				crate::trace::reject(&key_repr, quota.inner().burst_size().get(), wait.as_millis(), &name);
				let reason = RejectionReason::QuotaExceeded {
					wait,
					snapshot: synthetic_snapshot(quota),
					// Stack entries are type-erased, so the key itself cannot be carried
					// here; the formatted key is in the tracing event.
					key: Box::new(()) as Box<dyn std::any::Any + Send>,
					policy_name: Arc::clone(&name),
				};
				return Decision::Reject(build_reject_response(
					cfg,
					quota,
					wait,
					reason,
					&name,
					policy_entry,
					shared,
				));
			}
			StackedResult::Admit { remaining } => lowest_remaining = lowest_remaining.min(remaining),
		}
	}

	if dispatch.is_none() && shared.stack.is_empty() {
		return Decision::Admit(HeaderMap::new());
	}

	// The RateLimit: header names one policy: the primary when there is one, otherwise the
	// first advertised stack entry.
	let (admit_quota, admit_name) = if let Some(d) = dispatch.as_ref() {
		(d.quota, Arc::clone(&shared.default_label))
	} else if let Some(first) = policy_entry.and_then(|e| e.descriptors.first()) {
		(first.quota, Arc::clone(&first.name))
	} else {
		return Decision::Admit(HeaderMap::new());
	};

	let final_remaining = if lowest_remaining == u32::MAX { 0 } else { lowest_remaining };
	Decision::Admit(build_admit_headers(
		cfg.legacy_reset_epoch,
		admit_quota,
		&admit_name,
		final_remaining,
		policy_entry,
		shared,
		outcome.quota_override,
	))
}

enum LimiterOutcome {
	Admit { remaining: u32 },
	Reject { wait: Duration, quota: Quota },
}

/// Bundle the limiter, tracker and quota chosen by the precedence rules so the
/// dispatch site doesn't have to thread three options through the rest of the
/// flow. `Dispatch::run` performs the check and the tracker bookkeeping in a
/// single call so the hot path stays linear.
struct Dispatch<'a, K>
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	limiter: LimiterRef<'a, K>,
	tracker: Option<&'a KeyTracker<K>>,
	quota: Quota,
}

enum LimiterRef<'a, K>
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	Borrowed(&'a KeyedRateLimiter<K>),
	Owned(Arc<KeyedRateLimiter<K>>),
}

impl<K> LimiterRef<'_, K>
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	fn as_ref(&self) -> &KeyedRateLimiter<K> {
		match self {
			Self::Borrowed(l) => l,
			Self::Owned(arc) => arc.as_ref(),
		}
	}
}

impl<K> Dispatch<'_, K>
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	fn run(&self, key: &K) -> LimiterOutcome {
		let outcome = check_limiter(self.limiter.as_ref(), key, self.quota);
		if let Some(tracker) = self.tracker
			&& tracker.touch(key) == Some(EvictionReason::MaxKeys)
		{
			crate::trace::eviction("default");
			self.limiter.as_ref().retain_recent();
		}
		outcome
	}
}

fn pick_primary<'a, K>(
	shared: &'a LimiterShared<K>,
	method: &http::Method,
	quota_override: Option<Quota>,
) -> Option<Dispatch<'a, K>>
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	if let Some(method_quota) =
		shared.settings.quota_methods.iter().find(|(m, _)| m == method).map(|(_, q)| *q)
	{
		let limiter = shared
			.method_limiters
			.iter()
			.find(|(m, _)| m == method)
			.map(|(_, l)| l)
			.expect("method_limiters and quota_methods are built in parallel");
		let tracker = shared.method_trackers.iter().find(|(m, _)| m == method).map(|(_, t)| t);
		return Some(Dispatch { limiter: LimiterRef::Borrowed(limiter), tracker, quota: method_quota });
	}

	if let Some(override_quota) = quota_override {
		let limiter = shared.tier_cache.get_or_insert(override_quota);
		// Tier-cache buckets have their own state stores per quota; we don't track
		// the override path through the per-limiter LRU since hit counts would split
		// across (key, quota) combinations. The default tracker still sees the key,
		// so top_n reflects activity faithfully when the user opts into max_keys.
		return Some(Dispatch {
			limiter: LimiterRef::Owned(limiter),
			tracker: shared.default_tracker.as_ref(),
			quota: override_quota,
		});
	}

	if let Some(limiter) = shared.default_limiter.as_ref() {
		let q = shared.settings.quota_default.expect("default_limiter present => quota_default Some");
		return Some(Dispatch {
			limiter: LimiterRef::Borrowed(limiter),
			tracker: shared.default_tracker.as_ref(),
			quota: q,
		});
	}

	None
}

fn pick_policy_entry<'a, K>(
	shared: &'a LimiterShared<K>,
	method: &http::Method,
	has_tier_override: bool,
) -> Option<&'a PolicyEntry>
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	// Per-method overrides shadow the default-quota policy entry.
	if let Some(entry) = shared.policy_per_method.iter().find(|(m, _)| m == method).map(|(_, e)| e) {
		return Some(entry);
	}
	// Tier overrides change the "default" entry's quota at runtime, so the
	// pre-rendered default header is not accurate for that case. Fall back to
	// inline rendering at the call site instead of returning a stale entry.
	if has_tier_override {
		return None;
	}
	shared.policy_default.as_ref()
}

fn check_limiter<K: Hash + Eq + Clone + Send + Sync + 'static>(
	limiter: &KeyedRateLimiter<K>,
	key: &K,
	quota: Quota,
) -> LimiterOutcome {
	use governor::clock::Clock as _;
	match limiter.check_key(key) {
		Ok(snapshot) => LimiterOutcome::Admit { remaining: snapshot.remaining_burst_capacity() },
		Err(not_until) => {
			let now = governor::clock::DefaultClock::default().now();
			LimiterOutcome::Reject { wait: not_until.wait_time_from(now), quota }
		}
	}
}

fn synthetic_snapshot(quota: Quota) -> governor::middleware::StateSnapshot {
	use governor::middleware::StateInformationMiddleware;
	governor::RateLimiter::direct(quota.inner())
		.with_middleware::<StateInformationMiddleware>()
		.check()
		.expect("fresh direct limiter always allows first check")
}
