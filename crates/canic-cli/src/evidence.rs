use crate::{
    cli::clap::{parse_matches, parse_subcommand, passthrough_subcommand, path_option, value_arg},
    cli::help::print_help_or_version,
    output, version_text,
};
use canic_host::evidence_envelope::EvidenceEnvelopeV1;
use clap::Command as ClapCommand;
use serde::Serialize;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const COMPARED_FIELDS: &[&str] = &[
    "envelope_schema",
    "command",
    "target",
    "source_config",
    "inputs",
    "payload_schema",
    "payload_sha256",
    "summary",
    "exit_class",
];
const IGNORED_FIELDS: &[&str] = &["canic_version", "generated_at", "payload"];

///
/// EvidenceCommandError
///
#[derive(Debug, ThisError)]
pub enum EvidenceCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("envelopes differ:\n{0}")]
    EnvelopesDiffer(String),
}

///
/// EvidenceCompareOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct EvidenceCompareOptions {
    left: PathBuf,
    right: PathBuf,
    format: EvidenceCompareFormat,
}

impl EvidenceCompareOptions {
    fn parse<I>(args: I) -> Result<Self, EvidenceCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(evidence_compare_command(), args)
            .map_err(|_| EvidenceCommandError::Usage(compare_usage()))?;
        let format = match matches
            .get_one::<String>("format")
            .map_or("text", String::as_str)
        {
            "text" => EvidenceCompareFormat::Text,
            "json" => EvidenceCompareFormat::Json,
            _ => return Err(EvidenceCommandError::Usage(compare_usage())),
        };

        Ok(Self {
            left: path_option(&matches, "left").expect("clap requires left"),
            right: path_option(&matches, "right").expect("clap requires right"),
            format,
        })
    }
}

///
/// EvidenceCompareFormat
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EvidenceCompareFormat {
    Text,
    Json,
}

///
/// EvidenceCompareStatus
///
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum EvidenceCompareStatus {
    Matched,
    Different,
}

///
/// EvidenceCompareDifference
///
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct EvidenceCompareDifference {
    field: String,
    left: serde_json::Value,
    right: serde_json::Value,
}

///
/// EvidenceCompareReport
///
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct EvidenceCompareReport {
    schema_version: u32,
    status: EvidenceCompareStatus,
    left: String,
    right: String,
    compared_fields: Vec<String>,
    ignored_fields: Vec<String>,
    differences: Vec<EvidenceCompareDifference>,
}

/// Run an evidence subcommand.
pub fn run<I>(args: I) -> Result<(), EvidenceCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let Some((command, args)) = parse_subcommand(evidence_command(), args)
        .map_err(|_| EvidenceCommandError::Usage(usage()))?
    else {
        return Err(EvidenceCommandError::Usage(usage()));
    };

    match command.as_str() {
        "compare" => {
            if print_help_or_version(&args, compare_usage, version_text()) {
                return Ok(());
            }
            let options = EvidenceCompareOptions::parse(args)?;
            let report = compare_envelope_files(&options)?;
            write_compare_report(&options, &report)?;
            if report.status == EvidenceCompareStatus::Different {
                return Err(EvidenceCommandError::EnvelopesDiffer(
                    render_compare_differences(&report),
                ));
            }
            Ok(())
        }
        _ => unreachable!("evidence dispatch command only defines known commands"),
    }
}

fn compare_envelope_files(
    options: &EvidenceCompareOptions,
) -> Result<EvidenceCompareReport, EvidenceCommandError> {
    let left = output::read_json_file::<EvidenceEnvelopeV1, EvidenceCommandError>(&options.left)?;
    let right = output::read_json_file::<EvidenceEnvelopeV1, EvidenceCommandError>(&options.right)?;
    Ok(compare_envelopes(
        &left,
        &right,
        &options.left,
        &options.right,
    ))
}

fn compare_envelopes(
    left: &EvidenceEnvelopeV1,
    right: &EvidenceEnvelopeV1,
    left_path: &Path,
    right_path: &Path,
) -> EvidenceCompareReport {
    let left_value = serde_json::to_value(left).expect("envelope should serialize");
    let right_value = serde_json::to_value(right).expect("envelope should serialize");
    let mut differences = Vec::new();
    for field in COMPARED_FIELDS {
        let left_field = left_value.get(*field).cloned().unwrap_or_default();
        let right_field = right_value.get(*field).cloned().unwrap_or_default();
        if left_field != right_field {
            differences.push(EvidenceCompareDifference {
                field: (*field).to_string(),
                left: left_field,
                right: right_field,
            });
        }
    }

    EvidenceCompareReport {
        schema_version: 1,
        status: if differences.is_empty() {
            EvidenceCompareStatus::Matched
        } else {
            EvidenceCompareStatus::Different
        },
        left: left_path.display().to_string(),
        right: right_path.display().to_string(),
        compared_fields: COMPARED_FIELDS
            .iter()
            .map(|field| (*field).to_string())
            .collect(),
        ignored_fields: IGNORED_FIELDS
            .iter()
            .map(|field| (*field).to_string())
            .collect(),
        differences,
    }
}

fn write_compare_report(
    options: &EvidenceCompareOptions,
    report: &EvidenceCompareReport,
) -> Result<(), EvidenceCommandError> {
    match options.format {
        EvidenceCompareFormat::Text => {
            output::write_text::<EvidenceCommandError>(None, &render_compare_report(report))
        }
        EvidenceCompareFormat::Json => output::write_pretty_json(None, report),
    }
}

fn render_compare_report(report: &EvidenceCompareReport) -> String {
    let mut lines = vec![
        "Evidence envelope compare:".to_string(),
        format!("  left: {}", report.left),
        format!("  right: {}", report.right),
        format!(
            "  status: {}",
            match report.status {
                EvidenceCompareStatus::Matched => "matched",
                EvidenceCompareStatus::Different => "different",
            }
        ),
        format!("  compared_fields: {}", report.compared_fields.join(", ")),
        format!("  ignored_fields: {}", report.ignored_fields.join(", ")),
    ];

    if report.differences.is_empty() {
        lines.push("Differences: none".to_string());
    } else {
        lines.push("Differences:".to_string());
        for difference in &report.differences {
            lines.push(format!("  - {}", difference.field));
        }
    }

    lines.join("\n")
}

fn render_compare_differences(report: &EvidenceCompareReport) -> String {
    report
        .differences
        .iter()
        .map(|difference| format!("- {}", difference.field))
        .collect::<Vec<_>>()
        .join("\n")
}

fn usage() -> String {
    let mut command = evidence_command();
    command.render_help().to_string()
}

fn compare_usage() -> String {
    let mut command = evidence_compare_command();
    command.render_help().to_string()
}

fn evidence_command() -> ClapCommand {
    ClapCommand::new("evidence")
        .bin_name("canic evidence")
        .about("Compare stable Canic evidence envelopes")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("compare")
                .about("Compare two EvidenceEnvelopeV1 JSON files")
                .disable_help_flag(true),
        ))
}

fn evidence_compare_command() -> ClapCommand {
    ClapCommand::new("compare")
        .bin_name("canic evidence compare")
        .about("Compare two EvidenceEnvelopeV1 JSON files")
        .disable_help_flag(true)
        .arg(
            value_arg("left")
                .long("left")
                .value_name("file")
                .required(true),
        )
        .arg(
            value_arg("right")
                .long("right")
                .value_name("file")
                .required(true),
        )
        .arg(
            value_arg("format")
                .long("format")
                .value_name("text|json")
                .default_value("text"),
        )
        .after_help(
            "Compares stable envelope fields and ignores generated_at, canic_version, and the nested payload body. The payload_sha256 field is compared.",
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_host::evidence_envelope::{
        CommandProvenanceV1, EvidenceMessageSeverityV1, EvidenceMessageV1, EvidenceSummaryV1,
        EvidenceTargetKindV1, EvidenceTargetV1, ExitClassV1, adoption_report_schema,
        evidence_envelope_schema,
    };
    use serde_json::json;
    use std::{fs, time::UNIX_EPOCH};

    #[test]
    fn parses_compare_options() {
        let options = EvidenceCompareOptions::parse([
            OsString::from("--left"),
            OsString::from("left.json"),
            OsString::from("--right"),
            OsString::from("right.json"),
            OsString::from("--format"),
            OsString::from("json"),
        ])
        .expect("parse options");

        assert_eq!(options.left, PathBuf::from("left.json"));
        assert_eq!(options.right, PathBuf::from("right.json"));
        assert_eq!(options.format, EvidenceCompareFormat::Json);
    }

    #[test]
    fn compare_ignores_timestamp_version_and_payload_body() {
        let left_path = PathBuf::from("left.json");
        let right_path = PathBuf::from("right.json");
        let mut left = sample_envelope();
        let mut right = sample_envelope();
        right.canic_version = "different".to_string();
        right.generated_at = "2030-01-01T00:00:00Z".to_string();
        right.payload = json!({ "changed": true });

        let report = compare_envelopes(&left, &right, &left_path, &right_path);

        assert_eq!(report.status, EvidenceCompareStatus::Matched);
        assert!(report.differences.is_empty());
        left.payload_sha256 = Some("left-hash".to_string());
        right.payload_sha256 = Some("right-hash".to_string());

        let report = compare_envelopes(&left, &right, &left_path, &right_path);

        assert_eq!(report.status, EvidenceCompareStatus::Different);
        assert_eq!(report.differences[0].field, "payload_sha256");
    }

    #[test]
    fn compare_detects_exit_class_and_summary_differences() {
        let left_path = PathBuf::from("left.json");
        let right_path = PathBuf::from("right.json");
        let left = sample_envelope();
        let mut right = sample_envelope();
        right.exit_class = ExitClassV1::EvidenceConflict;
        right
            .summary
            .evidence_conflicts
            .push(EvidenceMessageV1::new(
                "test.conflict",
                "conflict",
                EvidenceMessageSeverityV1::Error,
            ));

        let report = compare_envelopes(&left, &right, &left_path, &right_path);

        assert_eq!(report.status, EvidenceCompareStatus::Different);
        assert!(
            report
                .differences
                .iter()
                .any(|difference| difference.field == "exit_class")
        );
        assert!(
            report
                .differences
                .iter()
                .any(|difference| difference.field == "summary")
        );
    }

    #[test]
    fn compare_files_returns_error_for_differences_after_writing_report() {
        let left_path = temp_json_path("canic-evidence-left");
        let right_path = temp_json_path("canic-evidence-right");
        let left = sample_envelope();
        let mut right = sample_envelope();
        right.exit_class = ExitClassV1::BlockedByPolicy;
        fs::write(&left_path, serde_json::to_vec(&left).expect("encode left")).expect("write left");
        fs::write(
            &right_path,
            serde_json::to_vec(&right).expect("encode right"),
        )
        .expect("write right");
        let options = EvidenceCompareOptions {
            left: left_path.clone(),
            right: right_path.clone(),
            format: EvidenceCompareFormat::Text,
        };

        let report = compare_envelope_files(&options).expect("compare files");

        fs::remove_file(left_path).expect("clean left");
        fs::remove_file(right_path).expect("clean right");
        assert_eq!(report.status, EvidenceCompareStatus::Different);
        assert_eq!(report.differences[0].field, "exit_class");
    }

    fn sample_envelope() -> EvidenceEnvelopeV1 {
        EvidenceEnvelopeV1 {
            envelope_schema: evidence_envelope_schema(),
            canic_version: env!("CARGO_PKG_VERSION").to_string(),
            command: CommandProvenanceV1 {
                name: "canic fleet adoption report".to_string(),
                argv_normalized: vec![
                    "canic".to_string(),
                    "fleet".to_string(),
                    "adoption".to_string(),
                    "report".to_string(),
                    "demo".to_string(),
                ],
                argv_redactions: Vec::new(),
                format: "envelope-json".to_string(),
            },
            target: EvidenceTargetV1 {
                kind: EvidenceTargetKindV1::FleetAdoption,
                deployment: None,
                fleet: Some("demo".to_string()),
                role: None,
                profile: Some("minimal".to_string()),
                network: None,
            },
            generated_at: "2026-05-31T00:00:00Z".to_string(),
            source_config: None,
            inputs: Vec::new(),
            payload_schema: adoption_report_schema(),
            payload_sha256: Some(sample_sha256("payload")),
            payload: json!({ "report_id": "report-1" }),
            summary: EvidenceSummaryV1 {
                warnings: Vec::new(),
                blocked_actions: Vec::new(),
                missing_or_stale_evidence: Vec::new(),
                evidence_conflicts: Vec::new(),
            },
            exit_class: ExitClassV1::Success,
        }
    }

    fn sample_sha256(label: &str) -> String {
        let mut hash = String::new();
        while hash.len() < 64 {
            hash.push_str(label);
        }
        hash.truncate(64);
        hash
    }

    fn temp_json_path(name: &str) -> PathBuf {
        let suffix = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{name}-{suffix}.json"))
    }
}
