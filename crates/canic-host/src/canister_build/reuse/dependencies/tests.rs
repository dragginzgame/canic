use super::*;

#[test]
fn generated_ownership_uses_resolved_paths_and_preserves_external_inputs() {
    let root = crate::test_support::temp_dir("reuse-output-ownership");
    let output = root.join("target");
    fs::create_dir_all(&output).unwrap();
    fs::create_dir_all(root.join("alias")).unwrap();
    let source = root.join("authored.json");
    let generated = output.join("translations.json");
    fs::write(&source, "authored").unwrap();
    fs::write(&generated, "generated").unwrap();
    let selected = resolve_existing_path(&root.join("alias/../target")).unwrap();
    assert_eq!(selected, output.canonicalize().unwrap());
    let mut files = BTreeMap::new();
    append_input(
        &root.join("alias/../target/translations.json"),
        std::slice::from_ref(&selected),
        &mut files,
    )
    .unwrap();
    assert!(
        files.is_empty(),
        "resolved generated include is not a source"
    );

    append_input(
        &output.join("../authored.json"),
        std::slice::from_ref(&selected),
        &mut files,
    )
    .unwrap();
    let source_key = source
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let before = files[&source_key].clone();
    fs::write(&source, "edited").unwrap();
    let mut fresh = BTreeMap::new();
    append_input(&source, std::slice::from_ref(&selected), &mut fresh).unwrap();
    assert_ne!(before, fresh[&source_key]);

    let missing = output.join("../not-yet-authored.json");
    append_input(&missing, std::slice::from_ref(&selected), &mut files).unwrap();
    assert_eq!(files[missing.to_str().unwrap()], "absent");
    let foreign = root.join("foreign/target/translations.json");
    fs::create_dir_all(foreign.parent().unwrap()).unwrap();
    fs::write(&foreign, "foreign").unwrap();
    append_input(&foreign, std::slice::from_ref(&selected), &mut files).unwrap();
    assert!(files.contains_key(foreign.canonicalize().unwrap().to_str().unwrap()));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cargo_paths_preserve_escaped_names_and_continuations() {
    assert_eq!(
        dependency_paths("/target/role.wasm: /source/lib.rs /outside/with\\ space.txt \\\n/outside/$$value\\#1.txt\n").unwrap(),
        ["/source/lib.rs", "/outside/with space.txt", "/outside/$value#1.txt"]
    );
    assert!(dependency_paths("/target/role.wasm: /outside/$(UNKNOWN)").is_err());
    assert!(dependency_paths("missing target separator").is_err());
}
