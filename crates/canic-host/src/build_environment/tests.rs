use super::*;
use std::collections::BTreeMap;

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
