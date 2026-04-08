// Category C - Artifact / deployment audit (embedded config).
// This audit relies on embedded production config by design.

mod instruction_audit_support;
mod root_harness;

#[test]
#[ignore = "audit runner"]
fn generate_instruction_footprint_report() {
    instruction_audit_support::generate_instruction_footprint_report();
}
