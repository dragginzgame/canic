use super::super::inspect as deploy_inspect;
use super::*;

#[test]
fn deploy_inspect_command_dispatches_raw_artifact_leaves() {
    for leaf in ["plan", "inventory", "diff", "report", "resume-report"] {
        let parsed = parse_subcommand(
            deploy_command(),
            [
                OsString::from("inspect"),
                OsString::from(leaf),
                OsString::from("demo"),
            ],
        )
        .expect("parse deploy inspect command")
        .expect("deploy inspect command");
        assert_eq!(parsed.0, "inspect");

        let nested = parse_subcommand(deploy_inspect::command(), parsed.1)
            .expect("parse deploy inspect leaf")
            .expect("deploy inspect leaf command");
        assert_eq!(nested.0, leaf);
        assert_eq!(nested.1, vec![OsString::from("demo")]);
    }

    let parsed = parse_subcommand(
        deploy_command(),
        [
            OsString::from("inspect"),
            OsString::from("compare"),
            OsString::from("--left"),
            OsString::from("staging-check.json"),
            OsString::from("--right"),
            OsString::from("prod-check.json"),
        ],
    )
    .expect("parse deploy inspect compare command")
    .expect("deploy inspect command");
    assert_eq!(parsed.0, "inspect");

    let nested = parse_subcommand(deploy_inspect::command(), parsed.1)
        .expect("parse deploy inspect compare leaf")
        .expect("deploy inspect compare leaf command");
    assert_eq!(nested.0, "compare");
    assert_eq!(
        nested.1,
        vec![
            OsString::from("--left"),
            OsString::from("staging-check.json"),
            OsString::from("--right"),
            OsString::from("prod-check.json")
        ]
    );

    let parsed = parse_subcommand(
        deploy_command(),
        [
            OsString::from("inspect"),
            OsString::from("root"),
            OsString::from("--request"),
            OsString::from("root-verification.json"),
        ],
    )
    .expect("parse deploy inspect root command")
    .expect("deploy inspect command");
    assert_eq!(parsed.0, "inspect");

    let nested = parse_subcommand(deploy_inspect::command(), parsed.1)
        .expect("parse deploy inspect root leaf")
        .expect("deploy inspect root leaf command");
    assert_eq!(nested.0, "root");
    assert_eq!(
        nested.1,
        vec![
            OsString::from("--request"),
            OsString::from("root-verification.json")
        ]
    );

    let parsed = parse_subcommand(
        deploy_command(),
        [
            OsString::from("inspect"),
            OsString::from("catalog"),
            OsString::from("list"),
        ],
    )
    .expect("parse deploy inspect catalog command")
    .expect("deploy inspect command");
    assert_eq!(parsed.0, "inspect");

    let nested = parse_subcommand(deploy_inspect::command(), parsed.1)
        .expect("parse deploy inspect catalog leaf")
        .expect("deploy inspect catalog leaf command");
    assert_eq!(nested.0, "catalog");
    assert_eq!(nested.1, vec![OsString::from("list")]);
}

#[test]
fn deploy_inspect_help_uses_canonical_paths() {
    let help = deploy_inspect::usage();

    assert!(help.contains("Inspect raw deployment truth artifacts"));
    assert!(help.contains("canic deploy inspect plan demo"));
    assert!(help.contains(
        "canic deploy inspect compare --left staging-check.json --right prod-check.json"
    ));
    assert!(help.contains("canic deploy inspect catalog list"));
    assert!(help.contains("canic deploy inspect root --request root-verification.json"));
    assert!(help.contains("canic deploy inspect resume-report --receipt receipt.json demo"));
    assert!(help.contains("Use `canic inspect` for live runtime-observed"));
}
