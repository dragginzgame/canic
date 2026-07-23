use super::*;
use crate::test_support::temp_dir;
use std::{fmt::Write as _, fs};

fn fixture(name: &str) -> (PathBuf, PathBuf, Vec<u8>, String) {
    let root = temp_dir(&format!("canic-network-{name}"));
    fs::create_dir_all(&root).expect("create project root");
    let mut root_key = vec![
        0x30, 0x81, 0x82, 0x30, 0x1d, 0x06, 0x0d, 0x2b, 0x06, 0x01, 0x04, 0x01, 0x82, 0xdc, 0x7c,
        0x05, 0x03, 0x01, 0x02, 0x01, 0x06, 0x0c, 0x2b, 0x06, 0x01, 0x04, 0x01, 0x82, 0xdc, 0x7c,
        0x05, 0x03, 0x02, 0x01, 0x03, 0x61, 0x00,
    ];
    root_key.extend_from_slice(&[9; 96]);
    let root_key_path = root.join("root-key.der");
    fs::write(&root_key_path, &root_key).expect("write root key");
    let fingerprint = encode_digest(sha256_digest(&root_key));
    (root, root_key_path, root_key, fingerprint)
}

fn enroll<'a>(
    root: &'a Path,
    environment: &'a str,
    root_key: &'a Path,
    fingerprint: &'a str,
) -> NetworkEnrollmentOptions<'a> {
    NetworkEnrollmentOptions {
        project_root: root,
        environment,
        root_key,
        fingerprint,
    }
}

fn write_named_local_environment(root: &Path, names: &[&str]) {
    let environments = names.iter().fold(String::new(), |mut output, name| {
        writeln!(output, "  - name: {name}\n    network: shared")
            .expect("writing to a String cannot fail");
        output
    });
    fs::write(
        root.join("icp.yaml"),
        format!(
            "networks:\n  - name: shared\n    mode: managed\n    gateway:\n      port: 8000\nenvironments:\n{environments}"
        ),
    )
    .expect("write icp.yaml");
}

#[test]
fn enrollment_publishes_authority_before_profile_and_resolves_it() {
    let (root, root_key_path, root_key, fingerprint) = fixture("success");

    let report = enroll_network(enroll(&root, "local", &root_key_path, &fingerprint))
        .expect("enroll network");

    assert!(report.created_profile);
    assert_eq!(
        fs::read(report.authority_directory.join(ROOT_KEY_RELATIVE_PATH))
            .expect("read enrolled root key"),
        root_key
    );
    let profile = fs::read_to_string(&report.profile_path).expect("read profile");
    assert_eq!(
        profile,
        format!(
            "{{\n  \"canonical_network_id\": \"{}\"\n}}\n",
            report.canonical_network_id
        )
    );
    assert_eq!(
        resolve_canonical_network_id_from_root(&root, "local").expect("resolve network"),
        report.canonical_network_id
    );

    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn exact_reenrollment_is_idempotent() {
    let (root, root_key_path, _, fingerprint) = fixture("idempotent");
    let options = enroll(&root, "local", &root_key_path, &fingerprint);

    let first = enroll_network(options).expect("first enrollment");
    let second = enroll_network(options).expect("repeat enrollment");

    assert!(first.created_profile);
    assert!(!second.created_profile);
    assert_eq!(first.canonical_network_id, second.canonical_network_id);
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn enrollment_resumes_after_authority_root_was_published() {
    let (root, root_key_path, _, fingerprint) = fixture("resume");
    let first = enroll_network(enroll(&root, "local", &root_key_path, &fingerprint))
        .expect("initial enrollment");
    fs::remove_file(first.authority_directory.join(ENROLLMENT_FILE))
        .expect("remove enrollment record");
    fs::remove_file(&first.profile_path).expect("remove profile");

    let resumed = enroll_network(enroll(&root, "local", &root_key_path, &fingerprint))
        .expect("resume enrollment");

    assert!(resumed.created_profile);
    assert_eq!(resumed.canonical_network_id, first.canonical_network_id);
    assert_eq!(
        resolve_canonical_network_id_from_root(&root, "local").expect("resolve resumed network"),
        first.canonical_network_id
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn two_environment_aliases_share_one_network_authority() {
    let (root, root_key_path, _, fingerprint) = fixture("aliases");
    write_named_local_environment(&root, &["dev", "qa"]);

    let dev =
        enroll_network(enroll(&root, "dev", &root_key_path, &fingerprint)).expect("enroll dev");
    let qa = enroll_network(enroll(&root, "qa", &root_key_path, &fingerprint)).expect("enroll qa");

    assert_eq!(dev.canonical_network_id, qa.canonical_network_id);
    assert_eq!(dev.authority_directory, qa.authority_directory);
    assert_eq!(
        resolve_canonical_network_id_from_root(&root, "qa").expect("resolve qa"),
        dev.canonical_network_id
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn fingerprint_mismatch_writes_nothing() {
    let (root, root_key_path, _, _) = fixture("mismatch");

    let error = enroll_network(enroll(&root, "local", &root_key_path, &"00".repeat(32)))
        .expect_err("mismatched fingerprint must reject");

    std::assert_matches!(error, NetworkIdentityError::FingerprintMismatch { .. });
    assert!(!root.join(CANIC_STATE_DIRECTORY).exists());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn noncanonical_fingerprint_writes_nothing() {
    let (root, root_key_path, _, fingerprint) = fixture("noncanonical-fingerprint");

    let error = enroll_network(enroll(
        &root,
        "local",
        &root_key_path,
        &fingerprint.to_uppercase(),
    ))
    .expect_err("uppercase fingerprint must reject");

    std::assert_matches!(error, NetworkIdentityError::InvalidFingerprint);
    assert!(!root.join(CANIC_STATE_DIRECTORY).exists());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn changed_anchor_cannot_rebind_an_existing_profile() {
    let (root, root_key_path, mut root_key, fingerprint) = fixture("rotation");
    let first = enroll_network(enroll(&root, "local", &root_key_path, &fingerprint))
        .expect("enroll first anchor");
    *root_key.last_mut().expect("root key byte") ^= 1;
    let changed_path = root.join("changed-root-key.der");
    fs::write(&changed_path, &root_key).expect("write changed root key");
    let changed_fingerprint = encode_digest(sha256_digest(&root_key));

    let error = enroll_network(enroll(&root, "local", &changed_path, &changed_fingerprint))
        .expect_err("profile rebinding must reject");

    std::assert_matches!(
        error,
        NetworkIdentityError::ProfileConflict { existing, .. }
            if existing == first.canonical_network_id
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn resolver_rejects_a_profile_without_complete_authority() {
    let (root, _, _, _) = fixture("incomplete");
    let identity = "03".repeat(32).parse().expect("canonical network ID");
    let profile_path = environment_profile_path(&root, "local");
    fs::create_dir_all(profile_path.parent().expect("profile parent"))
        .expect("create profile parent");
    fs::write(
        &profile_path,
        serde_json::to_vec(&EnvironmentNetworkProfile {
            canonical_network_id: identity,
        })
        .expect("encode profile"),
    )
    .expect("write profile");

    let error = resolve_canonical_network_id_from_root(&root, "local")
        .expect_err("incomplete authority must reject");

    std::assert_matches!(error, NetworkIdentityError::MissingAuthority { .. });
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn resolver_rejects_a_digest_that_disagrees_with_the_exact_anchor() {
    let (root, root_key_path, _, fingerprint) = fixture("digest-conflict");
    let report = enroll_network(enroll(&root, "local", &root_key_path, &fingerprint))
        .expect("enroll network");
    let enrollment_path = report.authority_directory.join(ENROLLMENT_FILE);
    let enrollment_bytes = fs::read(&enrollment_path).expect("read enrollment");
    let mut enrollment = serde_json::from_slice::<NetworkEnrollmentRecord>(&enrollment_bytes)
        .expect("decode enrollment");
    enrollment.root_key_digest = [0; 32];
    fs::write(
        &enrollment_path,
        serde_json::to_vec_pretty(&enrollment).expect("encode changed enrollment"),
    )
    .expect("replace enrollment");

    let error = resolve_canonical_network_id_from_root(&root, "local")
        .expect_err("contradictory digest must reject");

    std::assert_matches!(error, NetworkIdentityError::ContradictoryAuthority { .. });
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn public_ic_environment_aliases_resolve_the_compiled_identity() {
    let (root, _, _, _) = fixture("public");
    fs::write(
        root.join("icp.yaml"),
        "environments:\n  - name: staging\n    network: ic\n  - name: production\n    network: ic\n",
    )
    .expect("write public aliases");

    assert_eq!(
        resolve_canonical_network_id_from_root(&root, "ic").expect("resolve ic"),
        resolve_canonical_network_id_from_root(&root, "staging").expect("resolve staging")
    );
    assert_eq!(
        resolve_canonical_network_id_from_root(&root, "staging").expect("resolve staging"),
        resolve_canonical_network_id_from_root(&root, "production").expect("resolve production")
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn public_ic_trust_anchor_cannot_be_enrolled() {
    let (root, root_key_path, _, fingerprint) = fixture("public-enrollment");

    let error = enroll_network(enroll(&root, "ic", &root_key_path, &fingerprint))
        .expect_err("public IC enrollment must reject");

    std::assert_matches!(error, NetworkIdentityError::PublicIcEnrollment { .. });
    assert!(!root.join(CANIC_STATE_DIRECTORY).exists());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[cfg(unix)]
#[test]
fn enrollment_rejects_symlink_and_special_root_key_inputs_without_writing() {
    use std::os::unix::{fs::symlink, net::UnixListener};

    let (root, root_key_path, _, fingerprint) = fixture("unsafe-input");
    let symlink_path = root.join("root-key-link.der");
    symlink(&root_key_path, &symlink_path).expect("create symlink");

    let symlink_error = enroll_network(enroll(&root, "local", &symlink_path, &fingerprint))
        .expect_err("symlink input must reject");
    assert!(
        matches!(
            symlink_error,
            NetworkIdentityError::Io { .. } | NetworkIdentityError::RootKeyNotRegular { .. }
        ),
        "unexpected symlink error: {symlink_error}"
    );

    let socket_path = root.join("root-key.sock");
    let socket = UnixListener::bind(&socket_path).expect("bind unix socket");
    let socket_error = enroll_network(enroll(&root, "local", &socket_path, &fingerprint))
        .expect_err("special input must reject");
    std::assert_matches!(socket_error, NetworkIdentityError::RootKeyNotRegular { .. });
    assert!(!root.join(CANIC_STATE_DIRECTORY).exists());
    drop(socket);
    fs::remove_file(socket_path).expect("remove unix socket");
    fs::remove_dir_all(root).expect("remove fixture");
}
