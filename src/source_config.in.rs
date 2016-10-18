// This is not a standalone Rust module.  It gets processed by serde to
// generate serialization code and included directly into another module.

/// Configuration for an individual source tree.
#[derive(Debug, Clone, Deserialize)]
struct SourceConfig {
    /// The local or remote `context` for this source tree.  We don't
    /// really want to use `dc::RawOr` here, but it's the easiest way to
    /// get this to work with serde, because that's how it works in
    /// `docker-compose.yml` files, and that's what our `compose_yml`
    /// library supports.
    context: dc::RawOr<dc::Context>,
}
