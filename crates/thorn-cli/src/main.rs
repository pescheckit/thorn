mod config;
mod plugins;

use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;
use std::sync::Mutex;
use thorn_api::{Diagnostic, ModelGraph};
use thorn_core::Linter;

use config::ThornConfig;

static DYNAMIC_DIAGNOSTICS: Mutex<Vec<Diagnostic>> = Mutex::new(Vec::new());

/// Bundle format: { "graph": {...}, "diagnostics": [...] }
#[derive(serde::Deserialize)]
struct GraphBundle {
    graph: ModelGraph,
    #[serde(default)]
    diagnostics: Vec<Diagnostic>,
}

#[derive(Parser)]
#[command(
    name = "thorn",
    version,
    about = "A fast linter with live framework introspection"
)]
struct Args {
    /// Path to lint (file or directory)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Load a pre-extracted graph from a JSON file.
    /// Generate with: docker compose exec app python -m thorn_django > graph.json
    #[arg(long)]
    graph_file: Option<PathBuf>,

    /// Output format
    #[arg(long, default_value = "text", value_parser = ["text", "json"])]
    format: String,

    /// Glob patterns to exclude (e.g. "*/migrations/*")
    #[arg(long, short = 'e')]
    exclude: Vec<String>,

    /// Plugin options (e.g. --thorn-django-settings=myproject.settings.production)
    #[arg(long, value_name = "MODULE")]
    thorn_django_settings: Option<String>,

    /// What to check: "fix" (bugs only), "improve" (bugs + performance), "all" (everything)
    #[arg(long, default_value = "improve", value_parser = ["fix", "improve", "all"])]
    check: String,

    /// Rule codes to ignore (e.g. --ignore DJ001 --ignore DJ026)
    #[arg(long, short = 'i')]
    ignore: Vec<String>,

    /// List registered plugins and exit
    #[arg(long)]
    list_plugins: bool,
}

fn main() {
    let args = Args::parse();

    // Pass plugin CLI args through as env vars so plugins can read them
    if let Some(ref s) = args.thorn_django_settings {
        std::env::set_var("THORN_DJANGO_SETTINGS", s);
    }

    // Load config from pyproject.toml ([tool.thorn])
    let mut file_config = ThornConfig::from_project_dir(&args.path);
    file_config.merge_cli(&args.exclude, &args.graph_file);

    // Resolve the target path to absolute for reliable file discovery
    let resolved_path = std::fs::canonicalize(&args.path).unwrap_or_else(|_| args.path.clone());

    // Resolve graph_file: CLI flag > [tool.thorn] config > auto-discover .thorn/graph.json
    let graph_file = args
        .graph_file
        .clone()
        .or_else(|| file_config.graph_file.as_ref().map(PathBuf::from))
        .or_else(|| {
            let auto_path = resolved_path.join(".thorn/graph.json");
            if auto_path.exists() {
                Some(auto_path)
            } else {
                None
            }
        });

    // Collect plugin config excludes from pyproject.toml
    let mut all_excludes = file_config.exclude.clone();
    let pyproject_content = config::find_pyproject(&args.path)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_default();

    // Build the model graph from --graph-file (or empty if not provided)
    let graph = if let Some(ref graph_path) = graph_file {
        match std::fs::read_to_string(graph_path) {
            Ok(json_str) => {
                // Try bundle format: { graph, diagnostics }
                if let Ok(bundle) = serde_json::from_str::<GraphBundle>(&json_str) {
                    eprintln!(
                        "{} Loaded {} models from {}",
                        "✓".green(),
                        bundle.graph.models.len(),
                        graph_path.display()
                    );
                    if !bundle.diagnostics.is_empty() {
                        eprintln!(
                            "{} Dynamic validation found {} issue{}",
                            "✓".green(),
                            bundle.diagnostics.len(),
                            if bundle.diagnostics.len() == 1 {
                                ""
                            } else {
                                "s"
                            }
                        );
                        DYNAMIC_DIAGNOSTICS
                            .lock()
                            .unwrap()
                            .extend(bundle.diagnostics);
                    }
                    bundle.graph
                }
                // Try plain graph format: { models, ... }
                else if let Ok(g) = serde_json::from_str::<ModelGraph>(&json_str) {
                    eprintln!(
                        "{} Loaded {} models from {}",
                        "✓".green(),
                        g.models.len(),
                        graph_path.display()
                    );
                    g
                } else {
                    eprintln!("{} Failed to parse graph file", "✗".red());
                    ModelGraph::default()
                }
            }
            Err(e) => {
                eprintln!("{} Failed to read graph file: {e}", "✗".red());
                ModelGraph::default()
            }
        }
    } else {
        ModelGraph::default()
    };

    let has_graph = !graph.models.is_empty();

    // Build the linter and register plugins
    let mut linter = Linter::new(graph);
    plugins::register_all(&mut linter);

    // Ask each plugin for framework-specific excludes from pyproject.toml
    if !pyproject_content.is_empty() {
        all_excludes.extend(plugins::collect_config_excludes(&pyproject_content));
    }

    if !all_excludes.is_empty() {
        linter.set_excludes(all_excludes.clone());
    }

    if args.list_plugins {
        println!("Registered plugins:");
        for (name, prefix) in linter.plugin_summary() {
            println!("  [{prefix}] {name}");
        }
        return;
    }

    if !has_graph {
        let settings_module = std::env::var("THORN_DJANGO_SETTINGS")
            .or_else(|_| std::env::var("DJANGO_SETTINGS_MODULE"))
            .ok();
        let mut generated = false;

        if let Some(ref settings) = settings_module {
            // 1. Try PyO3 in-process (fastest — if Django is importable)
            #[cfg(feature = "plugin-django")]
            {
                if let Ok((g, dv)) = thorn_django::bridge::extract_and_validate(settings) {
                    eprintln!("{} Loaded {} models via PyO3", "✓".green(), g.models.len());
                    if !dv.is_empty() {
                        DYNAMIC_DIAGNOSTICS.lock().unwrap().extend(dv);
                    }
                    linter = Linter::new(g);
                    plugins::register_all(&mut linter);
                    if !all_excludes.is_empty() {
                        linter.set_excludes(all_excludes.clone());
                    }
                    generated = true;
                }
            }

            // 2. Try subprocess (if python3 + thorn_django + Django on host)
            if !generated {
                let graph_target = resolved_path.join(".thorn/graph.json");
                for python in &["python3", "python"] {
                    let ok = std::process::Command::new(python)
                        .args(["-m", "thorn_django", "--settings", settings])
                        .current_dir(&resolved_path)
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .status()
                        .map(|s| s.success())
                        .unwrap_or(false);
                    if ok && graph_target.exists() {
                        if let Ok(s) = std::fs::read_to_string(&graph_target) {
                            if let Ok(b) = serde_json::from_str::<GraphBundle>(&s) {
                                eprintln!(
                                    "{} Loaded {} models via python",
                                    "✓".green(),
                                    b.graph.models.len()
                                );
                                if !b.diagnostics.is_empty() {
                                    DYNAMIC_DIAGNOSTICS.lock().unwrap().extend(b.diagnostics);
                                }
                                linter = Linter::new(b.graph);
                                plugins::register_all(&mut linter);
                                if !all_excludes.is_empty() {
                                    linter.set_excludes(all_excludes.clone());
                                }
                                generated = true;
                                break;
                            }
                        }
                    }
                }
            }
        }

        if !generated {
            eprintln!(
                "{} No .thorn/graph.json and no Django environment found.\n  \
                 Generate once: python -m thorn_django --settings myproject.settings\n  \
                 Or in Docker:  docker compose exec app python -m thorn_django",
                "!".yellow(),
            );
        }
    } else if let Some(ref gf) = graph_file {
        // Check if graph is stale — any models/*.py newer than graph.json?
        if let Ok(graph_modified) = std::fs::metadata(gf).and_then(|m| m.modified()) {
            let has_newer = walkdir::WalkDir::new(&args.path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let p = e.path().to_string_lossy();
                    p.ends_with(".py")
                        && (p.contains("models") || p.contains("model"))
                        && !p.contains("site-packages")
                        && !p.contains("/.venv/")
                        && !p.contains("migrations")
                })
                .any(|e| {
                    e.metadata()
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .map(|t| t > graph_modified)
                        .unwrap_or(false)
                });
            if has_newer {
                eprintln!(
                    "{} .thorn/graph.json may be stale — model files have changed.\n  \
                     Regenerate with: docker compose exec app python -m thorn_django",
                    "!".yellow(),
                );
            }
        }
    }

    let mut diagnostics = linter.lint_dir_with_config(&args.path, &pyproject_content);
    let dynamic_diags: Vec<Diagnostic> = DYNAMIC_DIAGNOSTICS.lock().unwrap().drain(..).collect();

    // When dynamic validation covers the same check, suppress the static equivalent.
    // DV001 (runtime MRO walk) supersedes DJ101 (graph-based __str__ check).
    let has_dv001 = dynamic_diags.iter().any(|d| d.code == "DV001");
    if has_dv001 {
        diagnostics.retain(|d| d.code != "DJ101");
    }
    diagnostics.extend(dynamic_diags);

    // Filter out dynamic diagnostics from third-party/site-packages paths.
    // DV diagnostics with module-name filenames (no "/" or ".py") are from
    // third-party packages whose source file couldn't be resolved to a local path.
    diagnostics.retain(|d| {
        if d.code.starts_with("DV")
            && d.code != "DV-WARN"
            && d.code != "DV-ERR"
            && d.code != "DV-CRIT"
        {
            let f = &d.filename;
            if f.contains("site-packages") || f.contains("/venv/") || f.contains("/.venv/") {
                return false;
            }
            // Module-only filenames (e.g. "qualificationcheck.forms") without path separators
            // are from third-party packages
            if !f.contains('/')
                && !f.contains(".py")
                && f != "migrations"
                && f != "django.checks"
                && f != "settings"
            {
                return false;
            }
        }
        true
    });

    // Filter by check level
    // "fix"     = only bugs & security (HIGH)
    // "improve" = bugs + performance + deprecation (HIGH + MED) — default
    // "all"     = everything including style suggestions (HIGH + MED + LOW)
    let min_level = match args.check.as_str() {
        "fix" => thorn_api::Level::Fix,
        "all" => thorn_api::Level::All,
        _ => thorn_api::Level::Improve,
    };
    diagnostics.retain(|d| d.level <= min_level);

    // Filter by ignored codes (CLI --ignore + [tool.thorn] ignore)
    let mut ignored_codes = file_config.ignore.clone();
    ignored_codes.extend(args.ignore.iter().cloned());
    if !ignored_codes.is_empty() {
        diagnostics.retain(|d| !ignored_codes.contains(&d.code));
    }

    // Make paths relative to the scanned directory for clickable output
    let base = std::fs::canonicalize(&args.path).unwrap_or_else(|_| args.path.clone());
    let base_str = base.to_string_lossy();
    for d in &mut diagnostics {
        if d.filename.starts_with(base_str.as_ref()) {
            d.filename = d.filename[base_str.len()..]
                .trim_start_matches('/')
                .to_string();
        }
    }

    match args.format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&diagnostics).unwrap();
            println!("{json}");
        }
        _ => {
            for d in &diagnostics {
                let code = d.code.red().bold();
                let location = match (d.line, d.col) {
                    (Some(line), Some(col)) => format!("{}:{}:{}", d.filename, line, col),
                    (Some(line), None) => format!("{}:{}", d.filename, line),
                    _ => d.filename.clone(),
                };
                let level = d.level.label().dimmed();
                // Collapse multi-line messages (e.g. DV202 migration lists) to a single line
                let msg = d
                    .message
                    .lines()
                    .map(|l| l.trim())
                    .filter(|l| !l.is_empty())
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("{}  {} {} {}", location, level, code, msg);
            }
            if diagnostics.is_empty() {
                eprintln!("{} No issues found.", "✓".green());
            } else {
                eprintln!(
                    "\n{} Found {} issue{}.",
                    "✗".red(),
                    diagnostics.len(),
                    if diagnostics.len() == 1 { "" } else { "s" }
                );
                std::process::exit(1);
            }
        }
    }
}
