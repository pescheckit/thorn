pub mod config;

use clap::{Arg, Command};
use colored::Colorize;
use std::collections::HashMap;
use std::path::PathBuf;
use thorn_api::Plugin;

use config::ThornConfig;

/// Run the thorn CLI with the given plugins.
/// Call this from your binary with your plugins registered.
pub fn run(plugins_fn: fn() -> Vec<Box<dyn Plugin>>) {
    let mut cmd = Command::new("thorn")
        .version(env!("CARGO_PKG_VERSION"))
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
                .value_parser(["text", "json"])
                .help("Output format"),
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

    // Ask each plugin for its CLI params
    let plugins_tmp = plugins_fn();
    let mut plugin_params: Vec<(String, Vec<(String, bool)>)> = Vec::new();

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
    let list_plugins = matches.get_flag("list-plugins");

    let file_config = ThornConfig::from_project_dir(&path);
    let resolved_path = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
    let pyproject_content = config::find_pyproject(&path)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_default();

    // Initialize plugins
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

        let result = plugin.initialize(&resolved_path, &pyproject_content, &cli_args);
        if !result.graph.models.is_empty() {
            graph = result.graph;
        }
        dynamic_diagnostics.extend(result.diagnostics);
    }

    let mut linter = thorn_core::Linter::new(graph);
    for plugin in plugins {
        linter.register(plugin);
    }

    let mut all_excludes = file_config.exclude.clone();
    all_excludes.extend(excludes);
    if !pyproject_content.is_empty() {
        for plugin_excludes in linter.plugin_config_excludes(&pyproject_content) {
            all_excludes.extend(plugin_excludes);
        }
    }
    if !all_excludes.is_empty() {
        linter.set_excludes(all_excludes);
    }

    if list_plugins {
        println!("Registered plugins:");
        for (name, prefix) in linter.plugin_summary() {
            println!("  [{prefix}] {name}");
        }
        return;
    }

    let mut diagnostics = linter.lint_dir_with_config(&path, &pyproject_content);
    diagnostics.extend(dynamic_diagnostics);

    let min_level = match check.as_str() {
        "fix" => thorn_api::Level::Fix,
        "all" => thorn_api::Level::All,
        _ => thorn_api::Level::Improve,
    };
    diagnostics.retain(|d| d.level <= min_level);

    let mut ignored_codes = file_config.ignore.clone();
    ignored_codes.extend(ignores);
    if !ignored_codes.is_empty() {
        diagnostics.retain(|d| !ignored_codes.contains(&d.code));
    }

    let base = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
    let base_str = base.to_string_lossy();
    for d in &mut diagnostics {
        if d.filename.starts_with(base_str.as_ref()) {
            d.filename = d.filename[base_str.len()..]
                .trim_start_matches('/')
                .to_string();
        }
    }

    match format.as_str() {
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
