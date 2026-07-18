//! Module: canic_cli::medic::render
//!
//! Responsibility: render medic reports as stable JSON, operator text, or concise CI text.
//! Does not own: report construction, diagnostic collection, or command parsing.
//! Boundary: consumes the private medic report model after all checks are complete.

use crate::medic::{
    command::MedicCommandError,
    report::{MedicReport, MedicStatus, ordered_checks},
};

pub(super) const MEDIC_REPORT_WIDTH: usize = 100;

pub(super) fn render_medic_json(report: &MedicReport) -> Result<String, MedicCommandError> {
    serde_json::to_string_pretty(report).map_err(MedicCommandError::from)
}

pub(super) fn render_medic_text(report: &MedicReport) -> String {
    let mut lines = vec![
        report.command.clone(),
        format!("status: {}", report.status.label()),
        format!(
            "environment: {}",
            report.environment.as_deref().unwrap_or("not selected")
        ),
        format!(
            "deployment: {}",
            report.deployment.as_deref().unwrap_or("not selected")
        ),
    ];

    for check in ordered_checks(&report.checks) {
        lines.push(String::new());
        lines.push(format!(
            "{} [{}] {}",
            check.category.label(),
            check.status.label(),
            check.code
        ));
        push_medic_field(&mut lines, "subject", &check.subject);
        push_medic_field(&mut lines, "detail", &check.detail);
        push_medic_field(&mut lines, "next", &check.next);
        push_medic_field(&mut lines, "source", check.source.label());
    }
    lines.join("\n")
}

pub(super) fn render_medic_ci_text(report: &MedicReport) -> String {
    let mut lines = vec![
        report.command.clone(),
        format!("status: {}", report.status.label()),
    ];
    let failures = ordered_checks(&report.checks)
        .into_iter()
        .filter(|check| check.status == MedicStatus::Fail)
        .collect::<Vec<_>>();

    if failures.is_empty() {
        lines.push("failures: none".to_string());
        return lines.join("\n");
    }

    lines.push(format!("failures: {}", failures.len()));
    for check in failures {
        lines.push(format!(
            "{} {} {} {}",
            check.status.label(),
            check.category.label(),
            check.code,
            check.subject
        ));
        push_medic_field(&mut lines, "detail", &check.detail);
        push_medic_field(&mut lines, "next", &check.next);
        push_medic_field(&mut lines, "source", check.source.label());
    }

    lines.join("\n")
}

fn push_medic_field(lines: &mut Vec<String>, label: &str, value: &str) {
    let prefix = format!("  {label}: ");
    let continuation_prefix = " ".repeat(prefix.chars().count());
    let width = MEDIC_REPORT_WIDTH.saturating_sub(prefix.chars().count());

    for (index, line) in wrap_medic_text(value, width).into_iter().enumerate() {
        if index == 0 {
            lines.push(format!("{prefix}{line}"));
        } else if line.is_empty() {
            lines.push(String::new());
        } else {
            lines.push(format!("{continuation_prefix}{line}"));
        }
    }
}

fn wrap_medic_text(value: &str, width: usize) -> Vec<String> {
    let wrapped = value
        .lines()
        .flat_map(|line| wrap_medic_line(line, width))
        .collect::<Vec<_>>();
    if wrapped.is_empty() {
        vec![String::new()]
    } else {
        wrapped
    }
}

fn wrap_medic_line(line: &str, width: usize) -> Vec<String> {
    if line.trim().is_empty() {
        return vec![String::new()];
    }

    let width = width.max(1);
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in line.split_whitespace() {
        if word.chars().count() > width {
            if !current.is_empty() {
                lines.push(current);
                current = String::new();
            }
            lines.extend(split_medic_word(word, width));
            continue;
        }

        let candidate_width =
            current.chars().count() + usize::from(!current.is_empty()) + word.chars().count();
        if current.is_empty() {
            current.push_str(word);
        } else if candidate_width <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn split_medic_word(word: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut chunks = Vec::new();
    let mut current = String::new();
    for ch in word.chars() {
        if current.chars().count() == width {
            chunks.push(current);
            current = String::new();
        }
        current.push(ch);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}
