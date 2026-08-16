//! Module: tests
//!
//! Responsibility: prove exact source identity, extraction and rational reference scheduling.
//! Does not own: Dashboard observations or Burner behavior.
//! Boundary: exercises the host compiler without any external effect.

use crate::{
    CompileError, InputError, SELECTED_SOURCE_SHA256, bucket_boundary_ns,
    compile_selected_reference, integrate_controlled_burn, render_csv, render_preview_svg,
    schedule::RUN_DURATION_NS,
};
use std::path::Path;

const SELECTED_SOURCE: &[u8] =
    include_bytes!("../../../../../docs/design/ideas/saltz/saltz_reference_dezeen_860x573.jpg");
const CHECKED_CSV: &str =
    include_str!("../../../../../docs/design/ideas/saltz/saltz_24h_waveform_floor_100B_860.csv");
const CHECKED_PREVIEW: &str =
    include_str!("../../../../../docs/design/ideas/saltz/saltz_reference_overlay.svg");

#[test]
fn selected_reference_compiles_without_an_external_effect() {
    let compilation = compile_selected_reference(SELECTED_SOURCE).expect("selected image compiles");

    assert_eq!(compilation.source_sha256, SELECTED_SOURCE_SHA256);
    assert_eq!(compilation.trace.points.len(), 860);
    assert_eq!(compilation.buckets.len(), 860);
    assert_eq!(compilation.buckets[0].bucket_start_offset_ns, 0);
    assert_eq!(
        compilation
            .buckets
            .last()
            .expect("last bucket")
            .bucket_start_offset_ns
            + compilation
                .buckets
                .last()
                .expect("last bucket")
                .bucket_duration_ns,
        RUN_DURATION_NS
    );
    assert!(compilation.trace.highest_y_millipx < compilation.trace.lowest_y_millipx);
    assert!(compilation.zero_background_cycles > 0);
}

#[test]
fn source_identity_fails_before_jpeg_decoding() {
    let mut changed = SELECTED_SOURCE.to_vec();
    changed[0] ^= 1;

    assert!(matches!(
        compile_selected_reference(&changed),
        Err(CompileError::Input(InputError::SourceIdentity { .. }))
    ));
}

#[test]
fn rational_boundaries_cover_exactly_twenty_four_hours() {
    let mut previous = 0;
    for index in 0..=860 {
        let boundary = bucket_boundary_ns(index, 860, RUN_DURATION_NS).expect("valid boundary");
        assert!(boundary >= previous);
        previous = boundary;
    }
    assert_eq!(previous, RUN_DURATION_NS);
}

#[test]
fn renderers_share_the_compiled_trace() {
    let compilation = compile_selected_reference(SELECTED_SOURCE).expect("selected image compiles");
    let csv = render_csv(&compilation);
    let preview = render_preview_svg(
        &compilation,
        Path::new("saltz_reference_dezeen_860x573.jpg"),
    );

    assert_eq!(csv, CHECKED_CSV);
    assert_eq!(preview, CHECKED_PREVIEW);
    assert_eq!(csv.lines().count(), 861);
    assert!(preview.contains(&compilation.trace.points_sha256));
    assert!(preview.contains("EXPECTED DASHBOARD SHAPE"));
    assert_eq!(
        integrate_controlled_burn(&compilation.buckets, 0).expect("bounded integration"),
        compilation.zero_background_cycles
    );
}
