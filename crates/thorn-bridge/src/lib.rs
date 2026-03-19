mod extract;
pub mod validate;

use pyo3::prelude::*;
use thorn_api::graph::ModelGraph;
use thorn_api::Diagnostic;

/// Boot Django and extract the full model graph.
///
/// This calls `django.setup()` in-process via PyO3, then walks
/// `django.apps.apps.get_models()` to build the complete graph
/// including fields, relations, managers, and methods.
pub fn extract_model_graph(settings_module: &str) -> Result<ModelGraph, PyErr> {
    Python::with_gil(|py| {
        boot_django(py, settings_module)?;
        extract::extract_graph(py)
    })
}

/// Boot Django, extract the graph, AND run dynamic validation checks.
///
/// This is the full pipeline: boot Django once, extract model metadata,
/// then actually execute Django code to validate models, serializers,
/// URLs, and migrations. Returns both the graph and diagnostics.
pub fn extract_and_validate(
    settings_module: &str,
) -> Result<(ModelGraph, Vec<Diagnostic>), PyErr> {
    Python::with_gil(|py| {
        boot_django(py, settings_module)?;
        let graph = extract::extract_graph(py)?;
        let diagnostics = validate::run_all_dynamic_checks(py)?;
        Ok((graph, diagnostics))
    })
}

/// Run only the dynamic validation checks (Django must already be booted).
pub fn run_dynamic_checks(settings_module: &str) -> Result<Vec<Diagnostic>, PyErr> {
    Python::with_gil(|py| {
        boot_django(py, settings_module)?;
        validate::run_all_dynamic_checks(py)
    })
}

/// Set DJANGO_SETTINGS_MODULE and call django.setup().
fn boot_django(py: Python<'_>, settings_module: &str) -> PyResult<()> {
    let os = py.import("os")?;
    let environ = os.getattr("environ")?;
    environ.call_method1("setdefault", ("DJANGO_SETTINGS_MODULE", settings_module))?;

    let django = py.import("django")?;
    django.call_method0("setup")?;
    Ok(())
}
