/* src/layer.rs */

use crate::{GovernorConfig, GovernorMiddleware};
use std::clone::Clone;

/// A `tower::Layer` that applies rate-limiting to requests.
///
/// This layer wraps an inner service with the `GovernorMiddleware`. It requires
/// that the `real::RealIpLayer` has been applied beforehand to make the
/// client's IP address available in the request extensions.
#[derive(Debug, Clone)]
pub struct GovernorLayer {
    config: GovernorConfig,
}

impl GovernorLayer {
    /// Creates a new `GovernorLayer` with the given configuration.
    pub fn new(config: GovernorConfig) -> Self {
        Self { config }
    }
}

impl Default for GovernorLayer {
    /// Creates a `GovernorLayer` with the default configuration.
    ///
    /// Default mode uses `lazy_limit::limit!`, enforcing both global and
    /// route-specific rules.
    fn default() -> Self {
        Self {
            config: GovernorConfig::default(),
        }
    }
}

impl<S> tower::Layer<S> for GovernorLayer {
    type Service = GovernorMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        GovernorMiddleware::new(inner, self.config.clone())
    }
}
