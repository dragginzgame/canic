//! Recorded evidence distinguishes unchanged inputs, fresh qualification and corrupt artifacts.

use super::*;

#[test]
fn relocated_fixture_preserves_internal_and_external_dependency_owners() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let fixture = source::FixtureSource {
        root: Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/embedded-root")
            .join(format!("dependency-paths-{}-{nonce}", std::process::id())),
    };
    let workspace = fixture.root.join("workspace");
    let external = fixture.root.join("external library");
    let snapshot = fixture.root.join("snapshot");
    fs::create_dir_all(workspace.join("crates/internal")).unwrap();
    fs::create_dir_all(&external).unwrap();
    fs::create_dir_all(&snapshot).unwrap();
    let workspace = workspace.canonicalize().unwrap();
    let external = external.canonicalize().unwrap();
    let manifest = r#"
        [workspace.dependencies]
        internal = { version = "1.2.3", path = "crates/internal" }
        external = { version = "=0.1.0", path = "../external library", features = ["sample"] }
        registry = "2.3.4"
    "#;
    fs::write(workspace.join("Cargo.toml"), manifest).unwrap();
    fs::write(snapshot.join("Cargo.toml"), manifest).unwrap();
    source::bind_dependency_paths(&snapshot, &workspace).unwrap();
    let relocated: toml::Value =
        toml::from_str(&fs::read_to_string(snapshot.join("Cargo.toml")).unwrap()).unwrap();
    let dependencies = &relocated["workspace"]["dependencies"];
    assert_eq!(
        dependencies["internal"]["path"].as_str(),
        Some("crates/internal")
    );
    assert_eq!(dependencies["external"]["path"].as_str(), external.to_str());
    assert_eq!(dependencies["external"]["version"].as_str(), Some("=0.1.0"));
    assert_eq!(
        dependencies["external"]["features"][0].as_str(),
        Some("sample")
    );
    assert_eq!(dependencies["registry"].as_str(), Some("2.3.4"));
    assert_eq!(
        fs::read_to_string(workspace.join("Cargo.toml")).unwrap(),
        manifest
    );

    fs::remove_dir(&external).unwrap();
    assert_eq!(
        source::bind_dependency_paths(&snapshot, &workspace)
            .unwrap_err()
            .kind(),
        io::ErrorKind::NotFound
    );
}

#[test]
fn fixture_version_normalization_preserves_dependency_requirements_and_profiles() {
    let manifest = r#"
        [workspace.package]
        version = "0.110.50"
        [workspace.dependencies]
        canic = { version = "0.110.50", path = "crates/canic", default-features = false }
        external = "0.110.50"
        independently_versioned = { version = "1.2.3", path = "other" }
        [profile.fast]
        inherits = "release"
        lto = "thin"
    "#;
    let normalized = source::normalize_manifest(manifest).unwrap();
    let document: toml::Value = toml::from_str(&normalized).unwrap();
    assert_eq!(
        document["workspace"]["package"]["version"].as_str(),
        Some("0.0.0")
    );
    assert_eq!(
        document["workspace"]["dependencies"]["canic"]["version"].as_str(),
        Some("0.0.0")
    );
    assert_eq!(
        document["workspace"]["dependencies"]["external"].as_str(),
        Some("0.110.50")
    );
    assert_eq!(
        document["workspace"]["dependencies"]["independently_versioned"]["version"].as_str(),
        Some("1.2.3")
    );
    assert_eq!(document["profile"]["fast"]["lto"].as_str(), Some("thin"));
    assert_eq!(normalized, source::normalize_manifest(&normalized).unwrap());
    assert_eq!(
        normalized,
        source::normalize_manifest(
            &manifest.replace("version = \"0.110.50\"", "version = \"0.110.51\"")
        )
        .unwrap()
    );
    assert_ne!(
        normalized,
        source::normalize_manifest(&manifest.replace("lto = \"thin\"", "lto = false")).unwrap()
    );
    assert_ne!(
        normalized,
        source::normalize_manifest(
            &manifest.replace("external = \"0.110.50\"", "external = \"0.110.51\"")
        )
        .unwrap()
    );
}

#[test]
#[ignore = "explicit real-Wasm reproduction across paths and a release version transaction"]
fn embedded_peer_reproduces_across_paths_and_release_versions() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let manifest_before = fs::read(workspace.join("Cargo.toml")).unwrap();
    let lock_before = fs::read(workspace.join("Cargo.lock")).unwrap();
    let first = source::FixtureSource::prepare(&workspace).unwrap();
    let second = source::FixtureSource::prepare(&workspace).unwrap();
    source::output(Command::new("cargo").current_dir(&second.root).args([
        "set-version",
        "--workspace",
        "--offline",
        "0.110.51",
    ]))
    .unwrap();
    source::output(Command::new("cargo").current_dir(&second.root).args([
        "update",
        "--workspace",
        "--offline",
    ]))
    .unwrap();
    source::normalize(&second.root).unwrap();
    let first_spec = build_spec(&workspace, &first.root).unwrap();
    let second_spec = build_spec(&workspace, &second.root).unwrap();
    let first_inputs = resolve_cargo_build_inputs(&first_spec).unwrap();
    let second_inputs = resolve_cargo_build_inputs(&second_spec).unwrap();
    assert_eq!(first_inputs.input_digest(), second_inputs.input_digest());
    let (first_bytes, _) = build(&workspace, &first_spec).unwrap();
    let (second_bytes, _) = build(&workspace, &second_spec).unwrap();
    assert_eq!(sha256_hex(&first_bytes), sha256_hex(&second_bytes));
    assert_eq!(first_bytes, fs::read(workspace.join(ARTIFACT)).unwrap());
    assert!(
        !first_bytes
            .windows(workspace.as_os_str().len())
            .any(|window| window == workspace.as_os_str().as_encoded_bytes())
    );
    let producer = second
        .root
        .join("canisters/test/sharding_root_stub/src/lib.rs");
    let contents = fs::read_to_string(&producer).unwrap();
    fs::write(
        &producer,
        format!("{contents}\ncompile_error!(\"changed fixture producer\");\n"),
    )
    .unwrap();
    assert_ne!(
        resolve_cargo_build_inputs(&second_spec)
            .unwrap()
            .input_digest(),
        first_inputs.input_digest()
    );
    assert!(build(&workspace, &second_spec).is_err());
    assert_eq!(
        fs::read(workspace.join("Cargo.toml")).unwrap(),
        manifest_before
    );
    assert_eq!(fs::read(workspace.join("Cargo.lock")).unwrap(), lock_before);
}

#[test]
fn changed_sources_need_qualification_but_wrong_artifacts_never_reuse_evidence() {
    let bytes = b"fixture artifact";
    let evidence = FixtureEvidence {
        schema_version: 1,
        package: PACKAGE.into(),
        target: TARGET.into(),
        profile: PROFILE.into(),
        fixture_version: source::FIXTURE_VERSION.into(),
        source_input_digest: "a1".repeat(32),
        build_fingerprint: "b2".repeat(32),
        lock_sha256: "c3".repeat(32),
        producer_lock_sha256: "e5".repeat(32),
        artifact_sha256: sha256_hex(bytes),
        cargo: "cargo identity".into(),
        rustc: "rustc identity".into(),
    };
    assert!(recorded_match(&evidence, &"a1".repeat(32), bytes).unwrap());
    assert!(!recorded_match(&evidence, &"d4".repeat(32), bytes).unwrap());
    assert_eq!(
        recorded_match(&evidence, &"a1".repeat(32), b"changed artifact")
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
    let changed = FixtureEvidence {
        target: "another-target".into(),
        ..evidence
    };
    assert_eq!(
        recorded_match(&changed, &"a1".repeat(32), bytes)
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
}
