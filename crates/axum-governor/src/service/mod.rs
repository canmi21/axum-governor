//! `tower::Service` per-request flow.
//!
//! The precedence rules that turn a key into an admit or a reject live in `decide`; the
//! headers and reject bodies they produce are built in `respond`. This file holds only the
//! `Service` and `Future` plumbing that threads a request through them.

mod decide;
mod respond;

use std::future::Future;
use std::hash::Hash;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use http::{HeaderMap, Request, Response};
use pin_project_lite::pin_project;

use crate::builder::ExtractorSlot;
use crate::layer::LimiterShared;

use self::decide::{Decision, decide, is_whitelisted};

#[derive(Clone)]
pub struct Governor<S, K>
where
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	pub(crate) inner: S,
	pub(crate) shared: Arc<LimiterShared<K>>,
}

pin_project! {
	pub struct GovernorFuture<F, E> {
		#[pin] state: GovernorFutureState<F, E>,
	}
}

pin_project! {
	#[project = StateProj]
	enum GovernorFutureState<F, E> {
		Admit {
			#[pin] inner: F,
			headers: Option<HeaderMap>,
		},
		Reject {
			response: Option<Response<axum::body::Body>>,
		},
		// Boxed state for the async extractor branch.
		// Pin<Box<...>> is Unpin itself, so no #[pin] attribute is needed.
		Boxed {
			fut: Option<Pin<Box<dyn Future<Output = Result<Response<axum::body::Body>, E>> + Send>>>,
		},
	}
}

impl<S, K, ReqBody> tower::Service<Request<ReqBody>> for Governor<S, K>
where
	S: tower::Service<Request<ReqBody>, Response = Response<axum::body::Body>>
		+ Clone
		+ Send
		+ 'static,
	S::Future: Send + 'static,
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
	ReqBody: Send + 'static,
{
	type Response = Response<axum::body::Body>;
	type Error = S::Error;
	type Future = GovernorFuture<S::Future, S::Error>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), S::Error>> {
		self.inner.poll_ready(cx)
	}

	fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
		match &self.shared.extractor {
			ExtractorSlot::Async(_) => {
				call_async_dispatch(Arc::clone(&self.shared), self.inner.clone(), req)
			}
			_ => call_sync(&mut self.inner, &self.shared, req),
		}
	}
}

fn call_sync<S, K, ReqBody>(
	inner: &mut S,
	shared: &Arc<LimiterShared<K>>,
	req: Request<ReqBody>,
) -> GovernorFuture<S::Future, S::Error>
where
	S: tower::Service<Request<ReqBody>, Response = Response<axum::body::Body>>,
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
{
	let (parts, body) = req.into_parts();

	#[cfg(feature = "tracing")]
	let _span_guard = crate::trace::span_for(&parts.method, parts.uri.path()).entered();

	if is_whitelisted(&shared.settings, &parts) {
		return GovernorFuture::admit(inner.call(Request::from_parts(parts, body)), HeaderMap::new());
	}

	let outcome = match &shared.extractor {
		ExtractorSlot::Sync(e) => e.extract(&parts),
		ExtractorSlot::Async(_) => unreachable!("call_sync dispatched for sync extractor only"),
		ExtractorSlot::None => unreachable!("guarded at GovernorConfigBuilder::finish"),
	};

	match decide(shared, &parts, outcome) {
		Decision::Reject(response) => GovernorFuture::reject(response),
		Decision::Admit(headers) => {
			GovernorFuture::admit(inner.call(Request::from_parts(parts, body)), headers)
		}
	}
}

fn call_async_dispatch<S, K, ReqBody>(
	shared: Arc<LimiterShared<K>>,
	mut inner: S,
	req: Request<ReqBody>,
) -> GovernorFuture<S::Future, S::Error>
where
	S: tower::Service<Request<ReqBody>, Response = Response<axum::body::Body>> + Send + 'static,
	S::Future: Send + 'static,
	K: Hash + Eq + Clone + std::fmt::Debug + Send + Sync + 'static,
	ReqBody: Send + 'static,
{
	let (parts, body) = req.into_parts();

	type BoxedFut<E> = Pin<Box<dyn Future<Output = Result<Response<axum::body::Body>, E>> + Send>>;

	#[cfg(feature = "tracing")]
	let _async_span = crate::trace::span_for(&parts.method, parts.uri.path());

	let inner_fut = async move {
		if is_whitelisted(&shared.settings, &parts) {
			return inner.call(Request::from_parts(parts, body)).await;
		}

		let outcome = match &shared.extractor {
			ExtractorSlot::Async(e) => e.extract(&parts).await,
			_ => unreachable!("call_async_dispatch dispatched for async extractor only"),
		};

		match decide(&shared, &parts, outcome) {
			Decision::Reject(response) => Ok(response),
			Decision::Admit(headers) => {
				let mut resp = inner.call(Request::from_parts(parts, body)).await?;
				merge_headers(&mut resp, headers);
				Ok(resp)
			}
		}
	};

	#[cfg(feature = "tracing")]
	let fut: BoxedFut<S::Error> = {
		use tracing::Instrument as _;
		Box::pin(inner_fut.instrument(_async_span))
	};
	#[cfg(not(feature = "tracing"))]
	let fut: BoxedFut<S::Error> = Box::pin(inner_fut);

	GovernorFuture { state: GovernorFutureState::Boxed { fut: Some(fut) } }
}

fn merge_headers(resp: &mut Response<axum::body::Body>, extra: HeaderMap) {
	let dst = resp.headers_mut();
	for (name, value) in extra {
		if let Some(n) = name {
			dst.insert(n, value);
		}
	}
}

impl<F, E> GovernorFuture<F, E> {
	fn admit(inner: F, headers: HeaderMap) -> Self {
		Self { state: GovernorFutureState::Admit { inner, headers: Some(headers) } }
	}

	fn reject(response: Response<axum::body::Body>) -> Self {
		Self { state: GovernorFutureState::Reject { response: Some(response) } }
	}
}

impl<F, E> Future for GovernorFuture<F, E>
where
	F: Future<Output = Result<Response<axum::body::Body>, E>>,
{
	type Output = Result<Response<axum::body::Body>, E>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let this = self.project();
		match this.state.project() {
			StateProj::Admit { inner, headers } => match inner.poll(cx) {
				Poll::Ready(Ok(mut resp)) => {
					if let Some(extra) = headers.take() {
						merge_headers(&mut resp, extra);
					}
					Poll::Ready(Ok(resp))
				}
				Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
				Poll::Pending => Poll::Pending,
			},
			StateProj::Reject { response } => {
				Poll::Ready(Ok(response.take().expect("polled GovernorFuture::Reject after completion")))
			}
			// Pin<Box<dyn Future>> is Unpin; poll via as_mut().
			StateProj::Boxed { fut } => {
				let pinned = fut.as_mut().expect("polled GovernorFuture::Boxed after completion");
				pinned.as_mut().poll(cx)
			}
		}
	}
}

#[cfg(test)]
mod tests;
