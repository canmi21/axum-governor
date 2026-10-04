//! Layer-level tests of the request flow, split by topic: `flow` for the path a request
//! takes (whitelists, headers, error handling, async, eviction) and `policy` for which quota
//! it meets (per-method, stacked, tiered).

mod flow;
mod policy;

use http::{Method, Request, Response};

use crate::test_utils::{request, request_with_peer};

fn req(method: Method, path: &str) -> Request<axum::body::Body> {
	request(method, path)
}

fn req_with_peer(method: Method, path: &str, peer: &str) -> Request<axum::body::Body> {
	request_with_peer(method, path, peer.parse().unwrap())
}

fn policy_header(resp: &Response<axum::body::Body>) -> Option<&str> {
	resp.headers().get("ratelimit-policy").map(|v| v.to_str().unwrap())
}
