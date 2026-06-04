use super::{
    DeployCommandError, DeployTruthOptions, deploy_truth_leaf_command, load_deployment_check,
    output_format::{CheckOutputFormat, parse_check_output_format},
    print_json, value_arg,
};
use crate::{
    cli::{
        clap::{parse_matches, path_option, render_usage, typed_option},
        defaults::local_network,
        help::print_help_or_version,
    },
    evidence_support, version_text,
};
use canic_host::{
    build_provenance::build_provenance_schema,
    deployment_truth::{DeploymentCheckV1, SafetyReportV1, SafetyStatusV1},
    evidence_envelope::{
        CommandProvenanceV1, EvidenceEnvelopeV1, EvidenceMessageSeverityV1, EvidenceMessageV1,
        EvidenceSummaryV1, EvidenceTargetKindV1, EvidenceTargetV1, ExitClassV1, InputFingerprintV1,
        InputPathDisplayV1, PayloadSchemaRefV1, deployment_check_schema, evidence_envelope_schema,
        evidence_summary_exit_class, file_input_fingerprint, json_payload_sha256,
    },
};
use clap::Command as ClapCommand;
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

const DEPLOY_CHECK_HELP_AFTER: &str = "\
Examples:
  canic deploy check demo
  canic --network local deploy check --profile fast demo
  canic deploy check demo --format envelope-json
  canic deploy check demo --format envelope-json --build-provenance build-provenance.json

Prints the local DeploymentCheckV1 JSON without installing or mutating state.
Use --format envelope-json for the stable CI/GitOps evidence envelope.
--build-provenance is fingerprinted only in envelope output.";

const CHECK_COMMAND_NAME: &str = "check";
const FORMAT_ARG: &str = "format";
const BUILD_PROVENANCE_ARG: &str = "build-provenance";
const BUILD_PROVENANCE_FLAG: &str = "--build-provenance";

///
/// DeployCheckOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DeployCheckOptions {
    pub(super) truth: DeployTruthOptions,
    pub(super) format: CheckOutputFormat,
    pub(super) build_provenance: Option<PathBuf>,
}

pub(super) fn run<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let options = DeployCheckOptions::parse(args)?;
    let check = load_deployment_check(options.truth.clone())?;
    write_deployment_check(&options, &check)?;
    enforce_deployment_check_status(&check.report)
}

fn write_deployment_check(
    options: &DeployCheckOptions,
    check: &DeploymentCheckV1,
) -> Result<(), DeployCommandError> {
    match options.format {
        CheckOutputFormat::Json => print_json(check),
        CheckOutputFormat::EnvelopeJson => {
            let envelope = build_deployment_check_envelope(options, check)?;
            print_json(&envelope)
        }
    }
}

pub(super) fn build_deployment_check_envelope(
    options: &DeployCheckOptions,
    check: &DeploymentCheckV1,
) -> Result<EvidenceEnvelopeV1, DeployCommandError> {
    let payload = serde_json::to_value(check).map_err(Box::<dyn std::error::Error>::from)?;
    let payload_sha256 =
        Some(json_payload_sha256(check).map_err(Box::<dyn std::error::Error>::from)?);
    let config_root = deployment_check_config_root(check);
    let source_config = deployment_check_source_config_fingerprint(check)?;
    let summary = deployment_check_evidence_summary(check);
    let exit_class = combine_deployment_check_exit_class(check.report.status, &summary);

    Ok(EvidenceEnvelopeV1 {
        envelope_schema: evidence_envelope_schema(),
        canic_version: env!("CARGO_PKG_VERSION").to_string(),
        command: deployment_check_command_provenance(options, &config_root),
        target: EvidenceTargetV1 {
            kind: EvidenceTargetKindV1::Deployment,
            deployment: Some(check.plan.deployment_identity.deployment_name.clone()),
            fleet: Some(check.plan.fleet_template.clone()),
            role: None,
            profile: options
                .truth
                .profile
                .map(|profile| profile.target_dir_name().to_string()),
            network: Some(check.plan.deployment_identity.network.clone()),
        },
        generated_at: check.inventory.observed_at.clone(),
        source_config,
        inputs: deployment_check_input_fingerprints(options, &config_root)?,
        payload_schema: deployment_check_schema(),
        payload_sha256,
        payload,
        summary,
        exit_class,
    })
}

fn deployment_check_command_provenance(
    options: &DeployCheckOptions,
    config_root: &Path,
) -> CommandProvenanceV1 {
    let mut argv_normalized = vec![
        "canic".to_string(),
        "deploy".to_string(),
        "check".to_string(),
        options.truth.deployment.clone(),
        "--format".to_string(),
        "envelope-json".to_string(),
    ];
    if let Some(profile) = options.truth.profile {
        argv_normalized.push("--profile".to_string());
        argv_normalized.push(profile.target_dir_name().to_string());
    }
    if options.truth.network != local_network() {
        argv_normalized.push("--network".to_string());
        argv_normalized.push(options.truth.network.clone());
    }
    let mut argv_redactions = Vec::new();
    evidence_support::push_optional_path_arg(
        &mut argv_normalized,
        &mut argv_redactions,
        BUILD_PROVENANCE_FLAG,
        options.build_provenance.as_ref(),
        config_root,
    );

    CommandProvenanceV1 {
        name: "canic deploy check".to_string(),
        argv_normalized,
        argv_redactions,
        format: "envelope-json".to_string(),
    }
}

fn deployment_check_config_root(check: &DeploymentCheckV1) -> PathBuf {
    check
        .inventory
        .local_config
        .config_path
        .as_deref()
        .and_then(|path| Path::new(path).parent())
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

fn deployment_check_input_fingerprints(
    options: &DeployCheckOptions,
    config_root: &Path,
) -> Result<Vec<InputFingerprintV1>, DeployCommandError> {
    let mut inputs = Vec::new();
    if let Some(path) = &options.build_provenance {
        inputs.push(
            file_input_fingerprint(
                "build_provenance",
                path,
                config_root,
                Some(build_provenance_schema()),
                None,
            )
            .map_err(Box::<dyn std::error::Error>::from)
            .map_err(DeployCommandError::from)?,
        );
    }
    Ok(inputs)
}

fn deployment_check_source_config_fingerprint(
    check: &DeploymentCheckV1,
) -> Result<Option<InputFingerprintV1>, DeployCommandError> {
    let Some(config_path) = &check.inventory.local_config.config_path else {
        return Ok(None);
    };
    let path = Path::new(config_path);
    let config_root = path.parent().unwrap_or_else(|| Path::new("."));
    let mut fingerprint = match fs::metadata(path) {
        Ok(_) => file_input_fingerprint(
            "canic_config",
            path,
            config_root,
            Some(PayloadSchemaRefV1::internal("canic.config.toml", "1")),
            None,
        )
        .map_err(Box::<dyn std::error::Error>::from)
        .map_err(DeployCommandError::from)?,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => InputFingerprintV1 {
            kind: "canic_config".to_string(),
            path: None,
            path_display: InputPathDisplayV1::Omitted,
            sha256: None,
            size_bytes: None,
            modified_unix_secs: None,
            schema: Some(PayloadSchemaRefV1::internal("canic.config.toml", "1")),
            note: Some("source config path was recorded but file is not available".to_string()),
        },
        Err(err) => {
            return Err(DeployCommandError::from(
                Box::<dyn std::error::Error>::from(err),
            ));
        }
    };

    if let Some(raw_config_sha256) = &check.inventory.local_config.raw_config_sha256 {
        fingerprint.sha256 = Some(raw_config_sha256.clone());
    } else if fingerprint.note.is_none() {
        fingerprint.note = Some("raw config hash was not observed by deployment check".to_string());
    }

    Ok(Some(fingerprint))
}

fn deployment_check_evidence_summary(check: &DeploymentCheckV1) -> EvidenceSummaryV1 {
    EvidenceSummaryV1 {
        warnings: check
            .report
            .warnings
            .iter()
            .map(|finding| {
                EvidenceMessageV1::new(
                    &format!("deploy.warning.{}", finding.code),
                    finding.message.clone(),
                    EvidenceMessageSeverityV1::Warning,
                )
            })
            .collect(),
        blocked_actions: check
            .report
            .hard_failures
            .iter()
            .map(|finding| {
                EvidenceMessageV1::new(
                    &format!("deploy.blocked.{}", finding.code),
                    finding.message.clone(),
                    EvidenceMessageSeverityV1::Error,
                )
            })
            .collect(),
        missing_or_stale_evidence: deployment_check_missing_or_stale_evidence(check),
        evidence_conflicts: deployment_check_evidence_conflicts(check),
    }
}

fn deployment_check_missing_or_stale_evidence(check: &DeploymentCheckV1) -> Vec<EvidenceMessageV1> {
    check
        .inventory
        .unresolved_observations
        .iter()
        .map(|gap| {
            EvidenceMessageV1::new(
                "deploy.missing_or_stale.observation",
                gap.description.clone(),
                EvidenceMessageSeverityV1::Warning,
            )
        })
        .chain(check.plan.unresolved_assumptions.iter().map(|assumption| {
            EvidenceMessageV1::new(
                "deploy.missing_or_stale.assumption",
                assumption.description.clone(),
                EvidenceMessageSeverityV1::Warning,
            )
        }))
        .collect()
}

fn deployment_check_evidence_conflicts(check: &DeploymentCheckV1) -> Vec<EvidenceMessageV1> {
    check
        .report
        .hard_failures
        .iter()
        .chain(check.report.warnings.iter())
        .filter(|finding| finding.code.contains("conflict"))
        .map(|finding| {
            EvidenceMessageV1::new(
                &format!("deploy.evidence_conflict.{}", finding.code),
                finding.message.clone(),
                EvidenceMessageSeverityV1::Error,
            )
        })
        .collect()
}

const fn combine_deployment_check_exit_class(
    status: SafetyStatusV1,
    summary: &EvidenceSummaryV1,
) -> ExitClassV1 {
    let status_class = deployment_check_status_exit_class(status);
    let summary_class =
        evidence_summary_exit_class(summary, matches!(status, SafetyStatusV1::NotEvaluated));

    if summary_class.dominates(status_class) {
        summary_class
    } else {
        status_class
    }
}

const fn deployment_check_status_exit_class(status: SafetyStatusV1) -> ExitClassV1 {
    match status {
        SafetyStatusV1::Safe => ExitClassV1::Success,
        SafetyStatusV1::Warning => ExitClassV1::SuccessWithWarnings,
        SafetyStatusV1::Blocked => ExitClassV1::BlockedByPolicy,
        SafetyStatusV1::NotEvaluated => ExitClassV1::MissingRequiredEvidence,
    }
}

pub(super) fn enforce_deployment_check_status(
    report: &SafetyReportV1,
) -> Result<(), DeployCommandError> {
    if report.status == SafetyStatusV1::Blocked {
        return Err(DeployCommandError::Blocked(report.summary.clone()));
    }
    Ok(())
}

impl DeployCheckOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| DeployCommandError::Usage(usage()))?;
        let format = typed_option(&matches, FORMAT_ARG).unwrap_or(CheckOutputFormat::Json);
        let build_provenance = path_option(&matches, BUILD_PROVENANCE_ARG);
        if build_provenance.is_some() && format != CheckOutputFormat::EnvelopeJson {
            return Err(DeployCommandError::Usage(format!(
                "{BUILD_PROVENANCE_FLAG} requires --format envelope-json\n\n{}",
                usage()
            )));
        }

        Ok(Self {
            truth: DeployTruthOptions::from_matches(&matches),
            format,
            build_provenance,
        })
    }
}

pub(super) fn command() -> ClapCommand {
    deploy_truth_leaf_command(
        CHECK_COMMAND_NAME,
        "Print the local deployment truth check JSON",
    )
    .arg(check_format_arg())
    .arg(build_provenance_input_arg())
    .after_help(DEPLOY_CHECK_HELP_AFTER)
}

fn check_format_arg() -> clap::Arg {
    value_arg(FORMAT_ARG)
        .long(FORMAT_ARG)
        .value_name("json|envelope-json")
        .num_args(1)
        .value_parser(clap::builder::ValueParser::new(parse_check_output_format))
        .help("Output format; defaults to json")
}

fn build_provenance_input_arg() -> clap::Arg {
    value_arg(BUILD_PROVENANCE_ARG)
        .long(BUILD_PROVENANCE_ARG)
        .value_name("path")
        .num_args(1)
        .help("Fingerprint a BuildProvenanceV1 evidence envelope; requires --format envelope-json")
}

pub(super) fn usage() -> String {
    render_usage(command)
}
