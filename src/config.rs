/* src/config.rs */

/// Configuration for `GovernorLayer`.
///
/// This struct allows you to customize the behavior of the rate-limiting middleware.
#[derive(Debug, Clone, Default)]
pub struct GovernorConfig {
    /// If `true`, the middleware will use `lazy_limit::limit_override!`,
    /// which ignores the global rate limit and only applies route-specific rules.
    ///
    /// If `false` (default), it uses `lazy_limit::limit!`, which enforces the stricter
    /// of the global and route-specific rules.
    pub override_mode: bool,
}

impl GovernorConfig {
    /// Creates a new `GovernorConfig` with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the override mode.
    ///
    /// - `true`: Ignores the global rate limit.
    /// - `false`: Enforces both global and route-specific limits.
    pub fn override_mode(mut self, override_mode: bool) -> Self {
        self.override_mode = override_mode;
        self
    }
}
