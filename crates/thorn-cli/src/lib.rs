pub mod config;
pub mod format;

use clap::{Arg, ArgMatches, Command};
use std::collections::HashMap;
use std::path::PathBuf;
use thorn_api::Plugin;

use config::ThornConfig;

/// Run the thorn CLI with the given plugins.
/// Call this from your binary with your plugins registered.
pub fn run(plugins_fn: fn() -> Vec<Box<dyn Plugin>>) {
    let (matches, plugin_params) = parse_args(plugins_fn);

    let path = PathBuf::from(matches.get_one::<String>("path").unwrap());
    let format = matches.get_one::<String>("format").unwrap().clone();
    let check = matches.get_one::<String>("check").unwrap().clone();
    let excludes: Vec<String> = matches
        .get_many::<String>("exclude")
        .map(|v| v.cloned().collect())
        .unwrap_or_default();
    let ignores: Vec<String> = matches
        .get_many::<String>("ignore")
        .map(|v| v.cloned().collect())
        .unwrap_or_default();

    let file_config = ThornConfig::from_project_dir(&path);
    let pyproject_content = config::find_pyproject(&path)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_default();

    let (mut linter, dynamic_diagnostics) =
        init_plugins(plugins_fn, &matches, &plugin_params, &path, &pyproject_content);

    let all_excludes = merge_excludes(&file_config, excludes, &linter, &pyproject_content);
    if !all_excludes.is_empty() {
        linter.set_excludes(all_excludes);
    }

    if matches.get_flag("list-plugins") {
        println!("Registered plugins:");
        for (name, prefix) in linter.plugin_summary() {
            println!("  [{prefix}] {name}");
        }
        return;
    }

    let diagnostics = collect_diagnostics(
        &linter,
        &path,
        &pyproject_content,
        dynamic_diagnostics,
        &check,
        &file_config,
        ignores,
    );

    if format::render(&format, &diagnostics) {
        std::process::exit(1);
    }
}

// ── CLI parsing ────────────────────────────────────────────────────

type PluginParams = Vec<(String, Vec<(String, bool)>)>;

fn parse_args(plugins_fn: fn() -> Vec<Box<dyn Plugin>>) -> (ArgMatches, PluginParams) {
    let mut cmd = Command::new("thorn")
        .version(env!("THORN_VERSION"))
        .about("A fast linter with live framework introspection")
        .arg(
            Arg::new("path")
                .default_value(".")
                .help("Path to lint (file or directory)"),
        )
        .arg(
            Arg::new("format")
                .long("format")
                .default_value("text")
                .value_parser(["text", "json", "gitlab", "github", "sarif"])
                .help("Output format: text, json, gitlab, github, sarif"),
        )
        .arg(
            Arg::new("exclude")
                .long("exclude")
                .short('e')
                .action(clap::ArgAction::Append)
                .help("Glob patterns to exclude (e.g. \"*/migrations/*\")"),
        )
        .arg(
            Arg::new("check")
                .long("check")
                .default_value("improve")
                .value_parser(["fix", "improve", "all"])
                .help("What to check: fix (bugs), improve (+ perf), all (+ style)"),
        )
        .arg(
            Arg::new("ignore")
                .long("ignore")
                .short('i')
                .action(clap::ArgAction::Append)
                .help("Rule codes to ignore (e.g. --ignore DJ001)"),
        )
        .arg(
            Arg::new("list-plugins")
                .long("list-plugins")
                .action(clap::ArgAction::SetTrue)
                .help("List registered plugins and exit"),
        );

    let plugins_tmp = plugins_fn();
    let mut plugin_params: PluginParams = Vec::new();

    for plugin in &plugins_tmp {
        let params = plugin.cli_params();
        let mut param_info = Vec::new();
        for param in &params {
            let arg_name = format!("{}-{}", plugin.name(), param.name);
            let arg_id: &'static str = Box::leak(arg_name.into_boxed_str());
            let arg = if param.takes_value {
                Arg::new(arg_id).long(arg_id).help(param.help).num_args(1)
            } else {
                Arg::new(arg_id)
                    .long(arg_id)
                    .help(param.help)
                    .action(clap::ArgAction::SetTrue)
            };
            cmd = cmd.arg(arg);
            param_info.push((param.name.to_string(), param.takes_value));
        }
        plugin_params.push((plugin.name().to_string(), param_info));
    }
    drop(plugins_tmp);

    let matches = cmd.get_matches();
    (matches, plugin_params)
}

// ── Plugin initialization ──────────────────────────────────────────

fn init_plugins(
    plugins_fn: fn() -> Vec<Box<dyn Plugin>>,
    matches: &ArgMatches,
    plugin_params: &PluginParams,
    path: &PathBuf,
    pyproject_content: &str,
) -> (thorn_core::Linter, Vec<thorn_api::Diagnostic>) {
    let resolved_path = std::fs::canonicalize(path).unwrap_or_else(|_| path.clone());

    let mut plugins = plugins_fn();
    let mut graph = thorn_api::AppGraph::default();
    let mut dynamic_diagnostics = Vec::new();

    for (i, plugin) in plugins.iter_mut().enumerate() {
        let mut cli_args = HashMap::new();
        if let Some((plugin_name, param_info)) = plugin_params.get(i) {
            for (param_name, takes_value) in param_info {
                let arg_name = format!("{}-{}", plugin_name, param_name);
                if *takes_value {
                    if let Some(val) = matches.get_one::<String>(&arg_name) {
                        cli_args.insert(param_name.clone(), val.clone());
                    }
                } else if matches.get_flag(&arg_name) {
                    cli_args.insert(param_name.clone(), "true".to_string());
                }
            }
        }

        let result = plugin.initialize(&resolved_path, pyproject_content, &cli_args);
        if !result.graph.models.is_empty() {
            graph = result.graph;
        }
        dynamic_diagnostics.extend(result.diagnostics);
    }

    let mut linter = thorn_core::Linter::new(graph);
    for plugin in plugins {
        linter.register(plugin);
    }

    (linter, dynamic_diagnostics)
}

// ── Exclude merging ────────────────────────────────────────────────

fn merge_excludes(
    file_config: &ThornConfig,
    cli_excludes: Vec<String>,
    linter: &thorn_core::Linter,
    pyproject_content: &str,
) -> Vec<String> {
    let mut all = file_config.exclude.clone();
    all.extend(cli_excludes);
    if !pyproject_content.is_empty() {
        for plugin_excludes in linter.plugin_config_excludes(pyproject_content) {
            all.extend(plugin_excludes);
        }
    }
    all
}

// ── Diagnostic collection & filtering ──────────────────────────────

fn collect_diagnostics(
    linter: &thorn_core::Linter,
    path: &PathBuf,
    pyproject_content: &str,
    dynamic_diagnostics: Vec<thorn_api::Diagnostic>,
    check: &str,
    file_config: &ThornConfig,
    cli_ignores: Vec<String>,
) -> Vec<thorn_api::Diagnostic> {
    let mut diagnostics = linter.lint_dir_with_config(path, pyproject_content);
    diagnostics.extend(dynamic_diagnostics);

    let min_level = match check {
        "fix" => thorn_api::Level::Fix,
        "all" => thorn_api::Level::All,
        _ => thorn_api::Level::Improve,
    };
    diagnostics.retain(|d| d.level <= min_level);

    let mut ignored_codes = file_config.ignore.clone();
    ignored_codes.extend(cli_ignores);
    if !ignored_codes.is_empty() {
        diagnostics.retain(|d| !ignored_codes.contains(&d.code));
    }

    // Strip base path prefix for relative display
    let base = std::fs::canonicalize(path).unwrap_or_else(|_| path.clone());
    let base_str = base.to_string_lossy();
    for d in &mut diagnostics {
        if d.filename.starts_with(base_str.as_ref()) {
            d.filename = d.filename[base_str.len()..]
                .trim_start_matches('/')
                .to_string();
        }
    }

    diagnostics
}
