use super::*;

#[test]
fn parses_exact_enrollment_contract() {
    let options = EnrollOptions::parse([
        OsString::from("staging"),
        OsString::from("--root-key"),
        OsString::from("trust/root-key.der"),
        OsString::from("--fingerprint"),
        OsString::from("ab".repeat(32)),
    ])
    .expect("parse enrollment");

    assert_eq!(options.environment, "staging");
    assert_eq!(options.root_key, PathBuf::from("trust/root-key.der"));
    assert_eq!(options.fingerprint, "ab".repeat(32));
}

#[test]
fn enrollment_requires_confirmation_inputs() {
    for args in [
        vec![OsString::from("local")],
        vec![
            OsString::from("local"),
            OsString::from("--root-key"),
            OsString::from("root-key.der"),
        ],
        vec![
            OsString::from("local"),
            OsString::from("--fingerprint"),
            OsString::from("ab".repeat(32)),
        ],
    ] {
        std::assert_matches!(
            EnrollOptions::parse(args),
            Err(NetworkCommandError::Usage(_))
        );
    }
}

#[test]
fn renders_canonical_identity_and_idempotent_status() {
    let report = NetworkEnrollmentReport {
        environment: "local".to_string(),
        canonical_network_id: "04".repeat(32).parse().expect("canonical network ID"),
        root_key_fingerprint: "ab".repeat(32),
        authority_directory: PathBuf::from(".canic/networks/id"),
        profile_path: PathBuf::from(".canic/environment-profiles/local/network.json"),
        created_profile: false,
    };

    let output = render_enrollment(&report);

    assert!(output.contains(&format!(
        "canonical_network_id: {}",
        report.canonical_network_id
    )));
    assert!(output.contains("status: already_enrolled"));
}
