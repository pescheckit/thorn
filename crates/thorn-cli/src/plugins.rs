use thorn_api::Plugin;

/// Create all enabled plugin instances.
pub fn create_all() -> Vec<Box<dyn Plugin>> {
    let mut plugins: Vec<Box<dyn Plugin>> = Vec::new();

    #[cfg(feature = "plugin-django")]
    {
        plugins.push(Box::new(thorn_django::DjangoPlugin::new()));
    }

    plugins
}
