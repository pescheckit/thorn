use thorn_api::Plugin;
use thorn_core::Linter;

/// Register all plugins that are enabled via cargo features.
pub fn register_all(linter: &mut Linter) {
    #[cfg(feature = "plugin-django")]
    {
        linter.register(Box::new(thorn_django::DjangoPlugin::new()));
    }
}

/// Ask all enabled plugins to read their framework-specific excludes
/// from pyproject.toml content.
pub fn collect_config_excludes(toml_content: &str) -> Vec<String> {
    let mut excludes = Vec::new();

    #[cfg(feature = "plugin-django")]
    {
        let plugin = thorn_django::DjangoPlugin::new();
        excludes.extend(plugin.read_config_excludes(toml_content));
    }

    excludes
}
