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
    let catalog = diagnostic_catalog().expect("embedded diagnostic catalogue");
    let current = render_lookup(catalog.lookup(DiagnosticCode::from_raw(1)));
    assert!(current.contains("code: E1\nknown: true\nstatus: current"));
    assert!(current.contains("label: ACCESS_DEPENDENCY_UNAVAILABLE"));
    assert!(current.contains("disposition: retry_after_state_change"));

    assert_eq!(
        render_lookup(catalog.lookup(DiagnosticCode::from_raw(65_000))),
        "code: E65000\nknown: false\nstatus: unknown"
    );
}

#[test]
fn help_is_concise_and_uses_both_supported_input_forms() {
    let text = usage();
    assert!(text.contains("Usage: diagnostic <code>"));
    assert!(text.contains("canic diagnostic E123"));
    assert!(text.contains("canic diagnostic 123"));
}
