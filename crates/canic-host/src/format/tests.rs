use super::{compact_duration, wasm_size_label};

// Prefer the IC-installed wasm size while retaining gzip as optional context.
#[test]
fn formats_wasm_size_labels() {
    assert_eq!(
        wasm_size_label(Some(2 * 1024 * 1024), Some(512 * 1024)),
        "2.00 MiB (gz 512.00 KiB)"
    );
    assert_eq!(wasm_size_label(Some(2 * 1024 * 1024), None), "2.00 MiB");
    assert_eq!(
        wasm_size_label(None, Some(512 * 1024)),
        "n/a (gz 512.00 KiB)"
    );
    assert_eq!(wasm_size_label(None, None), "-");
}

// Keep human duration labels compact for CLI tables.
#[test]
fn formats_compact_durations() {
    assert_eq!(compact_duration(0), "0s");
    assert_eq!(compact_duration(45), "45s");
    assert_eq!(compact_duration(90), "1m 30s");
    assert_eq!(compact_duration(7_230), "2h");
    assert_eq!(compact_duration(9_000), "2h 30m");
    assert_eq!(compact_duration(97_200), "1d 3h");
    assert_eq!(compact_duration(1_555_200), "2w 4d");
}
