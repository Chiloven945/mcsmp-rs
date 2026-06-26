//! Runtime compatibility behavior for protocol-version differences.

/// Controls how the client treats methods that were not advertised through
/// `rpc.discover`.
///
/// The default is [`CompatibilityMode::Compatible`], which favors practical
/// interoperability with older Minecraft snapshots and extension namespaces.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum CompatibilityMode {
    /// Require a successful capability discovery before any method call and
    /// reject methods the schema does not advertise.
    Strict,

    /// Prefer the typed, stable surface while retaining legacy wire-format
    /// support and allowing extension methods. This is the default mode.
    #[default]
    Compatible,

    /// Never preflight calls against discovery results. This is intended for
    /// experimental servers, proxies, and mod-defined RPC namespaces.
    Permissive,
}
