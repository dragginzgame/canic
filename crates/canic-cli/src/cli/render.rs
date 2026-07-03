//! Module: cli::render
//! Responsibility: small shared rendering helpers for CLI text output.
//! Does not own: command-specific report construction or domain formatting.
//! Boundary: reusable text fragments shared across command renderers.

/// Append the standard no-write dry-run footer to command preview output.
pub fn append_dry_run_footer(lines: &mut Vec<String>) {
    lines.push("  dry_run: true".to_string());
    lines.push("  files_changed: 0".to_string());
}
