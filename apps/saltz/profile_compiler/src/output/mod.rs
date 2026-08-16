//! Module: output
//!
//! Responsibility: render deterministic review CSV and source/target SVG evidence.
//! Does not own: image extraction, profile authority, or executable plan serialization.
//! Boundary: consumes one validated compilation and performs no external observation.

use crate::{Compilation, schedule};
use std::{fmt::Write, path::Path};

pub fn render_csv(compilation: &Compilation) -> String {
    let mut csv = String::from(
        "index,bucket_start,bucket_start_offset_ns,bucket_duration_ns,source_x_px,source_y_px,height_px,height_ppm,target_visible_cycles_per_second,target_visible_Bcycles_per_second\n",
    );
    for bucket in &compilation.buckets {
        let source_y = decimal_millipixels(bucket.source_y_millipx);
        let height_millipx = compilation
            .trace
            .lowest_y_millipx
            .saturating_sub(bucket.source_y_millipx);
        let height = decimal_millipixels(height_millipx);
        let target_bcycles = decimal_bcycles(bucket.target_visible_cycles_per_second);
        writeln!(
            &mut csv,
            "{},{},{},{},{},{},{},{},{},{}",
            bucket.index,
            clock_offset(bucket.bucket_start_offset_ns),
            bucket.bucket_start_offset_ns,
            bucket.bucket_duration_ns,
            bucket.source_x_px,
            source_y,
            height,
            bucket.height_ppm,
            bucket.target_visible_cycles_per_second,
            target_bcycles
        )
        .expect("writing to String cannot fail");
    }
    csv
}

pub fn render_preview_svg(compilation: &Compilation, source_path: &Path) -> String {
    let image_href = xml_escape(
        source_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("saltz_reference_dezeen_860x573.jpg"),
    );
    let source_points = compilation
        .trace
        .points
        .iter()
        .map(|point| {
            format!(
                "{},{}",
                point.x_px,
                decimal_millipixels(point.source_y_millipx)
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let target_points = compilation
        .buckets
        .iter()
        .map(|bucket| {
            let y = chart_y_millipx(bucket.target_visible_cycles_per_second);
            format!("{},{}", bucket.source_x_px, decimal_millipixels(y))
        })
        .collect::<Vec<_>>()
        .join(" ");
    let band_top = chart_y_millipx(schedule::TARGET_FLOOR_CYCLES_PER_SECOND);
    let band_bottom = chart_y_millipx(30_000_000_000);
    let band_height = band_bottom - band_top;
    let band_top = decimal_millipixels(band_top);
    let band_height = decimal_millipixels(band_height);

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="860" height="860" viewBox="0 0 860 860">
  <rect width="860" height="860" fill="#120507"/>
  <image href="{image_href}" x="0" y="0" width="860" height="573"/>
  <polyline points="{source_points}" fill="none" stroke="#5fffe0" stroke-width="2" stroke-linejoin="round"/>
  <rect x="0" y="573" width="860" height="287" fill="#16080b"/>
  <text x="20" y="610" fill="#f8eee8" font-family="monospace" font-size="16">SELECTED SOURCE + EXTRACTED CENTRELINE</text>
  <text x="20" y="636" fill="#f8eee8" font-family="monospace" font-size="16">EXPECTED DASHBOARD SHAPE · 100.000–150.000 Bcycles/s</text>
  <rect x="0" y="{band_top}" width="860" height="{band_height}" fill="#5f1822" opacity="0.45"/>
  <line x1="0" y1="{band_top}" x2="860" y2="{band_top}" stroke="#c15a64" stroke-dasharray="5 5"/>
  <polyline points="{target_points}" fill="none" stroke="#ffdf72" stroke-width="2" stroke-linejoin="round"/>
  <text x="20" y="845" fill="#c8adb0" font-family="monospace" font-size="12">SHA-256 {source_sha} · {algorithm} · trace {trace_sha}</text>
</svg>
"##,
        source_sha = compilation.source_sha256,
        algorithm = compilation.trace.algorithm_id,
        trace_sha = compilation.trace.points_sha256,
    )
}

fn chart_y_millipx(rate: u128) -> u32 {
    const CHART_BOTTOM_MILLIPX: u128 = 820_000;
    const CHART_HEIGHT_MILLIPX: u128 = 160_000;
    const CHART_MIN: u128 = 25_000_000_000;
    const CHART_RANGE: u128 = 165_000_000_000;

    let bounded = rate.clamp(CHART_MIN, CHART_MIN + CHART_RANGE) - CHART_MIN;
    let vertical_offset = bounded * CHART_HEIGHT_MILLIPX / CHART_RANGE;
    u32::try_from(CHART_BOTTOM_MILLIPX - vertical_offset)
        .expect("bounded chart coordinate fits u32")
}

fn clock_offset(offset_ns: u64) -> String {
    let total_millis = offset_ns / 1_000_000;
    let millis = total_millis % 1_000;
    let total_seconds = total_millis / 1_000;
    let seconds = total_seconds % 60;
    let total_minutes = total_seconds / 60;
    let minutes = total_minutes % 60;
    let hours = total_minutes / 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}.{millis:03}")
}

fn decimal_bcycles(rate: u128) -> String {
    format!("{}.{:09}", rate / 1_000_000_000, rate % 1_000_000_000)
}

fn decimal_millipixels(value: u32) -> String {
    format!("{}.{:03}", value / 1_000, value % 1_000)
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
