use super::*;

mod authority;
mod comparison;
mod execution_preflight;
mod lifecycle;
mod promotion;
mod root_verification;

pub use authority::*;
pub use comparison::*;
pub use execution_preflight::*;
pub use lifecycle::*;
pub use promotion::*;
pub use root_verification::*;

fn append_hard_failure_items(lines: &mut Vec<String>, label: &str, failures: &[SafetyFindingV1]) {
    if failures.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{label}:"));
    for failure in failures {
        let subject = failure.subject.as_deref().unwrap_or("unknown subject");
        lines.push(format!(
            "  - [{}] {}: {}",
            failure.code, subject, failure.message
        ));
    }
}

fn append_warning_items(lines: &mut Vec<String>, label: &str, warnings: &[SafetyFindingV1]) {
    if warnings.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{label}:"));
    for warning in warnings {
        let subject = warning.subject.as_deref().unwrap_or("unknown subject");
        lines.push(format!(
            "  - [{}] {}: {}",
            warning.code, subject, warning.message
        ));
    }
}

fn append_string_items(lines: &mut Vec<String>, label: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{label}:"));
    for value in values {
        lines.push(format!("  - {value}"));
    }
}

const fn safety_status_label(status: SafetyStatusV1) -> &'static str {
    match status {
        SafetyStatusV1::NotEvaluated => "not_evaluated",
        SafetyStatusV1::Safe => "safe",
        SafetyStatusV1::Warning => "warning",
        SafetyStatusV1::Blocked => "blocked",
    }
}

fn optional_text(value: Option<&str>) -> &str {
    value.unwrap_or("none")
}

const fn optional_bool_label(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "unknown",
    }
}
