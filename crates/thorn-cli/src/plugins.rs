use thorn_api::Plugin;

/// Create all enabled plugin instances.
/// Plugins are registered by downstream crates that wrap thorn-cli,
/// not by thorn itself. thorn is framework-agnostic.
pub fn create_all() -> Vec<Box<dyn Plugin>> {
    vec![]
}
