use colored::Colorize;
use serde::Serialize;
use thorn_api::Diagnostic;

// ── GitLab Code Quality ─────────────────────────────────────────────

#[derive(Serialize)]
struct GitLabIssue {
    description: String,
    check_name: String,
    fingerprint: String,
    severity: &'static str,
    location: GitLabLocation,
}

#[derive(Serialize)]
struct GitLabLocation {
    path: String,
    lines: GitLabLines,
}

#[derive(Serialize)]
struct GitLabLines {
    begin: u32,
}

fn gitlab_severity(d: &Diagnostic) -> &'static str {
    match d.level {
        thorn_api::Level::Fix => "major",
        thorn_api::Level::Improve => "minor",
        thorn_api::Level::All => "info",
    }
}

fn fingerprint(d: &Diagnostic) -> String {
    let input = format!("{}:{}:{}", d.filename, d.code, d.line.unwrap_or(0));
    format!("{:x}", md5::compute(input.as_bytes()))
}

pub fn gitlab(diagnostics: &[Diagnostic]) -> String {
    let issues: Vec<GitLabIssue> = diagnostics
        .iter()
        .map(|d| GitLabIssue {
            description: format!("{}: {}", d.code, d.message),
            check_name: d.code.clone(),
            fingerprint: fingerprint(d),
            severity: gitlab_severity(d),
            location: GitLabLocation {
                path: d.filename.clone(),
                lines: GitLabLines {
                    begin: d.line.unwrap_or(1),
                },
            },
        })
        .collect();
    serde_json::to_string_pretty(&issues).unwrap()
}

// ── GitHub Actions ──────────────────────────────────────────────────

pub fn github(diagnostics: &[Diagnostic]) -> String {
    let mut out = String::new();
    for d in diagnostics {
        let level = match d.level {
            thorn_api::Level::Fix => "error",
            thorn_api::Level::Improve => "warning",
            thorn_api::Level::All => "notice",
        };
        let mut attrs = format!("file={}", d.filename);
        if let Some(line) = d.line {
            attrs.push_str(&format!(",line={line}"));
        }
        if let Some(col) = d.col {
            attrs.push_str(&format!(",col={col}"));
        }
        attrs.push_str(&format!(",title={}", d.code));
        out.push_str(&format!("::{level} {attrs}::{}\n", d.message));
    }
    out
}

// ── SARIF v2.1.0 ───────────────────────────────────────────────────

#[derive(Serialize)]
struct Sarif {
    #[serde(rename = "$schema")]
    schema: &'static str,
    version: &'static str,
    runs: Vec<SarifRun>,
}

#[derive(Serialize)]
struct SarifRun {
    tool: SarifTool,
    results: Vec<SarifResult>,
}

#[derive(Serialize)]
struct SarifTool {
    driver: SarifDriver,
}

#[derive(Serialize)]
struct SarifDriver {
    name: &'static str,
    version: &'static str,
}

#[derive(Serialize)]
struct SarifResult {
    #[serde(rename = "ruleId")]
    rule_id: String,
    level: &'static str,
    message: SarifMessage,
    locations: Vec<SarifLocation>,
}

#[derive(Serialize)]
struct SarifMessage {
    text: String,
}

#[derive(Serialize)]
struct SarifLocation {
    #[serde(rename = "physicalLocation")]
    physical_location: SarifPhysicalLocation,
}

#[derive(Serialize)]
struct SarifPhysicalLocation {
    #[serde(rename = "artifactLocation")]
    artifact_location: SarifArtifactLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    region: Option<SarifRegion>,
}

#[derive(Serialize)]
struct SarifArtifactLocation {
    uri: String,
}

#[derive(Serialize)]
struct SarifRegion {
    #[serde(rename = "startLine")]
    start_line: u32,
    #[serde(rename = "startColumn", skip_serializing_if = "Option::is_none")]
    start_column: Option<u32>,
}

fn sarif_level(d: &Diagnostic) -> &'static str {
    match d.level {
        thorn_api::Level::Fix => "error",
        thorn_api::Level::Improve => "warning",
        thorn_api::Level::All => "note",
    }
}

pub fn sarif(diagnostics: &[Diagnostic]) -> String {
    let results: Vec<SarifResult> = diagnostics
        .iter()
        .map(|d| SarifResult {
            rule_id: d.code.clone(),
            level: sarif_level(d),
            message: SarifMessage {
                text: d.message.clone(),
            },
            locations: vec![SarifLocation {
                physical_location: SarifPhysicalLocation {
                    artifact_location: SarifArtifactLocation {
                        uri: d.filename.clone(),
                    },
                    region: d.line.map(|line| SarifRegion {
                        start_line: line,
                        start_column: d.col,
                    }),
                },
            }],
        })
        .collect();

    let sarif = Sarif {
        schema: "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/sarif-2.1/schema/sarif-schema-2.1.0.json",
        version: "2.1.0",
        runs: vec![SarifRun {
            tool: SarifTool {
                driver: SarifDriver {
                    name: "thorn",
                    version: env!("CARGO_PKG_VERSION"),
                },
            },
            results,
        }],
    };
    serde_json::to_string_pretty(&sarif).unwrap()
}

// ── Text (human-readable, colored) ─────────────────────────────────

pub fn text(diagnostics: &[Diagnostic]) -> String {
    let mut out = String::new();
    for d in diagnostics {
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
        out.push_str(&format!("{}  {} {} {}\n", location, level, code, msg));
    }
    out
}

// ── JSON (raw diagnostics) ─────────────────────────────────────────

pub fn json(diagnostics: &[Diagnostic]) -> String {
    serde_json::to_string_pretty(diagnostics).unwrap()
}

// ── Dispatch ───────────────────────────────────────────────────────

/// Render diagnostics in the given format to stdout/stderr.
/// Returns `true` if there are issues (for exit code purposes).
pub fn render(format: &str, diagnostics: &[Diagnostic]) -> bool {
    let has_issues = !diagnostics.is_empty();

    match format {
        "json" => println!("{}", json(diagnostics)),
        "gitlab" => println!("{}", gitlab(diagnostics)),
        "github" => print!("{}", github(diagnostics)),
        "sarif" => println!("{}", sarif(diagnostics)),
        _ => {
            print!("{}", text(diagnostics));
            if has_issues {
                eprintln!(
                    "\n{} Found {} issue{}.",
                    "✗".red(),
                    diagnostics.len(),
                    if diagnostics.len() == 1 { "" } else { "s" }
                );
            } else {
                eprintln!("{} No issues found.", "✓".green());
            }
        }
    }

    has_issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use thorn_api::{Diagnostic, Level};

    fn make_diag(code: &str, msg: &str, file: &str, line: u32, col: u32, level: Level) -> Diagnostic {
        let mut d = Diagnostic::new(code, msg, file).with_level(level);
        d.line = Some(line);
        d.col = Some(col);
        d
    }

    fn sample_diagnostics() -> Vec<Diagnostic> {
        vec![
            make_diag("DJ001", "Avoid nullable CharField", "models.py", 10, 5, Level::Fix),
            make_diag("DJ012", "Use select_related", "views.py", 42, 12, Level::Improve),
            make_diag("DJ050", "Unused import", "utils.py", 1, 1, Level::All),
        ]
    }

    // ── GitLab ──────────────────────────────────────────────────────

    #[test]
    fn gitlab_empty() {
        let out = gitlab(&[]);
        assert_eq!(out, "[]");
    }

    #[test]
    fn gitlab_structure() {
        let diags = sample_diagnostics();
        let out = gitlab(&diags);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 3);

        // First issue: Fix → major
        let first = &arr[0];
        assert_eq!(first["check_name"], "DJ001");
        assert_eq!(first["severity"], "major");
        assert_eq!(first["location"]["path"], "models.py");
        assert_eq!(first["location"]["lines"]["begin"], 10);
        assert!(first["description"].as_str().unwrap().contains("Avoid nullable CharField"));
        // fingerprint is a 32-char hex string
        let fp = first["fingerprint"].as_str().unwrap();
        assert_eq!(fp.len(), 32);
        assert!(fp.chars().all(|c| c.is_ascii_hexdigit()));

        // Second: Improve → minor
        assert_eq!(arr[1]["severity"], "minor");

        // Third: All → info
        assert_eq!(arr[2]["severity"], "info");
    }

    #[test]
    fn gitlab_fingerprint_stable() {
        let diags = sample_diagnostics();
        let a = gitlab(&diags);
        let b = gitlab(&diags);
        assert_eq!(a, b, "fingerprints should be deterministic");
    }

    #[test]
    fn gitlab_no_line_defaults_to_1() {
        let d = Diagnostic::new("X001", "msg", "file.py").with_level(Level::Fix);
        let out = gitlab(&[d]);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed[0]["location"]["lines"]["begin"], 1);
    }

    // ── GitHub ──────────────────────────────────────────────────────

    #[test]
    fn github_empty() {
        assert_eq!(github(&[]), "");
    }

    #[test]
    fn github_format() {
        let diags = sample_diagnostics();
        let out = github(&diags);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 3);

        assert_eq!(
            lines[0],
            "::error file=models.py,line=10,col=5,title=DJ001::Avoid nullable CharField"
        );
        assert_eq!(
            lines[1],
            "::warning file=views.py,line=42,col=12,title=DJ012::Use select_related"
        );
        assert_eq!(
            lines[2],
            "::notice file=utils.py,line=1,col=1,title=DJ050::Unused import"
        );
    }

    #[test]
    fn github_no_line_no_col() {
        let d = Diagnostic::new("X001", "oops", "f.py");
        let out = github(&[d]);
        assert_eq!(out, "::warning file=f.py,title=X001::oops\n");
    }

    // ── SARIF ───────────────────────────────────────────────────────

    #[test]
    fn sarif_empty() {
        let out = sarif(&[]);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["version"], "2.1.0");
        assert!(parsed["$schema"].as_str().unwrap().contains("sarif"));
        let results = parsed["runs"][0]["results"].as_array().unwrap();
        assert!(results.is_empty());
        assert_eq!(parsed["runs"][0]["tool"]["driver"]["name"], "thorn");
    }

    #[test]
    fn sarif_structure() {
        let diags = sample_diagnostics();
        let out = sarif(&diags);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();

        let results = parsed["runs"][0]["results"].as_array().unwrap();
        assert_eq!(results.len(), 3);

        let r0 = &results[0];
        assert_eq!(r0["ruleId"], "DJ001");
        assert_eq!(r0["level"], "error");
        assert_eq!(r0["message"]["text"], "Avoid nullable CharField");
        let loc = &r0["locations"][0]["physicalLocation"];
        assert_eq!(loc["artifactLocation"]["uri"], "models.py");
        assert_eq!(loc["region"]["startLine"], 10);
        assert_eq!(loc["region"]["startColumn"], 5);

        // Improve → warning
        assert_eq!(results[1]["level"], "warning");
        // All → note
        assert_eq!(results[2]["level"], "note");
    }

    #[test]
    fn sarif_no_line_omits_region() {
        let d = Diagnostic::new("X001", "msg", "file.py");
        let out = sarif(&[d]);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        let loc = &parsed["runs"][0]["results"][0]["locations"][0]["physicalLocation"];
        assert!(loc.get("region").is_none(), "region should be omitted when no line");
    }

    #[test]
    fn sarif_valid_json() {
        let diags = sample_diagnostics();
        let out = sarif(&diags);
        // Must parse as valid JSON
        serde_json::from_str::<serde_json::Value>(&out).expect("SARIF output must be valid JSON");
    }
}
