use std::io::Write;

// Emit one short progress marker for long grouped PocketIC scenario tests.
pub fn test_progress(test_name: &str, phase: &str) {
    eprintln!("[pic_role_attestation] {test_name}: {phase}");
    let _ = std::io::stderr().flush();
}
