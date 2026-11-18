use super::*;

pub(crate) const UNBOUND_FLEET: &str = r#"[apps.app.canisters.root]
role = "root"

[apps.app.fleets.mainnet]
network = { kind = "Mainnet" }
journal = "must-not-exist"
canisters = {}
"#;

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

pub(crate) fn directory(
    parent: &std::path::Path,
    prefix: &str,
) -> std::io::Result<std::path::PathBuf> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    for _ in 0..32 {
        let path = parent.join(format!(
            "{prefix}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match std::fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    Err(std::io::ErrorKind::AlreadyExists.into())
}
