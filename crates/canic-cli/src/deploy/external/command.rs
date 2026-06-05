use super::super::{deploy_truth_leaf_command, output_format::ExternalOutputFormat};
use crate::cli::clap::{passthrough_subcommand, render_usage, value_arg};
use clap::Command as ClapCommand;

#[derive(Clone, Copy)]
struct ExternalSubcommand {
    name: &'static str,
    about: &'static str,
}

#[derive(Clone, Copy)]
struct ExternalTruthCommand {
    name: &'static str,
    about: &'static str,
    bin_name: &'static str,
    help_after: &'static str,
}

#[derive(Clone, Copy)]
struct ExternalRequestCommand {
    name: &'static str,
    about: &'static str,
    bin_name: &'static str,
    usage: &'static str,
    request_help: &'static str,
    help_after: &'static str,
}

const TOP_COMMANDS: &[ExternalSubcommand] = &[
    ExternalSubcommand {
        name: "plan",
        about: "Build a passive external lifecycle plan",
    },
    ExternalSubcommand {
        name: "check",
        about: "Build a passive external lifecycle check",
    },
    ExternalSubcommand {
        name: "handoff",
        about: "Build a passive external lifecycle handoff packet",
    },
    ExternalSubcommand {
        name: "proposals",
        about: "Build passive external upgrade proposals",
    },
    ExternalSubcommand {
        name: "pending",
        about: "Build a passive external lifecycle pending report",
    },
    ExternalSubcommand {
        name: "critical-fix",
        about: "Build a passive critical external fix report",
    },
    ExternalSubcommand {
        name: "inspect",
        about: "Inspect passive external lifecycle internals",
    },
    ExternalSubcommand {
        name: "verify",
        about: "Build a passive external upgrade verification report",
    },
];

const INSPECT_COMMANDS: &[ExternalSubcommand] = &[
    ExternalSubcommand {
        name: "consent",
        about: "Build passive external consent evidence",
    },
    ExternalSubcommand {
        name: "verification-policy",
        about: "Build passive external verification policy",
    },
    ExternalSubcommand {
        name: "verification-check",
        about: "Build passive external verification check",
    },
    ExternalSubcommand {
        name: "completion",
        about: "Build passive external completion report",
    },
];

const DEPLOY_EXTERNAL_HELP_AFTER: &str = "\
Examples:
  canic deploy external plan demo
  canic deploy external check demo
  canic deploy external handoff demo
  canic deploy external proposals demo
  canic deploy external pending demo
  canic deploy external critical-fix --fix-id fix-2026-05 --severity critical demo
  canic deploy external inspect consent --request external-consent.json
  canic deploy external inspect verification-policy --request external-verification-policy.json
  canic deploy external inspect verification-check --request external-verification-check.json
  canic deploy external verify --request external-verification.json
  canic deploy external plan --format text demo
  canic deploy external verify --request external-verification.json --format text
  canic --network local deploy external critical-fix --fix-id fix-2026-05 --severity high --profile fast demo

0.45 external lifecycle commands are passive reports. They do not request
consent, execute external upgrades, install code, or mutate deployment state.";
const DEPLOY_EXTERNAL_INSPECT_HELP_AFTER: &str = "\
Examples:
  canic deploy external inspect consent --request external-consent.json
  canic deploy external inspect consent --request external-consent.json --format text
  canic deploy external inspect verification-policy --request external-verification-policy.json
  canic deploy external inspect verification-policy --request external-verification-policy.json --format text
  canic deploy external inspect verification-check --request external-verification-check.json
  canic deploy external inspect verification-check --request external-verification-check.json --format text
  canic deploy external inspect completion --request external-completion.json
  canic deploy external inspect completion --request external-completion.json --format text

Advanced external lifecycle inspection commands expose archived/passive DTOs.
They do not request consent, execute external upgrades, install code, or mutate
deployment state.";
const DEPLOY_EXTERNAL_CONSENT_HELP_AFTER: &str = "\
Examples:
  canic deploy external inspect consent --request external-consent.json
  canic deploy external inspect consent --request external-consent.json --format text

Reads an ExternalUpgradeConsentEvidenceRequest-shaped JSON file and prints
ExternalUpgradeConsentEvidenceV1 JSON by default, or host-owned passive text
with --format text. Consent evidence records reported consent/action state; it
does not verify live completion.";
const DEPLOY_EXTERNAL_VERIFICATION_POLICY_HELP_AFTER: &str = "\
Examples:
  canic deploy external inspect verification-policy --request external-verification-policy.json
  canic deploy external inspect verification-policy --request external-verification-policy.json --format text

Reads an ExternalUpgradeVerificationPolicyRequest-shaped JSON file and prints
ExternalUpgradeVerificationPolicyV1 JSON by default, or host-owned passive text
with --format text. Verification policies describe required live-inventory
postconditions; they do not query live inventory or verify completion.";
const DEPLOY_EXTERNAL_VERIFICATION_CHECK_HELP_AFTER: &str = "\
Examples:
  canic deploy external inspect verification-check --request external-verification-check.json
  canic deploy external inspect verification-check --request external-verification-check.json --format text

Reads an ExternalUpgradeVerificationCheckRequest-shaped JSON file and prints
ExternalUpgradeVerificationCheckV1 JSON by default, or host-owned passive text
with --format text. Verification checks evaluate supplied observation facts or
an embedded DeploymentCheckV1 inventory artifact against a verification policy;
they do not query live inventory or execute external lifecycle work.";
const DEPLOY_EXTERNAL_COMPLETION_HELP_AFTER: &str = "\
Examples:
  canic deploy external inspect completion --request external-completion.json
  canic deploy external inspect completion --request external-completion.json --format text

Reads an ExternalUpgradeCompletionReportRequest-shaped JSON file and prints
ExternalUpgradeCompletionReportV1 JSON by default, or host-owned passive text
with --format text. Completion reports combine proposal, consent evidence, and
verification-check evidence; only deployment-truth inventory verification can
mark external lifecycle work verified complete.";
const DEPLOY_EXTERNAL_PLAN_HELP_AFTER: &str = "\
Examples:
  canic deploy external plan demo
  canic deploy external plan --format text demo
  canic --network local deploy external plan --profile fast demo

Prints ExternalLifecyclePlanV1 JSON by default, or host-owned passive text with
--format text. No consent delivery, external execution, install, or mutation is
attempted.";
const DEPLOY_EXTERNAL_CHECK_HELP_AFTER: &str = "\
Examples:
  canic deploy external check demo
  canic deploy external check --format text demo
  canic --network local deploy external check --profile fast demo

Prints ExternalLifecycleCheckV1 JSON by default, or host-owned passive text
with --format text. External lifecycle checks summarize direct, pending,
blocked, and residual-exposure status without requesting consent, executing
external upgrades, or mutating state.";
const DEPLOY_EXTERNAL_HANDOFF_HELP_AFTER: &str = "\
Examples:
  canic deploy external handoff demo
  canic deploy external handoff --format text demo
  canic --network local deploy external handoff --profile fast demo

Prints ExternalLifecycleHandoffV1 JSON by default, or host-owned passive text
with --format text. Handoff packets package pending external proposals into
operator coordination instructions; they do not deliver consent, execute
external upgrades, or mutate state.";
const DEPLOY_EXTERNAL_PROPOSALS_HELP_AFTER: &str = "\
Examples:
  canic deploy external proposals demo
  canic deploy external proposals --format text demo
  canic --network local deploy external proposals --profile fast demo

Prints ExternalUpgradeProposalReportV1 JSON by default, or host-owned passive
text with --format text. Proposals are derived from the local lifecycle plan
and do not grant consent or execute upgrades.";
const DEPLOY_EXTERNAL_PENDING_HELP_AFTER: &str = "\
Examples:
  canic deploy external pending demo
  canic deploy external pending --format text demo
  canic --network local deploy external pending --profile fast demo

Prints ExternalLifecyclePendingReportV1 JSON by default, or host-owned passive
text with --format text. Pending reports summarize unresolved external actions,
blocked subjects, and residual exposure without requesting consent or executing
upgrades.";
const DEPLOY_EXTERNAL_CRITICAL_FIX_HELP_AFTER: &str = "\
Examples:
  canic deploy external critical-fix --fix-id fix-2026-05 --severity critical demo
  canic deploy external critical-fix --fix-id fix-2026-05 --severity critical --format text demo
  canic --network local deploy external critical-fix --fix-id fix-2026-05 --severity high --profile fast demo

Prints CriticalExternalFixReportV1 JSON by default, or host-owned passive text
with --format text. Critical-fix reports summarize directly patchable roles,
external blockers, required external actions, protected-call implications, and
residual exposure without claiming deployment completion or mutating state.";
const DEPLOY_EXTERNAL_VERIFY_HELP_AFTER: &str = "\
Examples:
  canic deploy external verify --request external-verification.json
  canic deploy external verify --request external-verification.json --format text

Reads an ExternalUpgradeVerificationReportRequest-shaped JSON file and prints
ExternalUpgradeVerificationReportV1 JSON by default, or host-owned passive text
with --format text. Verification reports package proposal/receipt structural
evidence only; live inventory remains the source of truth for deployment
state.";

const PLAN_COMMAND: ExternalTruthCommand = ExternalTruthCommand {
    name: "plan",
    about: "Print the local external lifecycle plan",
    bin_name: "canic deploy external plan",
    help_after: DEPLOY_EXTERNAL_PLAN_HELP_AFTER,
};
const CHECK_COMMAND: ExternalTruthCommand = ExternalTruthCommand {
    name: "check",
    about: "Print the local external lifecycle check",
    bin_name: "canic deploy external check",
    help_after: DEPLOY_EXTERNAL_CHECK_HELP_AFTER,
};
const HANDOFF_COMMAND: ExternalTruthCommand = ExternalTruthCommand {
    name: "handoff",
    about: "Print the local external lifecycle handoff",
    bin_name: "canic deploy external handoff",
    help_after: DEPLOY_EXTERNAL_HANDOFF_HELP_AFTER,
};
const PROPOSALS_COMMAND: ExternalTruthCommand = ExternalTruthCommand {
    name: "proposals",
    about: "Print local external upgrade proposals",
    bin_name: "canic deploy external proposals",
    help_after: DEPLOY_EXTERNAL_PROPOSALS_HELP_AFTER,
};
const PENDING_COMMAND: ExternalTruthCommand = ExternalTruthCommand {
    name: "pending",
    about: "Print the local external lifecycle pending report",
    bin_name: "canic deploy external pending",
    help_after: DEPLOY_EXTERNAL_PENDING_HELP_AFTER,
};
const CRITICAL_FIX_COMMAND: ExternalTruthCommand = ExternalTruthCommand {
    name: "critical-fix",
    about: "Print the local critical external fix report",
    bin_name: "canic deploy external critical-fix",
    help_after: DEPLOY_EXTERNAL_CRITICAL_FIX_HELP_AFTER,
};
const VERIFY_COMMAND: ExternalRequestCommand = ExternalRequestCommand {
    name: "verify",
    about: "Build a passive external upgrade verification report",
    bin_name: "canic deploy external verify",
    usage: "canic deploy external verify --request <file>",
    request_help: "ExternalUpgradeVerificationReportRequest JSON file to verify",
    help_after: DEPLOY_EXTERNAL_VERIFY_HELP_AFTER,
};
const CONSENT_COMMAND: ExternalRequestCommand = ExternalRequestCommand {
    name: "consent",
    about: "Build passive external consent evidence",
    bin_name: "canic deploy external inspect consent",
    usage: "canic deploy external inspect consent --request <file>",
    request_help: "ExternalUpgradeConsentEvidenceRequest JSON file to inspect",
    help_after: DEPLOY_EXTERNAL_CONSENT_HELP_AFTER,
};
const VERIFICATION_POLICY_COMMAND: ExternalRequestCommand = ExternalRequestCommand {
    name: "verification-policy",
    about: "Build passive external verification policy",
    bin_name: "canic deploy external inspect verification-policy",
    usage: "canic deploy external inspect verification-policy --request <file>",
    request_help: "ExternalUpgradeVerificationPolicyRequest JSON file to inspect",
    help_after: DEPLOY_EXTERNAL_VERIFICATION_POLICY_HELP_AFTER,
};
const VERIFICATION_CHECK_COMMAND: ExternalRequestCommand = ExternalRequestCommand {
    name: "verification-check",
    about: "Build passive external verification check",
    bin_name: "canic deploy external inspect verification-check",
    usage: "canic deploy external inspect verification-check --request <file>",
    request_help: "ExternalUpgradeVerificationCheckRequest JSON file to inspect",
    help_after: DEPLOY_EXTERNAL_VERIFICATION_CHECK_HELP_AFTER,
};
const COMPLETION_COMMAND: ExternalRequestCommand = ExternalRequestCommand {
    name: "completion",
    about: "Build passive external completion report",
    bin_name: "canic deploy external inspect completion",
    usage: "canic deploy external inspect completion --request <file>",
    request_help: "ExternalUpgradeCompletionReportRequest JSON file to inspect",
    help_after: DEPLOY_EXTERNAL_COMPLETION_HELP_AFTER,
};

pub fn command() -> ClapCommand {
    TOP_COMMANDS
        .iter()
        .fold(
            ClapCommand::new("external")
                .bin_name("canic deploy external")
                .about("Build passive external lifecycle reports")
                .disable_help_flag(true),
            |command, subcommand| command.subcommand(external_passthrough_command(*subcommand)),
        )
        .after_help(DEPLOY_EXTERNAL_HELP_AFTER)
}

pub fn inspect_command() -> ClapCommand {
    INSPECT_COMMANDS
        .iter()
        .fold(
            ClapCommand::new("inspect")
                .bin_name("canic deploy external inspect")
                .about("Inspect passive external lifecycle internals")
                .disable_help_flag(true),
            |command, subcommand| command.subcommand(external_passthrough_command(*subcommand)),
        )
        .after_help(DEPLOY_EXTERNAL_INSPECT_HELP_AFTER)
}

pub fn plan_command() -> ClapCommand {
    external_truth_command(PLAN_COMMAND)
}

pub fn check_command() -> ClapCommand {
    external_truth_command(CHECK_COMMAND)
}

pub fn handoff_command() -> ClapCommand {
    external_truth_command(HANDOFF_COMMAND)
}

pub fn proposals_command() -> ClapCommand {
    external_truth_command(PROPOSALS_COMMAND)
}

pub fn pending_command() -> ClapCommand {
    external_truth_command(PENDING_COMMAND)
}

pub fn critical_fix_command() -> ClapCommand {
    external_truth_command(CRITICAL_FIX_COMMAND)
        .arg(
            value_arg("fix-id")
                .long("fix-id")
                .value_name("id")
                .required(true)
                .help("Critical fix identifier to record in the report"),
        )
        .arg(
            value_arg("severity")
                .long("severity")
                .value_name("severity")
                .required(true)
                .help("Critical fix severity label to record in the report"),
        )
}

pub fn verify_command() -> ClapCommand {
    external_request_command(VERIFY_COMMAND)
}

pub fn consent_command() -> ClapCommand {
    external_request_command(CONSENT_COMMAND)
}

pub fn verification_policy_command() -> ClapCommand {
    external_request_command(VERIFICATION_POLICY_COMMAND)
}

pub fn verification_check_command() -> ClapCommand {
    external_request_command(VERIFICATION_CHECK_COMMAND)
}

pub fn completion_command() -> ClapCommand {
    external_request_command(COMPLETION_COMMAND)
}

fn format_arg() -> clap::Arg {
    value_arg("format")
        .long("format")
        .value_name("json|text")
        .num_args(1)
        .default_value("json")
        .value_parser(clap::value_parser!(ExternalOutputFormat))
        .help("Output format; defaults to json")
}

fn external_passthrough_command(spec: ExternalSubcommand) -> ClapCommand {
    passthrough_subcommand(
        ClapCommand::new(spec.name)
            .about(spec.about)
            .disable_help_flag(true),
    )
}

fn external_truth_command(spec: ExternalTruthCommand) -> ClapCommand {
    deploy_truth_leaf_command(spec.name, spec.about)
        .arg(format_arg())
        .bin_name(spec.bin_name)
        .after_help(spec.help_after)
}

fn external_request_command(spec: ExternalRequestCommand) -> ClapCommand {
    ClapCommand::new(spec.name)
        .bin_name(spec.bin_name)
        .about(spec.about)
        .disable_help_flag(true)
        .override_usage(spec.usage)
        .arg(
            value_arg("request")
                .long("request")
                .value_name("file")
                .required(true)
                .help(spec.request_help),
        )
        .arg(format_arg())
        .after_help(spec.help_after)
}

pub fn usage() -> String {
    render_usage(command)
}

pub fn plan_usage() -> String {
    render_usage(plan_command)
}

pub fn check_usage() -> String {
    render_usage(check_command)
}

pub fn handoff_usage() -> String {
    render_usage(handoff_command)
}

pub fn proposals_usage() -> String {
    render_usage(proposals_command)
}

pub fn pending_usage() -> String {
    render_usage(pending_command)
}

pub fn critical_fix_usage() -> String {
    render_usage(critical_fix_command)
}

pub fn inspect_usage() -> String {
    render_usage(inspect_command)
}

pub fn consent_usage() -> String {
    render_usage(consent_command)
}

pub fn verification_policy_usage() -> String {
    render_usage(verification_policy_command)
}

pub fn verification_check_usage() -> String {
    render_usage(verification_check_command)
}

pub fn completion_usage() -> String {
    render_usage(completion_command)
}

pub fn verify_usage() -> String {
    render_usage(verify_command)
}
