use super::super::value_arg;
use crate::cli::clap::{flag_arg, passthrough_subcommand, render_usage};
use clap::Command as ClapCommand;

#[derive(Clone, Copy)]
struct PromoteSubcommand {
    name: &'static str,
    about: &'static str,
}

#[derive(Clone, Copy)]
struct PromoteReportCommand {
    name: &'static str,
    about: &'static str,
    bin_name: &'static str,
    help_after: &'static str,
}

pub(super) const TEXT_ARG: &str = "text";

const TOP_COMMANDS: &[PromoteSubcommand] = &[
    PromoteSubcommand {
        name: "plan",
        about: "Build a passive artifact promotion plan",
    },
    PromoteSubcommand {
        name: "check",
        about: "Build a passive artifact promotion readiness check",
    },
    PromoteSubcommand {
        name: "diff",
        about: "Build a passive artifact promotion diff",
    },
    PromoteSubcommand {
        name: "inspect",
        about: "Inspect passive artifact promotion internals",
    },
];

const INSPECT_COMMANDS: &[PromoteSubcommand] = &[
    PromoteSubcommand {
        name: "readiness",
        about: "Build a passive promotion readiness report",
    },
    PromoteSubcommand {
        name: "artifact-identity",
        about: "Build a passive promotion artifact identity report",
    },
    PromoteSubcommand {
        name: "transform",
        about: "Build a passive promoted-plan transform",
    },
    PromoteSubcommand {
        name: "transform-evidence",
        about: "Build passive promoted-plan transform evidence",
    },
    PromoteSubcommand {
        name: "target-lineage",
        about: "Build passive target execution lineage",
    },
    PromoteSubcommand {
        name: "provenance",
        about: "Build a passive artifact promotion provenance report",
    },
    PromoteSubcommand {
        name: "wasm-store-identity",
        about: "Build a passive wasm-store identity report",
    },
    PromoteSubcommand {
        name: "catalog-verification",
        about: "Build a passive wasm-store catalog verification report",
    },
    PromoteSubcommand {
        name: "execution-receipt",
        about: "Build a passive artifact promotion execution receipt wrapper",
    },
    PromoteSubcommand {
        name: "policy",
        about: "Build a passive promotion policy check",
    },
    PromoteSubcommand {
        name: "materialization-identity",
        about: "Build a passive source/build materialization identity report",
    },
];

const DEPLOY_PROMOTE_HELP_AFTER: &str = "\
Examples:
  canic deploy promote plan --request promotion-plan.json
  canic deploy promote check --request promotion-check.json
  canic deploy promote diff --request promotion-diff.json
  canic deploy promote inspect readiness --request promotion-readiness.json
  canic deploy promote inspect artifact-identity --request promotion-artifacts.json
  canic deploy promote inspect provenance --request promotion-provenance.json
  canic deploy promote inspect readiness --request promotion-readiness.json --text

Promotion commands are passive report builders. They do not install,
stage artifacts, query wasm_store, or mutate deployment/controller state.";
const DEPLOY_PROMOTE_INSPECT_HELP_AFTER: &str = "\
Examples:
  canic deploy promote inspect readiness --request promotion-readiness.json
  canic deploy promote inspect artifact-identity --request promotion-artifacts.json
  canic deploy promote inspect transform --request promotion-transform.json
  canic deploy promote inspect transform-evidence --request transform-evidence.json
  canic deploy promote inspect target-lineage --request target-lineage.json
  canic deploy promote inspect provenance --request promotion-provenance.json
  canic deploy promote inspect wasm-store-identity --request wasm-store-identity.json
  canic deploy promote inspect catalog-verification --request catalog-verification.json
  canic deploy promote inspect materialization-identity --request materialization.json
  canic deploy promote inspect policy --request promotion-policy.json
  canic deploy promote inspect execution-receipt --request promotion-execution-receipt.json

Advanced promotion inspection commands expose archived/passive artifact DTOs.
They do not install, stage artifacts, query wasm_store, or mutate deployment or
controller state.";
const DEPLOY_PROMOTE_READINESS_HELP_AFTER: &str = "\
Examples:
  canic deploy promote inspect readiness --request promotion-readiness.json
  canic deploy promote inspect readiness --request promotion-readiness.json --text

Reads a PromotionReadinessRequest-shaped JSON file and prints
PromotionReadinessV1 JSON by default, or passive text with --text.";
const DEPLOY_PROMOTE_CHECK_HELP_AFTER: &str = "\
Examples:
  canic deploy promote check --request promotion-check.json
  canic deploy promote check --request promotion-check.json --text

Reads a PromotionReadinessRequest-shaped JSON file and prints a passive
PromotionReadinessV1 check report by default, or passive text with
--text.";
const DEPLOY_PROMOTE_ARTIFACT_IDENTITY_HELP_AFTER: &str = "\
Examples:
  canic deploy promote inspect artifact-identity --request promotion-artifacts.json
  canic deploy promote inspect artifact-identity --request promotion-artifacts.json --text

Reads a PromotionArtifactIdentityReportRequest-shaped JSON file and prints
PromotionArtifactIdentityReportV1 JSON by default, or passive text with
--text.";
const DEPLOY_PROMOTE_TRANSFORM_HELP_AFTER: &str = "\
Examples:
  canic deploy promote inspect transform --request promotion-transform.json
  canic deploy promote inspect transform --request promotion-transform.json --text

Reads a PromotionPlanTransformRequest-shaped JSON file and prints
PromotionPlanTransformV1 JSON by default, or passive text with --text.";
const DEPLOY_PROMOTE_DIFF_HELP_AFTER: &str = "\
Examples:
  canic deploy promote diff --request promotion-diff.json
  canic deploy promote diff --request promotion-diff.json --text

Reads a PromotionPlanTransformRequest-shaped JSON file and prints a passive
PromotionPlanTransformV1 diff report by default, or passive text with
--text.";
const DEPLOY_PROMOTE_TRANSFORM_EVIDENCE_HELP_AFTER: &str = "\
Examples:
  canic deploy promote inspect transform-evidence --request transform-evidence.json
  canic deploy promote inspect transform-evidence --request transform-evidence.json --text

Reads a PromotionPlanTransformEvidenceRequest-shaped JSON file and prints
PromotionPlanTransformEvidenceV1 JSON by default, or passive text with
--text.";
const DEPLOY_PROMOTE_TARGET_LINEAGE_HELP_AFTER: &str = "\
Examples:
  canic deploy promote inspect target-lineage --request target-lineage.json
  canic deploy promote inspect target-lineage --request target-lineage.json --text

Reads a PromotionTargetExecutionLineageRequest-shaped JSON file and prints
PromotionTargetExecutionLineageV1 JSON by default, or passive text with
--text.";
const DEPLOY_PROMOTE_PLAN_HELP_AFTER: &str = "\
Examples:
  canic deploy promote plan --request promotion-plan.json
  canic deploy promote plan --request promotion-plan.json --text

Reads an ArtifactPromotionPlanRequest-shaped JSON file and prints
ArtifactPromotionPlanV1 JSON by default, or passive text with --text.";
const DEPLOY_PROMOTE_PROVENANCE_HELP_AFTER: &str = "\
Examples:
  canic deploy promote inspect provenance --request promotion-provenance.json
  canic deploy promote inspect provenance --request promotion-provenance.json --text

Reads an ArtifactPromotionProvenanceReportRequest-shaped JSON file and prints
ArtifactPromotionProvenanceReportV1 JSON by default, or passive text with
--text.";
const DEPLOY_PROMOTE_WASM_STORE_IDENTITY_HELP_AFTER: &str = "\
Examples:
  canic deploy promote inspect wasm-store-identity --request wasm-store-identity.json
  canic deploy promote inspect wasm-store-identity --request wasm-store-identity.json --text

Reads a PromotionWasmStoreIdentityReportRequest-shaped JSON file and prints
PromotionWasmStoreIdentityReportV1 JSON by default, or passive text with
--text.";
const DEPLOY_PROMOTE_CATALOG_VERIFICATION_HELP_AFTER: &str = "\
Examples:
  canic deploy promote inspect catalog-verification --request catalog-verification.json
  canic deploy promote inspect catalog-verification --request catalog-verification.json --text

Reads a PromotionWasmStoreCatalogVerificationRequest-shaped JSON file and
prints PromotionWasmStoreCatalogVerificationV1 JSON by default, or passive
text with --text.";
const DEPLOY_PROMOTE_EXECUTION_RECEIPT_HELP_AFTER: &str = "\
Examples:
  canic deploy promote inspect execution-receipt --request promotion-execution-receipt.json
  canic deploy promote inspect execution-receipt --request promotion-execution-receipt.json --text

Reads an ArtifactPromotionExecutionReceiptRequest-shaped JSON file and prints
ArtifactPromotionExecutionReceiptV1 JSON by default, or passive text with
--text.";
const DEPLOY_PROMOTE_POLICY_CHECK_HELP_AFTER: &str = "\
Examples:
  canic deploy promote inspect policy --request promotion-policy.json
  canic deploy promote inspect policy --request promotion-policy.json --text

Reads a PromotionPolicyCheckRequest-shaped JSON file and prints
PromotionPolicyCheckV1 JSON by default, or passive text with --text.";
const DEPLOY_PROMOTE_MATERIALIZATION_IDENTITY_HELP_AFTER: &str = "\
Examples:
  canic deploy promote inspect materialization-identity --request materialization.json
  canic deploy promote inspect materialization-identity --request materialization.json --text

Reads a PromotionMaterializationIdentityReportRequest-shaped JSON file and
prints PromotionMaterializationIdentityReportV1 JSON by default, or passive
text with --text.";

const READINESS_REPORT_COMMAND: PromoteReportCommand = PromoteReportCommand {
    name: "readiness",
    about: "Build a passive promotion readiness report",
    bin_name: "canic deploy promote inspect readiness",
    help_after: DEPLOY_PROMOTE_READINESS_HELP_AFTER,
};
const CHECK_REPORT_COMMAND: PromoteReportCommand = PromoteReportCommand {
    name: "check",
    about: "Build a passive artifact promotion readiness check",
    bin_name: "canic deploy promote check",
    help_after: DEPLOY_PROMOTE_CHECK_HELP_AFTER,
};
const ARTIFACT_IDENTITY_REPORT_COMMAND: PromoteReportCommand = PromoteReportCommand {
    name: "artifact-identity",
    about: "Build a passive promotion artifact identity report",
    bin_name: "canic deploy promote inspect artifact-identity",
    help_after: DEPLOY_PROMOTE_ARTIFACT_IDENTITY_HELP_AFTER,
};
const TRANSFORM_REPORT_COMMAND: PromoteReportCommand = PromoteReportCommand {
    name: "transform",
    about: "Build a passive promoted-plan transform",
    bin_name: "canic deploy promote inspect transform",
    help_after: DEPLOY_PROMOTE_TRANSFORM_HELP_AFTER,
};
const DIFF_REPORT_COMMAND: PromoteReportCommand = PromoteReportCommand {
    name: "diff",
    about: "Build a passive artifact promotion diff",
    bin_name: "canic deploy promote diff",
    help_after: DEPLOY_PROMOTE_DIFF_HELP_AFTER,
};
const TRANSFORM_EVIDENCE_REPORT_COMMAND: PromoteReportCommand = PromoteReportCommand {
    name: "transform-evidence",
    about: "Build passive promoted-plan transform evidence",
    bin_name: "canic deploy promote inspect transform-evidence",
    help_after: DEPLOY_PROMOTE_TRANSFORM_EVIDENCE_HELP_AFTER,
};
const TARGET_LINEAGE_REPORT_COMMAND: PromoteReportCommand = PromoteReportCommand {
    name: "target-lineage",
    about: "Build passive target execution lineage",
    bin_name: "canic deploy promote inspect target-lineage",
    help_after: DEPLOY_PROMOTE_TARGET_LINEAGE_HELP_AFTER,
};
const PLAN_REPORT_COMMAND: PromoteReportCommand = PromoteReportCommand {
    name: "plan",
    about: "Build a passive artifact promotion plan",
    bin_name: "canic deploy promote plan",
    help_after: DEPLOY_PROMOTE_PLAN_HELP_AFTER,
};
const PROVENANCE_REPORT_COMMAND: PromoteReportCommand = PromoteReportCommand {
    name: "provenance",
    about: "Build a passive artifact promotion provenance report",
    bin_name: "canic deploy promote inspect provenance",
    help_after: DEPLOY_PROMOTE_PROVENANCE_HELP_AFTER,
};
const WASM_STORE_IDENTITY_REPORT_COMMAND: PromoteReportCommand = PromoteReportCommand {
    name: "wasm-store-identity",
    about: "Build a passive wasm-store identity report",
    bin_name: "canic deploy promote inspect wasm-store-identity",
    help_after: DEPLOY_PROMOTE_WASM_STORE_IDENTITY_HELP_AFTER,
};
const CATALOG_VERIFICATION_REPORT_COMMAND: PromoteReportCommand = PromoteReportCommand {
    name: "catalog-verification",
    about: "Build a passive wasm-store catalog verification report",
    bin_name: "canic deploy promote inspect catalog-verification",
    help_after: DEPLOY_PROMOTE_CATALOG_VERIFICATION_HELP_AFTER,
};
const EXECUTION_RECEIPT_REPORT_COMMAND: PromoteReportCommand = PromoteReportCommand {
    name: "execution-receipt",
    about: "Build a passive artifact promotion execution receipt wrapper",
    bin_name: "canic deploy promote inspect execution-receipt",
    help_after: DEPLOY_PROMOTE_EXECUTION_RECEIPT_HELP_AFTER,
};
const POLICY_CHECK_REPORT_COMMAND: PromoteReportCommand = PromoteReportCommand {
    name: "policy",
    about: "Build a passive promotion policy check",
    bin_name: "canic deploy promote inspect policy",
    help_after: DEPLOY_PROMOTE_POLICY_CHECK_HELP_AFTER,
};
const MATERIALIZATION_IDENTITY_REPORT_COMMAND: PromoteReportCommand = PromoteReportCommand {
    name: "materialization-identity",
    about: "Build a passive source/build materialization identity report",
    bin_name: "canic deploy promote inspect materialization-identity",
    help_after: DEPLOY_PROMOTE_MATERIALIZATION_IDENTITY_HELP_AFTER,
};

pub fn deploy_promote_command() -> ClapCommand {
    TOP_COMMANDS
        .iter()
        .fold(
            ClapCommand::new("promote")
                .bin_name("canic deploy promote")
                .about("Build passive artifact promotion reports")
                .disable_help_flag(true),
            |command, subcommand| command.subcommand(promote_passthrough_command(*subcommand)),
        )
        .after_help(DEPLOY_PROMOTE_HELP_AFTER)
}

pub fn deploy_promote_inspect_command() -> ClapCommand {
    INSPECT_COMMANDS
        .iter()
        .fold(
            ClapCommand::new("inspect")
                .bin_name("canic deploy promote inspect")
                .about("Inspect passive artifact promotion internals")
                .disable_help_flag(true),
            |command, subcommand| command.subcommand(promote_passthrough_command(*subcommand)),
        )
        .after_help(DEPLOY_PROMOTE_INSPECT_HELP_AFTER)
}

pub fn deploy_promote_readiness_command() -> ClapCommand {
    deploy_promote_report_command(READINESS_REPORT_COMMAND)
}

pub fn deploy_promote_check_command() -> ClapCommand {
    deploy_promote_report_command(CHECK_REPORT_COMMAND)
}

pub fn deploy_promote_artifact_identity_command() -> ClapCommand {
    deploy_promote_report_command(ARTIFACT_IDENTITY_REPORT_COMMAND)
}

pub fn deploy_promote_transform_command() -> ClapCommand {
    deploy_promote_report_command(TRANSFORM_REPORT_COMMAND)
}

pub fn deploy_promote_diff_command() -> ClapCommand {
    deploy_promote_report_command(DIFF_REPORT_COMMAND)
}

pub fn deploy_promote_transform_evidence_command() -> ClapCommand {
    deploy_promote_report_command(TRANSFORM_EVIDENCE_REPORT_COMMAND)
}

pub fn deploy_promote_target_lineage_command() -> ClapCommand {
    deploy_promote_report_command(TARGET_LINEAGE_REPORT_COMMAND)
}

pub fn deploy_promote_plan_command() -> ClapCommand {
    deploy_promote_report_command(PLAN_REPORT_COMMAND)
}

pub fn deploy_promote_provenance_command() -> ClapCommand {
    deploy_promote_report_command(PROVENANCE_REPORT_COMMAND)
}

pub fn deploy_promote_wasm_store_identity_command() -> ClapCommand {
    deploy_promote_report_command(WASM_STORE_IDENTITY_REPORT_COMMAND)
}

pub fn deploy_promote_catalog_verification_command() -> ClapCommand {
    deploy_promote_report_command(CATALOG_VERIFICATION_REPORT_COMMAND)
}

pub fn deploy_promote_execution_receipt_command() -> ClapCommand {
    deploy_promote_report_command(EXECUTION_RECEIPT_REPORT_COMMAND)
}

pub fn deploy_promote_policy_check_command() -> ClapCommand {
    deploy_promote_report_command(POLICY_CHECK_REPORT_COMMAND)
}

pub fn deploy_promote_materialization_identity_command() -> ClapCommand {
    deploy_promote_report_command(MATERIALIZATION_IDENTITY_REPORT_COMMAND)
}

fn promote_passthrough_command(spec: PromoteSubcommand) -> ClapCommand {
    passthrough_subcommand(
        ClapCommand::new(spec.name)
            .about(spec.about)
            .disable_help_flag(true),
    )
}

fn deploy_promote_report_command(spec: PromoteReportCommand) -> ClapCommand {
    ClapCommand::new(spec.name)
        .bin_name(spec.bin_name)
        .about(spec.about)
        .disable_help_flag(true)
        .arg(
            value_arg("request")
                .long("request")
                .value_name("file")
                .required(true)
                .help("Request JSON file for the passive promotion report"),
        )
        .arg(text_arg())
        .after_help(spec.help_after)
}

fn text_arg() -> clap::Arg {
    flag_arg(TEXT_ARG)
        .long(TEXT_ARG)
        .help("Print human-readable text output")
}

pub fn promote_usage() -> String {
    render_usage(deploy_promote_command)
}

pub fn promote_inspect_usage() -> String {
    render_usage(deploy_promote_inspect_command)
}

pub fn promote_readiness_usage() -> String {
    render_usage(deploy_promote_readiness_command)
}

pub fn promote_check_usage() -> String {
    render_usage(deploy_promote_check_command)
}

pub fn promote_artifact_identity_usage() -> String {
    render_usage(deploy_promote_artifact_identity_command)
}

pub fn promote_transform_usage() -> String {
    render_usage(deploy_promote_transform_command)
}

pub fn promote_diff_usage() -> String {
    render_usage(deploy_promote_diff_command)
}

pub fn promote_transform_evidence_usage() -> String {
    render_usage(deploy_promote_transform_evidence_command)
}

pub fn promote_target_lineage_usage() -> String {
    render_usage(deploy_promote_target_lineage_command)
}

pub fn promote_plan_usage() -> String {
    render_usage(deploy_promote_plan_command)
}

pub fn promote_provenance_usage() -> String {
    render_usage(deploy_promote_provenance_command)
}

pub fn promote_wasm_store_identity_usage() -> String {
    render_usage(deploy_promote_wasm_store_identity_command)
}

pub fn promote_catalog_verification_usage() -> String {
    render_usage(deploy_promote_catalog_verification_command)
}

pub fn promote_execution_receipt_usage() -> String {
    render_usage(deploy_promote_execution_receipt_command)
}

pub fn promote_policy_check_usage() -> String {
    render_usage(deploy_promote_policy_check_command)
}

pub fn promote_materialization_identity_usage() -> String {
    render_usage(deploy_promote_materialization_identity_command)
}
