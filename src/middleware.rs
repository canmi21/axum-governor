/* src/middleware.rs */

use crate::GovernorConfig;
use axum::{
    body::Body,
    http::{Request, Response, StatusCode},
};
use futures_util::future::BoxFuture;
use real::RealIp;
use std::{
    fmt,
    task::{Context, Poll},
};
use tower::Service;
use tracing::warn;

/// The middleware service that performs rate-limiting.
#[derive(Clone)]
pub struct GovernorMiddleware<S> {
    inner: S,
    config: GovernorConfig,
}

impl<S> GovernorMiddleware<S> {
    pub fn new(inner: S, config: GovernorConfig) -> Self {
        Self { inner, config }
    }
}

impl<S> fmt::Debug for GovernorMiddleware<S>
where
    S: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GovernorMiddleware")
            .field("inner", &self.inner)
            .field("config", &self.config)
            .finish()
    }
}

impl<S, ReqBody> Service<Request<ReqBody>> for GovernorMiddleware<S>
where
    S: Service<Request<ReqBody>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let mut inner = self.inner.clone();
        let config = self.config.clone();

        Box::pin(async move {
            // Extract the RealIp extension. This must be present.
            // Ensure `RealIpLayer` is added *before* `GovernorLayer`.
            let ip_ext = req.extensions().get::<RealIp>();

            if ip_ext.is_none() {
                warn!(
                    "RealIp extension not found. Make sure RealIpLayer is installed before GovernorLayer."
                );
                let response = Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(
                        "Internal Server Error: Rate limiter misconfigured",
                    ))
                    .unwrap();
                return Ok(response);
            }

            let ip_str = ip_ext.unwrap().ip().to_string();
            let path = req.uri().path().to_string();

            let allowed = if config.override_mode {
                lazy_limit::limit_override!(&ip_str, &path).await
            } else {
                lazy_limit::limit!(&ip_str, &path).await
            };

            if allowed {
                // Request is allowed, pass it to the inner service.
                inner.call(req).await
            } else {
                // Request is denied, return `429 Too Many Requests`.
                let response = Response::builder()
                    .status(StatusCode::TOO_MANY_REQUESTS)
                    .body(Body::from("Too Many Requests"))
                    .unwrap();
                Ok(response)
            }
        })
    }
}
