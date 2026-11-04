//! Protected Auth observations routed through the target's current Fleet Root.
use super::*;
use canic_contracts::dto::{
    observability::{CanisterObservabilityRequest, CanisterObservabilityResponse},
    portable_inspection::{self, RuntimeState},
};
use canic_host::observability::{FleetObservabilityError, observe_fleet_canister};
use ic_platform::principal::ExternalPrincipal as Principal;

/// Preserve routing, local selection and receiver refusal as separate outcomes.
#[derive(Debug, ThisError)]
pub enum InspectionError {
    #[error("invalid {field} principal: {value}")]
    Principal { field: &'static str, value: String },
    #[error("canister {0} is not in the current Fleet")]
    UnknownCanister(String),
    #[error(transparent)]
    Fleet(#[from] CurrentFleetInventoryError),
    #[error(transparent)]
    Root(#[from] IcpConfigError),
    #[error(transparent)]
    Observation(#[from] FleetObservabilityError),
    #[error("receiver inspection refused: {0:?}")]
    Refused(portable_inspection::Refusal),
    #[error("Root returned a different observation kind")]
    UnexpectedResponse,
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

fn common(command: ClapCommand) -> ClapCommand {
    command
        .arg(
            value_arg(FLEET_ARG)
                .required(true)
                .help("Terminal current Fleet name"),
        )
        .arg(
            value_arg("canister")
                .long("canister")
                .required(true)
                .help("Receiver canister principal"),
        )
        .arg(flag_arg(JSON_ARG).long(JSON_ARG).help("Print JSON output"))
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
}

pub(super) fn policy_command() -> ClapCommand {
    common(
        ClapCommand::new("policy")
            .about("Show declared policy and current protected issuer enrollment"),
    )
    .after_help("Example:\n  canic auth policy local --canister rrkah-fqaaa-aaaaa-aaaaq-cai --json")
}

pub(super) fn sessions_command() -> ClapCommand {
    common(
        ClapCommand::new("sessions")
            .about("Inspect one bounded page of retained portable sessions"),
    )
    .arg(
        value_arg("after")
            .long("after")
            .help("Exclusive caller cursor from the previous page"),
    )
    .arg(
        value_arg("limit")
            .long("limit")
            .default_value("32")
            .value_parser(clap::value_parser!(u16).range(1..=32)),
    )
    .after_help(
        "Example:\n  canic auth sessions local --canister rrkah-fqaaa-aaaaa-aaaaq-cai --limit 16",
    )
}

fn principal(field: &'static str, value: &str) -> Result<Principal, InspectionError> {
    Principal::from_text(value).map_err(|_| InspectionError::Principal {
        field,
        value: value.into(),
    })
}

fn request(
    name: &str,
    matches: &clap::ArgMatches,
) -> Result<CanisterObservabilityRequest, InspectionError> {
    if name == "policy" {
        return Ok(CanisterObservabilityRequest::PortablePolicy);
    }
    let after = matches
        .get_one::<String>("after")
        .map(|value| principal("after", value).map(Into::into))
        .transpose()?;
    Ok(CanisterObservabilityRequest::PortableSessions(
        portable_inspection::Request {
            after,
            limit: *matches
                .get_one::<u16>("limit")
                .expect("default bounded page size"),
        },
    ))
}

pub(super) fn run(name: &str, matches: &clap::ArgMatches) -> Result<(), InspectionError> {
    let common = common_options(matches);
    let fleet_name = required_string(matches, FLEET_ARG);
    let target = principal("canister", &required_string(matches, "canister"))?.to_text();
    let request = request(name, matches)?;
    let root = resolve_current_canic_icp_root()?;
    let fleet = resolve_current_fleet(&root, &common.environment, &fleet_name)?;
    let entry = fleet
        .registry
        .entries
        .iter()
        .find(|entry| entry.pid == target)
        .ok_or_else(|| InspectionError::UnknownCanister(target.clone()))?;
    let result = observe_fleet_canister(&icp_cli(&common), &root, &fleet, entry, request)?;
    let json = matches.get_flag(JSON_ARG);
    match result {
        CanisterObservabilityResponse::PortableSessions(result) if name == "sessions" => {
            let page = result.map_err(InspectionError::Refused)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&page)?);
            } else {
                println!(
                    "Canister {target}: generation {} at {} ns",
                    page.generation, page.observed_at_ns
                );
                for row in page.sessions {
                    println!(
                        "caller={} issuer={} subject={} scopes={} expires={} generation={} expired={} stale={}",
                        row.caller,
                        row.issuer,
                        row.subject,
                        row.scopes.join(","),
                        row.expires_at_ns,
                        row.generation,
                        row.expired,
                        row.stale_generation
                    );
                }
                if let Some(after) = page.next_after {
                    println!("Next page: --after {after}");
                }
            }
        }
        CanisterObservabilityResponse::PortablePolicy(result) if name == "policy" => {
            let report = result.map_err(InspectionError::Refused)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "Canister {target}: generation {} at {} ns",
                    report.authority_generation, report.observed_at_ns
                );
                if let Some(declared) = report.declared {
                    println!(
                        "Role: {}\nAllowed scopes: {}\nSession TTL: default={}s maximum={}s",
                        declared.role,
                        declared.allowed_scopes.join(","),
                        declared.default_session_ttl_secs,
                        declared.maximum_session_ttl_secs
                    );
                    if let Some(receiver) = declared.receiver {
                        println!(
                            "Clock skew: {}ns; proof TTL: {}ns; login TTL: {}ns; certificate age: {}ns",
                            receiver.clock_skew_ns,
                            receiver.max_proof_ttl_ns,
                            receiver.max_login_ttl_ns,
                            receiver.max_certificate_age_ns
                        );
                    }
                }
                match report.runtime {
                    RuntimeState::Ready(enrollment) => {
                        println!(
                            "Enrollment: ready; Root={} revision={} generation={} expires={}ns",
                            enrollment.authority,
                            enrollment.revision,
                            enrollment.generation,
                            enrollment.expires_at_ns
                        );
                        for issuer in enrollment.issuers {
                            println!(
                                "Issuer: {} expires={}ns grants={}",
                                issuer.issuer,
                                issuer.expires_at_ns,
                                serde_json::to_string(&issuer.grants)?
                            );
                        }
                    }
                    other => println!("Receiver: {other:?}"),
                }
            }
        }
        _ => return Err(InspectionError::UnexpectedResponse),
    }
    Ok(())
}

impl InspectionError {
    pub(super) const fn exit_code(&self) -> u8 {
        match self {
            Self::Principal { .. }
            | Self::UnknownCanister(_)
            | Self::Fleet(_)
            | Self::Root(_)
            | Self::Json(_) => 1,
            Self::Observation(_) => 2,
            Self::Refused(_) | Self::UnexpectedResponse => 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn session_command_bounds_pages_and_preserves_the_exclusive_cursor() {
        let caller = Principal::from_slice(&[7]);
        let args = [
            "sessions",
            "local",
            "--canister",
            "aaaaa-aa",
            "--after",
            &caller.to_text(),
            "--limit",
            "2",
        ];
        let matches = sessions_command().try_get_matches_from(args).unwrap();
        let CanisterObservabilityRequest::PortableSessions(request) =
            request("sessions", &matches).unwrap()
        else {
            panic!("session request");
        };
        assert_eq!(request.after, Some(caller.into()));
        assert_eq!(request.limit, 2);
        for limit in ["0", "33", "65536"] {
            assert!(
                sessions_command()
                    .try_get_matches_from([
                        "sessions",
                        "local",
                        "--canister",
                        "aaaaa-aa",
                        "--limit",
                        limit
                    ])
                    .is_err()
            );
        }
    }
    #[test]
    fn policy_has_no_session_cursor_and_invalid_cursors_are_refused() {
        assert!(
            policy_command()
                .try_get_matches_from([
                    "policy",
                    "local",
                    "--canister",
                    "aaaaa-aa",
                    "--after",
                    "aaaaa-aa"
                ])
                .is_err()
        );
        let matches = sessions_command()
            .try_get_matches_from([
                "sessions",
                "local",
                "--canister",
                "aaaaa-aa",
                "--after",
                "bad",
            ])
            .unwrap();
        assert!(matches!(
            request("sessions", &matches),
            Err(InspectionError::Principal { field: "after", .. })
        ));
    }
}
