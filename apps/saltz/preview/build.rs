//! Module: saltz_preview build script
//!
//! Responsibility: verify the pinned Saltz CSV and compile its exact points into SVG data.
//! Does not own: browser presentation, canister routing, run authorization, or cycle burning.
//! Boundary: a digest or structural mismatch fails the canister build before Wasm exists.

use std::{env, error::Error, fmt::Write, fs, io, path::PathBuf};

use sha2::{Digest, Sha256};

const EXPECTED_CSV_SHA256: &str =
    "11fd75eb8fd0fed4f075d324051cc880db50619837bfe6c889fe9d654647d911";
const EXPECTED_DURATION_NS: u64 = 86_400_000_000_000;
const EXPECTED_MAX_HEIGHT_PPM: u64 = 1_000_000;
const EXPECTED_MAX_RATE: u64 = 150_000_000_000;
const EXPECTED_MIN_HEIGHT_PPM: u64 = 0;
const EXPECTED_MIN_RATE: u64 = 100_000_000_000;
const EXPECTED_POINT_COUNT: usize = 860;
const GRAPH_MAX_RATE: u64 = 150_000_000_000;
const GRAPH_MIN_RATE: u64 = 0;
const GRAPH_X_MAX_MILLI: u128 = 1_200_000;
const GRAPH_X_MIN_MILLI: u128 = 80_000;
const GRAPH_Y_BOTTOM_MILLI: u128 = 249_705;
const GRAPH_Y_RANGE_MILLI: u128 = 189_705;

fn main() -> Result<(), Box<dyn Error>> {
    let manifest_dir = required_path("CARGO_MANIFEST_DIR")?;
    let csv_path =
        manifest_dir.join("../../../docs/design/ideas/saltz/saltz_24h_waveform_floor_100B_860.csv");
    println!("cargo:rerun-if-changed={}", csv_path.display());

    let bytes = fs::read(&csv_path)?;
    let actual_digest = sha256_hex(&bytes);
    require(
        actual_digest == EXPECTED_CSV_SHA256,
        format!("Saltz CSV digest {actual_digest} does not match {EXPECTED_CSV_SHA256}"),
    )?;

    let csv = std::str::from_utf8(&bytes)?;
    let compiled = compile_points(csv)?;
    validate_compilation(&compiled)?;

    let generated = format!(
        "const CSV_SHA256: &str = \"{EXPECTED_CSV_SHA256}\";\n\
         const RUN_DURATION_NS: u64 = 86_400_000_000_000;\n\
         const WAVEFORM_MAX_RATE: u64 = 150_000_000_000;\n\
         const WAVEFORM_MIN_RATE: u64 = 100_000_000_000;\n\
         const WAVEFORM_POINT_COUNT: usize = {};\n\
         pub const WAVEFORM_SVG_POINTS: &str = \"{}\";\n",
        compiled.point_count, compiled.svg_points,
    );
    fs::write(required_path("OUT_DIR")?.join("waveform.rs"), generated)?;
    Ok(())
}

struct CompiledWaveform {
    duration_ns: u64,
    max_height_ppm: u64,
    max_rate: u64,
    min_height_ppm: u64,
    min_rate: u64,
    point_count: usize,
    svg_points: String,
}

fn compile_points(csv: &str) -> Result<CompiledWaveform, Box<dyn Error>> {
    let mut lines = csv.lines();
    let header = lines
        .next()
        .ok_or_else(|| io::Error::other("Saltz CSV is empty"))?;
    require(
        header
            == "index,bucket_start,bucket_start_offset_ns,bucket_duration_ns,source_x_px,source_y_px,height_px,height_ppm,target_visible_cycles_per_second,target_visible_Bcycles_per_second",
        "Saltz CSV header changed",
    )?;

    let mut expected_offset = 0_u64;
    let mut max_height_ppm = 0_u64;
    let mut max_rate = 0_u64;
    let mut min_height_ppm = u64::MAX;
    let mut min_rate = u64::MAX;
    let mut point_count = 0_usize;
    let mut svg_points = String::with_capacity(EXPECTED_POINT_COUNT * 18);

    for line in lines {
        let mut columns = line.split(',');
        let index = parse_u64(&mut columns, "index")?;
        let _bucket_start = required_column(&mut columns, "bucket_start")?;
        let offset = parse_u64(&mut columns, "bucket_start_offset_ns")?;
        let duration = parse_u64(&mut columns, "bucket_duration_ns")?;
        let source_x = parse_u64(&mut columns, "source_x_px")?;
        let _source_y = required_column(&mut columns, "source_y_px")?;
        let _height_px = required_column(&mut columns, "height_px")?;
        let height_ppm = parse_u64(&mut columns, "height_ppm")?;
        let target_rate = parse_u64(&mut columns, "target_visible_cycles_per_second")?;
        let _target_rate_billions =
            required_column(&mut columns, "target_visible_Bcycles_per_second")?;
        require(columns.next().is_none(), "Saltz CSV row has extra columns")?;

        let expected_index = u64::try_from(point_count)?;
        require(
            index == expected_index,
            "Saltz CSV indexes are not contiguous",
        )?;
        require(
            source_x == index,
            "Saltz source X no longer matches its index",
        )?;
        require(
            offset == expected_offset,
            "Saltz bucket offsets are not contiguous",
        )?;
        require(
            (GRAPH_MIN_RATE..=GRAPH_MAX_RATE).contains(&target_rate),
            "Saltz target rate falls outside the preview graph",
        )?;

        expected_offset = expected_offset
            .checked_add(duration)
            .ok_or_else(|| io::Error::other("Saltz duration overflow"))?;
        max_height_ppm = max_height_ppm.max(height_ppm);
        max_rate = max_rate.max(target_rate);
        min_height_ppm = min_height_ppm.min(height_ppm);
        min_rate = min_rate.min(target_rate);

        let x_milli = GRAPH_X_MIN_MILLI
            + u128::from(index) * (GRAPH_X_MAX_MILLI - GRAPH_X_MIN_MILLI)
                / u128::try_from(EXPECTED_POINT_COUNT - 1)?;
        let visible_delta = u128::from(target_rate - GRAPH_MIN_RATE);
        let y_reduction = visible_delta
            .checked_mul(GRAPH_Y_RANGE_MILLI)
            .ok_or_else(|| io::Error::other("Saltz SVG Y coordinate overflow"))?
            / u128::from(GRAPH_MAX_RATE - GRAPH_MIN_RATE);
        let y_milli = GRAPH_Y_BOTTOM_MILLI
            .checked_sub(y_reduction)
            .ok_or_else(|| io::Error::other("Saltz SVG Y coordinate underflow"))?;

        if !svg_points.is_empty() {
            svg_points.push(' ');
        }
        write!(
            svg_points,
            "{},{}",
            fixed_milli(x_milli),
            fixed_milli(y_milli)
        )?;
        point_count += 1;
    }

    Ok(CompiledWaveform {
        duration_ns: expected_offset,
        max_height_ppm,
        max_rate,
        min_height_ppm,
        min_rate,
        point_count,
        svg_points,
    })
}

fn validate_compilation(compiled: &CompiledWaveform) -> Result<(), Box<dyn Error>> {
    require(
        compiled.point_count == EXPECTED_POINT_COUNT,
        "Saltz point count changed",
    )?;
    require(
        compiled.duration_ns == EXPECTED_DURATION_NS,
        "Saltz run duration changed",
    )?;
    require(
        compiled.min_height_ppm == EXPECTED_MIN_HEIGHT_PPM
            && compiled.max_height_ppm == EXPECTED_MAX_HEIGHT_PPM,
        "Saltz normalized height range changed",
    )?;
    require(
        compiled.min_rate == EXPECTED_MIN_RATE && compiled.max_rate == EXPECTED_MAX_RATE,
        "Saltz visible-rate range changed",
    )?;
    Ok(())
}

fn fixed_milli(value: u128) -> String {
    format!("{}.{:03}", value / 1_000, value % 1_000)
}

fn parse_u64<'a>(
    columns: &mut impl Iterator<Item = &'a str>,
    name: &str,
) -> Result<u64, Box<dyn Error>> {
    Ok(required_column(columns, name)?.parse()?)
}

fn required_column<'a>(
    columns: &mut impl Iterator<Item = &'a str>,
    name: &str,
) -> Result<&'a str, Box<dyn Error>> {
    columns
        .next()
        .ok_or_else(|| io::Error::other(format!("Saltz CSV row is missing {name}")))
        .map_err(Into::into)
}

fn required_path(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    env::var_os(name)
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::other(format!("{name} is not set")))
        .map_err(Into::into)
}

fn require(condition: bool, message: impl Into<String>) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(io::Error::other(message.into()).into())
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        write!(encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}
