use super::*;
use canic_host::diagnostics::diagnostic_catalog;

#[test]
fn parser_accepts_only_lossless_decimal_and_uppercase_prefixed_codes() {
    assert_eq!(parse_code("123").expect("decimal").raw(), 123);
    assert_eq!(parse_code("E123").expect("prefixed decimal").raw(), 123);
    assert_eq!(parse_code("E0").expect("lossless zero").raw(), 0);

    for invalid in ["", "E", "e123", "+123", "-1", " 123", "123 ", "65536"] {
        assert!(
            parse_code(invalid).is_err(),
            "accepted invalid code: {invalid}"
        );
    }
}

#[test]
fn renderer_distinguishes_current_and_unknown_codes() {
    let catalog = diagnostic_catalog();
    let current = render_lookup(catalog.lookup(DiagnosticCode::from_raw(1)));
    assert!(current.contains("code: E1\nknown: true\nstatus: current"));
    assert!(current.contains("name: ACCESS_UNAVAILABLE"));
    assert!(current.contains("origin: access"));
    assert!(current.contains("summary: Access is unavailable."));

    assert_eq!(
        render_lookup(catalog.lookup(DiagnosticCode::from_raw(65_000))),
        "code: E65000\nknown: false\nstatus: unknown"
    );
}

#[test]
fn help_is_concise_and_uses_both_supported_input_forms() {
    let text = usage();
    assert!(text.contains("Usage: canic diagnostic"));
    assert!(text.contains("canic diagnostic E123"));
    assert!(text.contains("canic diagnostic 123"));
}

#[test]
fn build_lock_parser_requires_an_exact_path_and_preserves_code_lookup() {
    let parse =
        |args: &[&str]| parse_matches(diagnostic_command(), args.iter().map(OsString::from));
    let matches = parse(&["build-lock", "--lock", "/tmp/work space/lock", "--json"]).unwrap();
    let options = matches.subcommand_matches("build-lock").unwrap();
    assert_eq!(
        options.get_one::<PathBuf>("lock"),
        Some(&PathBuf::from("/tmp/work space/lock"))
    );
    assert!(options.get_flag("json"));
    assert_eq!(
        parse(&["build-lock"]).unwrap_err().kind(),
        clap::error::ErrorKind::MissingRequiredArgument
    );
    assert!(parse(&["123", "build-lock", "--lock", "/tmp/lock"]).is_err());
    assert_eq!(
        required_string(&parse(&["E123"]).unwrap(), CODE_ARGUMENT),
        "E123"
    );
    assert_eq!(
        parse(&["build-lock", "--help"]).unwrap_err().kind(),
        clap::error::ErrorKind::DisplayHelp
    );
}
