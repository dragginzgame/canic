#![expect(clippy::unused_async)]

use candid::{CandidType, Deserialize};
use canic::prelude::*;
use ic_cdk::api::canister_self;

const IMAGE: &[u8] = include_bytes!("../assets/wenzelroll.png");
const IMAGE_PATH: &str = "/wenzelroll.png";
const PREVIEW_CRAWLER_MARKERS: &[&str] = &[
    "discordbot",
    "facebookexternalhit",
    "facebot",
    "linkedinbot",
    "microsoftpreview",
    "skypeuripreview",
    "slackbot",
    "telegrambot",
    "twitterbot",
    "whatsapp",
];
const PREVIEW_HTML_TEMPLATE: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>__PREVIEW_TITLE__</title>
__PREVIEW_METADATA__
</head>
<body></body>
</html>"#;
const PREVIEW_DESCRIPTION: &str = "Component topology and runtime status endpoint.";
const PREVIEW_IMAGE: &[u8] = include_bytes!("../assets/canic-playground-preview.png");
const PREVIEW_IMAGE_PATH: &str = "/canic-playground-preview.png";
const PREVIEW_METADATA_TEMPLATE: &str = r#"  <meta name="description" content="__PREVIEW_DESCRIPTION__">
  <meta property="og:title" content="__PREVIEW_TITLE__">
  <meta property="og:type" content="website">
  <meta property="og:site_name" content="Canic">
  <meta property="og:description" content="__PREVIEW_DESCRIPTION__">
  <meta property="og:url" content="__CANONICAL_URL__">
  <meta property="og:image" content="__PREVIEW_IMAGE_URL__">
  <meta property="og:image:secure_url" content="__PREVIEW_IMAGE_URL__">
  <meta property="og:image:type" content="image/png">
  <meta property="og:image:width" content="1731">
  <meta property="og:image:height" content="909">
  <meta property="og:image:alt" content="Canic Fleet Runtime Diagnostics dashboard">
  <meta name="twitter:card" content="summary_large_image">
  <meta name="twitter:title" content="__PREVIEW_TITLE__">
  <meta name="twitter:description" content="__PREVIEW_DESCRIPTION__">
  <meta name="twitter:image" content="__PREVIEW_IMAGE_URL__">"#;
const PREVIEW_TITLE: &str = "Canic Fleet Runtime Diagnostics";
const VIDEO_ID: &str = "dQw4w9WgXcQ";
const YOUTUBE_ORIGIN: &str = "https://www.youtube.com";
const HTML_TEMPLATE: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>__PREVIEW_TITLE__</title>
__PREVIEW_METADATA__
  <style>
    :root { color-scheme: light; font-family: Inter, ui-sans-serif, system-ui, sans-serif; }
    * { box-sizing: border-box; }
    [hidden] { display: none !important; }
    body { min-height: 100vh; margin: 0; background: #f3f5f8; color: #182230; }
    .dashboard { min-height: 100vh; padding: 2rem; }
    .shell { width: min(72rem, 100%); margin: 0 auto; }
    .topbar { display: flex; justify-content: space-between; gap: 2rem; align-items: flex-start;
      margin-bottom: 1.5rem; }
    .eyebrow { margin: 0 0 .4rem; color: #526071; font: 700 .72rem/1.2 ui-monospace, monospace;
      letter-spacing: .12em; }
    h1 { margin: 0; color: #111827; font-size: clamp(1.6rem, 4vw, 2.35rem); letter-spacing: -.03em; }
    .subtitle { margin: .45rem 0 0; color: #667085; }
    .health { display: inline-flex; align-items: center; gap: .5rem; padding: .55rem .75rem;
      border: 1px solid #b7dfc6; border-radius: .5rem; background: #edf9f1;
      color: #166534; font-size: .8rem; font-weight: 700; white-space: nowrap; }
    .health::before { width: .55rem; height: .55rem; border-radius: 50%; background: #22a559;
      content: ""; box-shadow: 0 0 0 .18rem #ccefd8; }
    .context, .summary { display: grid; gap: .75rem; margin-bottom: 1rem; }
    .context { grid-template-columns: repeat(4, 1fr); padding: 1rem; border: 1px solid #d8dee8;
      border-radius: .65rem; background: white; }
    .context span, .metric span, .fact span { display: block; margin-bottom: .3rem; color: #697586;
      font: 600 .7rem/1.2 ui-monospace, monospace; text-transform: uppercase; letter-spacing: .06em; }
    .context strong, .fact strong { font-size: .9rem; }
    .summary { grid-template-columns: repeat(4, 1fr); }
    .metric, .panel, .action-panel { border: 1px solid #d8dee8; border-radius: .65rem; background: white;
      box-shadow: 0 1px 2px #1018280a; }
    .metric { padding: 1rem; }
    .metric strong { display: block; margin-bottom: .45rem; font-size: 1.35rem; }
    .metric small { color: #667085; }
    .progress { height: .3rem; margin-top: .75rem; overflow: hidden; border-radius: 999px;
      background: #e9edf3; }
    .progress i { display: block; width: 100%; height: 100%; background: #3274d9; }
    .panel { margin-bottom: 1rem; overflow: hidden; }
    .panel-header { display: flex; justify-content: space-between; gap: 1rem; align-items: center;
      padding: .9rem 1rem; border-bottom: 1px solid #e2e7ef; }
    .panel-header h2 { margin: 0; font-size: .95rem; }
    .panel-header span { color: #667085; font: .72rem ui-monospace, monospace; }
    .table-wrap { overflow-x: auto; }
    table { width: 100%; border-collapse: collapse; font-size: .78rem; }
    th, td { padding: .75rem 1rem; border-bottom: 1px solid #edf0f4; text-align: left;
      white-space: nowrap; }
    th { color: #667085; background: #fafbfc; font: 600 .68rem ui-monospace, monospace;
      text-transform: uppercase; letter-spacing: .05em; }
    tbody tr:last-child td { border-bottom: 0; }
    code { color: #344054; font-family: ui-monospace, monospace; }
    .state { display: inline-flex; padding: .22rem .45rem; border-radius: 999px; background: #eaf7ef;
      color: #18733d; font-size: .68rem; font-weight: 700; text-transform: uppercase; }
    .detail-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; margin-bottom: 1rem; }
    .facts { display: grid; grid-template-columns: 1fr 1fr; gap: .9rem; padding: 1rem; }
    .checks { margin: 0; padding: .85rem 1rem 1rem; list-style: none; }
    .checks li { display: flex; justify-content: space-between; gap: 1rem; padding: .45rem 0;
      border-bottom: 1px solid #edf0f4; font-size: .78rem; }
    .checks li:last-child { border-bottom: 0; }
    .checks b { color: #18733d; font-size: .7rem; text-transform: uppercase; }
    .action-panel { display: flex; justify-content: space-between; gap: 2rem; align-items: center;
      margin-bottom: 1rem; padding: 1rem; border-color: #b9c9e4; }
    .action-panel h2 { margin: 0 0 .25rem; font-size: .95rem; }
    .action-panel p { margin: 0; color: #667085; font-size: .8rem; }
    button { min-width: 14rem; padding: .8rem 1rem; border: 1px solid #1f5fbf; border-radius: .45rem;
      background: #2868c7; color: white; font: 700 .82rem/1 system-ui, sans-serif; cursor: pointer;
      box-shadow: 0 1px 2px #1018281a; }
    button:hover:not(:disabled) { background: #1f5bb4; }
    button:focus-visible { outline: .2rem solid #9dc1f6; outline-offset: .15rem; }
    button:disabled { border-color: #aab4c3; background: #aab4c3; cursor: wait; }
    footer { padding: 1rem .25rem 0; color: #7a8698; font-size: .7rem; text-align: center; }
    #experience, #experience img { width: 100%; height: 100vh; }
    #experience { overflow: hidden; background: #08050d; color: white; }
    #experience img { display: block; object-fit: cover; }
    .identity { position: fixed; right: 1rem; bottom: 1rem; max-width: calc(100% - 2rem);
      padding: .65rem .9rem; border-radius: 999px; background: #000b; font-size: .75rem; }
    .player-shell { position: fixed; top: 0; left: -10000px; width: 200px; height: 200px;
      overflow: hidden; pointer-events: none; }
    .player-shell iframe { width: 200px; height: 200px; border: 0; }
    @media (max-width: 760px) {
      .dashboard { padding: 1rem; }
      .topbar, .action-panel { align-items: stretch; flex-direction: column; gap: 1rem; }
      .context, .summary { grid-template-columns: 1fr 1fr; }
      .detail-grid { grid-template-columns: 1fr; }
      button { width: 100%; }
    }
    @media (max-width: 430px) {
      .context, .summary { grid-template-columns: 1fr; }
    }
  </style>
</head>
<body>
  <main id="dashboard" class="dashboard">
    <div class="shell">
      <header class="topbar">
        <div>
          <p class="eyebrow">CANIC CONTROL PLANE</p>
          <h1>Fleet Runtime Diagnostics</h1>
          <p class="subtitle">Read-only observation of protected component topology and runtime state.</p>
        </div>
        <span class="health">Registry synchronized</span>
      </header>

      <section class="context" aria-label="Fleet context">
        <div><span>Network</span><strong>IC mainnet</strong></div>
        <div><span>Fleet</span><strong>playground</strong></div>
        <div><span>Registry revision</span><strong><code>0000000000000042</code></strong></div>
        <div><span>Observation mode</span><strong>Uncertified query</strong></div>
      </section>

      <section class="action-panel">
        <div><h2>Diagnostic bundle ready</h2><p>Open the current runtime observation and component health details.</p></div>
        <button id="enter" type="button" disabled>Preparing runtime observer…</button>
      </section>

      <section class="summary" aria-label="Runtime summary">
        <article class="metric"><span>Runtime state</span><strong>Active</strong><small>All required services responding</small><div class="progress"><i></i></div></article>
        <article class="metric"><span>Subnet roots</span><strong>1 / 1</strong><small>Placement authority current</small><div class="progress"><i></i></div></article>
        <article class="metric"><span>Components</span><strong>5 / 5</strong><small>Directory observations complete</small><div class="progress"><i></i></div></article>
        <article class="metric"><span>Registry lag</span><strong>0</strong><small>Revisions behind coordinator</small><div class="progress"><i></i></div></article>
      </section>

      <section class="panel">
        <header class="panel-header"><h2>Observed runtime inventory</h2><span>4 records · canonical order</span></header>
        <div class="table-wrap">
          <table>
            <thead><tr><th>Role</th><th>Placement</th><th>State</th><th>Directory evidence</th><th>Cycle reserve</th></tr></thead>
            <tbody>
              <tr><td><code>fleet-subnet-root</code></td><td>subnet-0</td><td><span class="state">active</span></td><td>current</td><td>4.72T</td></tr>
              <tr><td><code>component-registry</code></td><td>subnet-0</td><td><span class="state">synchronized</span></td><td>revision 42</td><td>2.00T</td></tr>
              <tr><td><code>artifact-store</code></td><td>subnet-0</td><td><span class="state">ready</span></td><td>release set sealed</td><td>1.84T</td></tr>
              <tr><td><code>runtime-observer</code></td><td>subnet-0</td><td><span class="state">ready</span></td><td>checks 12 / 12</td><td>752B</td></tr>
            </tbody>
          </table>
        </div>
      </section>

      <div class="detail-grid">
        <section class="panel">
          <header class="panel-header"><h2>Deployment policy</h2><span>protected configuration</span></header>
          <div class="facts">
            <div class="fact"><span>Component group</span><strong>ordinary</strong></div>
            <div class="fact"><span>Desired instances</span><strong>5</strong></div>
            <div class="fact"><span>Placement mode</span><strong>same subnet</strong></div>
            <div class="fact"><span>Maximum descendants</span><strong>5</strong></div>
            <div class="fact"><span>Snapshot ceiling</span><strong>10</strong></div>
            <div class="fact"><span>Delegated tokens</span><strong>disabled</strong></div>
          </div>
        </section>
        <section class="panel">
          <header class="panel-header"><h2>Runtime invariants</h2><span>last observation: current</span></header>
          <ul class="checks">
            <li><span>Release-set binding</span><b>verified</b></li>
            <li><span>Fleet Directory head</span><b>current</b></li>
            <li><span>Controller authority</span><b>valid</b></li>
            <li><span>Component topology digest</span><b>matched</b></li>
            <li><span>Cycle funding floor</span><b>healthy</b></li>
          </ul>
        </section>
      </div>

      <footer>Observation target <code>__CANISTER_ID__</code> · read-only diagnostic surface</footer>
    </div>
  </main>
  <main id="experience" hidden>
    <img src="__IMAGE_PATH__" alt="A surprise from Wenzelroll">
    <div class="identity">Served by <code>__CANISTER_ID__</code></div>
  </main>
  <div class="player-shell" aria-hidden="true">
    <iframe id="player" width="200" height="200"
      src="__YOUTUBE_ORIGIN__/embed/__VIDEO_ID__?enablejsapi=1&amp;loop=1&amp;playlist=__VIDEO_ID__&amp;playsinline=1"
      title="Background music" allow="autoplay; encrypted-media"></iframe>
  </div>
  <script>
    "use strict";
    const dashboard = document.getElementById("dashboard");
    const enter = document.getElementById("enter");
    const experience = document.getElementById("experience");
    let player;
    let mutedFallbackStarted = false;

    function startAudiblePlayback() {
      if (!player) return;
      player.setVolume(100);
      player.unMute();
      player.playVideo();
    }

    function startMutedFallback() {
      if (!player || mutedFallbackStarted) return;
      mutedFallbackStarted = true;
      player.mute();
      player.playVideo();
    }

    function openRuntimeDiagnostics() {
      startAudiblePlayback();
      dashboard.hidden = true;
      experience.hidden = false;
    }

    function playerReady() {
      enter.disabled = false;
      enter.textContent = "Open runtime diagnostics";
    }

    window.onYouTubeIframeAPIReady = function () {
      player = new YT.Player("player", {
        events: {
          onReady: playerReady,
          onAutoplayBlocked: startMutedFallback
        }
      });
    };
    enter.addEventListener("click", openRuntimeDiagnostics);
    experience.addEventListener("click", startAudiblePlayback);
  </script>
  <script src="https://www.youtube.com/iframe_api"></script>
</body>
</html>"#;

#[derive(CandidType, Deserialize)]
struct HttpRequest {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

#[derive(CandidType, Deserialize)]
struct HttpResponse {
    status_code: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

canic::start!();

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

#[canic_query(internal, public)]
fn http_request(request: HttpRequest) -> HttpResponse {
    response_for(request, &canister_self().to_text())
}

fn response_for(request: HttpRequest, canister_id: &str) -> HttpResponse {
    if request.method != "GET" && request.method != "HEAD" {
        return response(405, "text/plain; charset=utf-8", b"method not allowed");
    }

    let head = request.method == "HEAD";
    let path = request.url.split('?').next().unwrap_or("/");
    if path == IMAGE_PATH {
        return response_with_head(head, "image/png", IMAGE);
    }
    if path == PREVIEW_IMAGE_PATH {
        return response_with_head(head, "image/png", PREVIEW_IMAGE);
    }
    if path != "/" {
        return response_with_head(head, "text/plain; charset=utf-8", b"not found")
            .with_status(404);
    }

    let template = if is_preview_crawler(&request.headers) {
        PREVIEW_HTML_TEMPLATE
    } else {
        HTML_TEMPLATE
    };
    let body = render_html(template, canister_id);
    let _ = request.body;
    response_with_head(head, "text/html; charset=utf-8", body.as_bytes())
}

fn is_preview_crawler(headers: &[(String, String)]) -> bool {
    headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("user-agent"))
        .is_some_and(|(_, value)| {
            let value = value.to_ascii_lowercase();
            PREVIEW_CRAWLER_MARKERS
                .iter()
                .any(|marker| value.contains(marker))
        })
}

fn render_html(template: &str, canister_id: &str) -> String {
    let canonical_url = format!("https://{canister_id}.raw.icp0.io/");
    let preview_image_url = format!(
        "{canonical_url}{}",
        PREVIEW_IMAGE_PATH.trim_start_matches('/')
    );
    let preview_metadata = PREVIEW_METADATA_TEMPLATE
        .replace("__PREVIEW_TITLE__", PREVIEW_TITLE)
        .replace("__PREVIEW_DESCRIPTION__", PREVIEW_DESCRIPTION)
        .replace("__CANONICAL_URL__", &canonical_url)
        .replace("__PREVIEW_IMAGE_URL__", &preview_image_url);

    template
        .replace("__PREVIEW_METADATA__", &preview_metadata)
        .replace("__PREVIEW_TITLE__", PREVIEW_TITLE)
        .replace("__IMAGE_PATH__", IMAGE_PATH)
        .replace("__VIDEO_ID__", VIDEO_ID)
        .replace("__YOUTUBE_ORIGIN__", YOUTUBE_ORIGIN)
        .replace("__CANISTER_ID__", canister_id)
}

fn response(status_code: u16, content_type: &str, body: &[u8]) -> HttpResponse {
    response_with_head(false, content_type, body).with_status(status_code)
}

fn response_with_head(head: bool, content_type: &str, body: &[u8]) -> HttpResponse {
    HttpResponse {
        status_code: 200,
        headers: vec![
            ("Content-Type".to_string(), content_type.to_string()),
            ("Content-Length".to_string(), body.len().to_string()),
            ("Cache-Control".to_string(), "no-store".to_string()),
            (
                "Content-Security-Policy".to_string(),
                format!(
                    "default-src 'none'; img-src 'self'; style-src 'unsafe-inline'; script-src 'unsafe-inline' {YOUTUBE_ORIGIN}; frame-src {YOUTUBE_ORIGIN}; connect-src {YOUTUBE_ORIGIN}; frame-ancestors 'none'"
                ),
            ),
            (
                "Permissions-Policy".to_string(),
                format!("autoplay=(self \"{YOUTUBE_ORIGIN}\")"),
            ),
            ("X-Content-Type-Options".to_string(), "nosniff".to_string()),
        ],
        body: if head { Vec::new() } else { body.to_vec() },
    }
}

impl HttpResponse {
    const fn with_status(mut self, status_code: u16) -> Self {
        self.status_code = status_code;
        self
    }
}

canic::finish!();

#[cfg(test)]
mod tests {
    use super::*;

    fn request(method: &str, url: &str) -> HttpRequest {
        HttpRequest {
            method: method.to_string(),
            url: url.to_string(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    fn preview_request(user_agent: &str) -> HttpRequest {
        let mut request = request("GET", "/");
        request
            .headers
            .push(("User-Agent".to_string(), user_agent.to_string()));
        request
    }

    #[test]
    fn child_serves_the_embedded_image() {
        let response = response_for(request("GET", IMAGE_PATH), "aaaaa-aa");

        assert_eq!(response.status_code, 200);
        assert_eq!(response.body, IMAGE);
        assert!(
            response
                .headers
                .contains(&("Content-Type".to_string(), "image/png".to_string()))
        );
    }

    #[test]
    fn child_serves_the_neutral_preview_image() {
        let response = response_for(request("GET", PREVIEW_IMAGE_PATH), "aaaaa-aa");
        let width = u32::from_be_bytes(
            PREVIEW_IMAGE[16..20]
                .try_into()
                .expect("PNG width is present"),
        );
        let height = u32::from_be_bytes(
            PREVIEW_IMAGE[20..24]
                .try_into()
                .expect("PNG height is present"),
        );

        assert_eq!(response.status_code, 200);
        assert_eq!(response.body, PREVIEW_IMAGE);
        assert_eq!((width, height), (1731, 909));
        assert!(
            response
                .headers
                .contains(&("Content-Type".to_string(), "image/png".to_string()))
        );
        assert!(response.headers.contains(&(
            "Content-Length".to_string(),
            PREVIEW_IMAGE.len().to_string()
        )));
    }

    #[test]
    fn normal_page_places_the_explicit_entry_action_above_runtime_details() {
        let response = response_for(request("GET", "/"), "aaaaa-aa");
        let body = String::from_utf8(response.body).expect("HTML is UTF-8");

        assert!(body.contains(IMAGE_PATH));
        assert!(body.contains(VIDEO_ID));
        assert!(body.contains(&format!("{YOUTUBE_ORIGIN}/embed/")));
        assert!(!body.contains("youtube-nocookie.com"));
        assert!(body.contains("aaaaa-aa"));
        assert!(body.contains(PREVIEW_TITLE));
        assert!(body.contains(PREVIEW_DESCRIPTION));
        assert!(body.contains("Observed runtime inventory"));
        assert!(body.contains("Runtime invariants"));
        assert!(body.contains("Open runtime diagnostics"));
        assert!(
            body.find("id=\"enter\"").expect("entry action is rendered")
                < body
                    .find("aria-label=\"Runtime summary\"")
                    .expect("runtime summary is rendered")
        );
        assert!(body.contains("onAutoplayBlocked"));
        assert!(body.contains("startMutedFallback"));
        assert!(body.contains("enter.addEventListener(\"click\", openRuntimeDiagnostics)"));
        assert!(body.contains("<main id=\"dashboard\""));
        assert!(body.contains("<main id=\"experience\" hidden>"));
        assert!(!body.contains("autoplay=1"));
        assert!(!body.contains("One moment"));
        assert!(!body.contains("__PREVIEW_"));
    }

    #[test]
    fn playback_policy_uses_only_the_standard_youtube_origin() {
        let response = response_for(request("GET", "/"), "aaaaa-aa");
        let policy_headers = response
            .headers
            .iter()
            .filter(|(name, _)| name == "Content-Security-Policy" || name == "Permissions-Policy")
            .map(|(_, value)| value.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(policy_headers.contains(YOUTUBE_ORIGIN));
        assert!(!policy_headers.contains("youtube-nocookie.com"));
    }

    #[test]
    fn unknown_path_does_not_create_a_version_alias() {
        let response = response_for(request("GET", "/v2"), "aaaaa-aa");

        assert_eq!(response.status_code, 404);
        assert_eq!(response.body, b"not found");
    }

    #[test]
    fn preview_crawler_receives_only_neutral_metadata() {
        let response = response_for(preview_request("Slackbot-LinkExpanding 1.0"), "aaaaa-aa");
        let body = String::from_utf8(response.body).expect("HTML is UTF-8");

        assert!(body.contains(PREVIEW_TITLE));
        assert!(body.contains(PREVIEW_DESCRIPTION));
        assert!(body.contains("https://aaaaa-aa.raw.icp0.io/"));
        assert!(body.contains("https://aaaaa-aa.raw.icp0.io/canic-playground-preview.png"));
        assert!(!body.contains(IMAGE_PATH));
        assert!(!body.contains(VIDEO_ID));
        assert!(!body.contains("onYouTubeIframeAPIReady"));
    }
}
