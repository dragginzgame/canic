//! Module: saltz_preview::tests
//!
//! Responsibility: verify exact rendering, routing, provenance, and read-only HTTP behavior.
//! Does not own: mainnet deployment, Dashboard qualification, or destructive calibration.
//! Boundary: exercises pure request handling over the build-verified embedded waveform.

use crate::{
    http::{HttpRequest, response_for},
    render,
};

fn request(method: &str, url: &str) -> HttpRequest {
    HttpRequest {
        body: Vec::new(),
        headers: Vec::new(),
        method: method.to_string(),
        url: url.to_string(),
    }
}

#[test]
fn page_tells_the_inert_preview_story_without_rehosting_the_source_photo() {
    let response = response_for(request("GET", "/"), "aaaaa-aa");
    let body = String::from_utf8(response.body).expect("Saltz page is UTF-8");

    assert_eq!(response.status_code, 200);
    assert!(body.contains("<title>Das Domrestaurantwandkunstzyklusbrenngraphnachbildung</title>"));
    assert!(
        body.contains(
            "<h1>Das <span>Domrestaurantwandkunstzyklusbrenngraphnachbildung</span></h1>"
        )
    );
    assert!(body.contains("INERT CANISTER PREVIEW // STATIC MODEL"));
    assert!(body.contains("PREVIEW RESPONSE // SERVED"));
    assert!(body.contains("BURN CAPABILITY: NOT COMPILED"));
    assert!(body.contains("HTTP: RAW / UNCERTIFIED"));
    assert!(body.contains("Proposed Dashboard Total"));
    assert!(body.contains("STATIC MODEL // NOT LIVE OR QUALIFIED"));
    assert!(body.contains("100–150B/s"));
    assert!(body.contains("DATED GLOBAL SAMPLE // 31.7–49.9B/s // 2026-08-15/16"));
    assert!(body.contains("PROPOSED WAVEFORM FLOOR // 100B/s"));
    assert!(body.contains("HYPOTHETICAL ELAPSED TIME // NO START IS ARMED"));
    assert!(body.contains("Impossible in this Wasm"));
    assert!(body.contains("10,464.206 Tcycles"));
    assert!(body.contains("Made with <span class=\"heart\">❤️</span> by <a href=\"https://github.com/dragginzgame/canic\" rel=\"noreferrer\">Canic</a>"));
    assert!(body.contains(">0B</text>"));
    assert!(body.contains("<!-- stop being a paranoid dickhead -->"));
    assert!(body.contains("aaaaa-aa"));
    assert_eq!(render::WAVEFORM_SVG_POINTS.split_whitespace().count(), 860);
    assert!(!body.contains("SIMULATION // ONLINE"));
    assert!(!body.contains("Expected Dashboard Shape"));
    assert!(!body.contains("DATED ORDINARY IC PLANNING BAND"));
    let anonymous_body = body.to_ascii_lowercase();
    assert!(!anonymous_body.contains("saltz"));
    assert!(!anonymous_body.contains("neon"));
    assert!(!anonymous_body.contains("dezeen"));
    assert_eq!(body.matches("href=\"https://").count(), 1);
    assert!(!body.contains("<image"));
    assert!(!body.contains(".webp"));
    assert!(!body.contains("/assets/"));
    assert!(!body.contains("<script"));
}

#[test]
fn graph_is_code_native_and_contains_no_raster_image() {
    let response = response_for(request("GET", "/"), "aaaaa-aa");
    let body = String::from_utf8(response.body).expect("Saltz page is UTF-8");

    assert!(body.contains("<svg"));
    assert!(body.contains("This data-only graph contains no restaurant image"));
    assert!(!body.contains("<image"));
    assert!(!body.contains("data:image"));
}

#[test]
fn displayed_trace_preserves_the_source_rise_to_width_ratio() {
    let mut minimum_y = f64::MAX;
    let mut maximum_y = f64::MIN;
    for point in render::WAVEFORM_SVG_POINTS.split_whitespace() {
        let (_, y) = point.split_once(',').expect("SVG point has X and Y");
        let y = y.parse::<f64>().expect("SVG Y coordinate is numeric");
        minimum_y = minimum_y.min(y);
        maximum_y = maximum_y.max(y);
    }

    let displayed_ratio = (maximum_y - minimum_y) / 1_120.0;
    let source_ratio = 48.499 / 859.0;
    assert!((displayed_ratio - source_ratio).abs() < 0.000_002);
}

#[test]
fn provenance_json_identifies_the_inert_single_method_canister() {
    let response = response_for(request("GET", "/api/status.json"), "aaaaa-aa");
    let anonymous_body = std::str::from_utf8(&response.body).expect("status is UTF-8");
    let value: serde_json::Value =
        serde_json::from_slice(&response.body).expect("Saltz status is JSON");

    assert_eq!(response.status_code, 200);
    assert_eq!(value["mode"], "inert_preview");
    assert_eq!(
        value["response"]["certification"],
        "uncertified_query_response"
    );
    assert_eq!(value["response"]["raw_gateway_required"], true);
    assert_eq!(
        value["runtime"]["query_methods"],
        serde_json::json!(["http_request"])
    );
    assert_eq!(value["runtime"]["update_methods"], serde_json::json!([]));
    assert_eq!(value["runtime"]["live_metric_fetch"], false);
    assert_eq!(value["intentional_burn"]["capability_compiled"], false);
    assert_eq!(value["intentional_burn"]["active_run"], false);
    assert_eq!(value["intentional_burn"]["cycles_burned_by_this_wasm"], 0);
    assert_eq!(
        value["intentional_burn"]["ordinary_query_execution_consumes_cycles"],
        true
    );
    assert_eq!(value["waveform"]["status"], "proposed_unqualified");
    assert_eq!(value["waveform"]["point_count"], 860);
    assert_eq!(
        value["waveform"]["csv_sha256"],
        "11fd75eb8fd0fed4f075d324051cc880db50619837bfe6c889fe9d654647d911"
    );
    assert_eq!(value["dated_global_observation"]["point_count"], 865);
    assert_eq!(
        value["dated_global_observation"]["purpose"],
        "orientation_only"
    );
    assert_eq!(value["presentation"]["raster_images_served"], false);
    assert_eq!(value["presentation"]["image_pipeline_present"], false);
    let anonymous_body = anonymous_body.to_ascii_lowercase();
    assert!(!anonymous_body.contains("saltz"));
    assert!(!anonymous_body.contains("neon"));
    assert!(!anonymous_body.contains("dezeen"));
}

#[test]
fn csv_route_serves_the_digest_verified_reference_artifact() {
    let response = response_for(request("GET", "/waveform.csv"), "aaaaa-aa");
    let csv = String::from_utf8(response.body).expect("Saltz CSV is UTF-8");

    assert_eq!(response.status_code, 200);
    assert_eq!(csv.lines().count(), 861);
    assert!(csv.starts_with("index,bucket_start,"));
    assert!(csv.ends_with(",100000000000,100.000000000\n"));
    let anonymous_csv = csv.to_ascii_lowercase();
    assert!(!anonymous_csv.contains("saltz"));
    assert!(!anonymous_csv.contains("neon"));
    assert!(!anonymous_csv.contains("dezeen"));
}

#[test]
fn head_preserves_length_and_mutation_methods_fail_closed() {
    let get = response_for(request("GET", "/"), "aaaaa-aa");
    let head = response_for(request("HEAD", "/"), "aaaaa-aa");

    assert!(head.body.is_empty());
    assert_eq!(head.status_code, 200);
    assert_eq!(
        header(&head.headers, "Content-Length"),
        get.body.len().to_string()
    );
    assert_eq!(
        response_for(request("POST", "/"), "aaaaa-aa").status_code,
        405
    );
    assert_eq!(
        response_for(request("GET", "/missing"), "aaaaa-aa").status_code,
        404
    );
    let content_security_policy = header(&get.headers, "Content-Security-Policy");
    assert!(content_security_policy.contains("default-src 'none'"));
    assert!(!content_security_policy.contains("img-src"));
}

#[test]
fn rendered_identity_is_escaped_for_html_and_json() {
    let html = response_for(request("GET", "/"), "<bad&identity>");
    let html = String::from_utf8(html.body).expect("Saltz page is UTF-8");
    let json = response_for(request("GET", "/api/status.json"), "bad\"identity\\value");
    let json: serde_json::Value =
        serde_json::from_slice(&json.body).expect("escaped Saltz status is JSON");

    assert!(html.contains("&lt;bad&amp;identity&gt;"));
    assert!(!html.contains("<bad&identity>"));
    assert_eq!(json["canister_id"], "bad\"identity\\value");
}

fn header(headers: &[(String, String)], name: &str) -> String {
    headers
        .iter()
        .find(|(candidate, _)| candidate == name)
        .map(|(_, value)| value.clone())
        .expect("response header is present")
}
