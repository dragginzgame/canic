use super::*;

#[test]
fn human_errors_escape_terminal_controls_without_changing_json_evidence() {
    let error = Error::Invalid("bad\u{1b}[2J\rpath\u{202e}\ncompiler context".into());
    let text = error_text(&error, false);
    assert!(text.starts_with("Error: "));
    assert!(!text.contains(['\u{1b}', '\r', '\u{202e}']));
    assert!(text.contains("\n  compiler context"));
    let json: serde_json::Value = serde_json::from_str(&error_text(&error, true)).unwrap();
    assert_eq!(
        json,
        serde_json::json!({"error":"invalid", "message":error.to_string()})
    );
}
