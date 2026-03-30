use thorn_api::Diagnostic;

/// Filter out diagnostics suppressed by inline comments.
///
/// Supported syntax:
///   - `# noqa` — suppress all on this line
///   - `# noqa: DJ001` or `# noqa: DJ001,DJ002` — suppress specific codes
///   - `# thorn: ignore` — suppress all on this line
///   - `# thorn: ignore[DJ001]` or `# thorn: ignore[DJ001,DJ002]` — suppress specific codes
pub fn filter_suppressed(diagnostics: &mut Vec<Diagnostic>, source: &str) {
    let source_lines: Vec<&str> = source.lines().collect();
    diagnostics.retain(|d| {
        let Some(line_num) = d.line else { return true };
        let Some(line) = source_lines.get((line_num - 1) as usize) else {
            return true;
        };

        let Some(comment_start) = line.find('#') else {
            return true;
        };
        let comment = &line[comment_start..];

        // # noqa: DJ001 or # noqa: DJ001,DJ002
        if let Some(noqa_pos) = comment.find("noqa:") {
            let codes = &comment[noqa_pos + 5..].trim();
            let codes: Vec<&str> = codes.split(',').map(|s| s.trim()).collect();
            if codes.iter().any(|c| *c == d.code) {
                return false;
            }
        }
        // # noqa (suppress all on this line)
        if comment.contains("noqa") && !comment.contains("noqa:") {
            return false;
        }

        // # thorn: ignore[DJ001] or # thorn: ignore
        if let Some(thorn_pos) = comment.find("thorn: ignore") {
            let rest = &comment[thorn_pos + 13..];
            if rest.starts_with('[') {
                if let Some(end) = rest.find(']') {
                    let codes = &rest[1..end];
                    let codes: Vec<&str> = codes.split(',').map(|s| s.trim()).collect();
                    if codes.iter().any(|c| *c == d.code) {
                        return false;
                    }
                }
            } else {
                return false;
            }
        }

        true
    });
}
