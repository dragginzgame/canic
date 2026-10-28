use super::*;
use std::collections::BTreeMap;

#[test]
fn launcher_path_normalization_preserves_tool_search_inputs() {
    let root = crate::test_support::temp_dir("build-launcher-path");
    let home = root.join("codex");
    let helper = home.join("tmp/arg0-linux/codex-arg0fixture");
    fs::create_dir_all(&helper).unwrap();
    for name in [".lock", "apply_patch", "applypatch", "codex-linux-sandbox"] {
        fs::write(helper.join(name), b"fixture").unwrap();
    }
    let tools = root.join("tools");
    let missing = home.join("tmp/arg0/codex-arg0expired");
    let unknown = root.join("unknown-missing-tools");
    let selected = env::join_paths([&helper, &tools, &missing, &unknown]).unwrap();
    assert_eq!(
        build_path(&selected, Some(&home)),
        env::join_paths([&tools, &unknown]).unwrap()
    );
    assert_eq!(build_path(&selected, None), selected);
    fs::write(helper.join("rustc"), b"compiler").unwrap();
    assert_eq!(
        build_path(&selected, Some(&home)),
        env::join_paths([&helper, &tools, &unknown]).unwrap()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn command_and_fingerprint_share_the_captured_environment() {
    let expected: BTreeMap<_, _> = inputs().into_iter().collect();
    let command = command("unused");
    let actual: BTreeMap<_, _> = command
        .get_envs()
        .map(|(key, value)| (key.to_owned(), value.unwrap().to_owned()))
        .collect();
    assert_eq!(actual, expected);
    assert_eq!(
        expected.get(OsStr::new("SHLVL")),
        Some(&OsString::from("0"))
    );
    for key in WITHHELD_INPUTS {
        assert!(!expected.contains_key(OsStr::new(key)));
    }
}

#[cfg(unix)]
#[test]
fn child_keeps_explicit_overrides_and_removals() {
    let original = inputs();
    let mut command = command("/bin/sh");
    command
        .env("CANIC_ENVIRONMENT_PROBE", "selected")
        .env_remove("HOME")
        .args(["-c", "test \"$CANIC_ENVIRONMENT_PROBE\" = selected && test -z \"${HOME+x}\" && test -z \"${CODEX_SESSION_ID+x}\" && test -z \"${CODEX_THREAD_ID+x}\""]);
    assert!(command.status().unwrap().success());
    assert_eq!(inputs(), original);
}
