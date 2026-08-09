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
  <title>Canic Playground</title>
__PREVIEW_METADATA__
</head>
<body></body>
</html>"#;
const PREVIEW_IMAGE: &[u8] = include_bytes!("../assets/canic-playground-preview.png");
const PREVIEW_IMAGE_PATH: &str = "/canic-playground-preview.png";
const PREVIEW_METADATA_TEMPLATE: &str = r#"  <meta name="description" content="A small Internet Computer experiment.">
  <meta property="og:title" content="Canic Playground">
  <meta property="og:type" content="website">
  <meta property="og:description" content="A small Internet Computer experiment.">
  <meta property="og:url" content="__CANONICAL_URL__">
  <meta property="og:image" content="__PREVIEW_IMAGE_URL__">
  <meta property="og:image:secure_url" content="__PREVIEW_IMAGE_URL__">
  <meta property="og:image:type" content="image/png">
  <meta property="og:image:width" content="1729">
  <meta property="og:image:height" content="910">
  <meta property="og:image:alt" content="An abstract network of connected glowing nodes">
  <meta name="twitter:card" content="summary_large_image">
  <meta name="twitter:title" content="Canic Playground">
  <meta name="twitter:description" content="A small Internet Computer experiment.">
  <meta name="twitter:image" content="__PREVIEW_IMAGE_URL__">"#;
const VIDEO_ID: &str = "dQw4w9WgXcQ";
const HTML_TEMPLATE: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Canic Playground</title>
__PREVIEW_METADATA__
  <style>
    :root { color-scheme: dark; font-family: ui-rounded, system-ui, sans-serif; }
    * { box-sizing: border-box; }
    body { min-height: 100vh; margin: 0; overflow: hidden; background: #08050d; color: white; }
    main, main img { width: 100%; height: 100vh; }
    main img { display: block; object-fit: cover; }
    .identity { position: fixed; right: 1rem; bottom: 1rem; max-width: calc(100% - 2rem);
      padding: .65rem .9rem; border-radius: 999px; background: #000b; font-size: .75rem; }
    .player-shell { position: fixed; top: 0; left: -10000px; width: 200px; height: 200px;
      overflow: hidden; pointer-events: none; }
    .player-shell iframe { width: 200px; height: 200px; border: 0; }
    code { overflow-wrap: anywhere; }
  </style>
</head>
<body>
  <main id="experience">
    <img src="__IMAGE_PATH__" alt="A surprise from Wenzelroll">
    <div class="identity">Served by <code>__CANISTER_ID__</code></div>
  </main>
  <div class="player-shell" aria-hidden="true">
    <iframe id="player" width="200" height="200"
      src="https://www.youtube-nocookie.com/embed/__VIDEO_ID__?enablejsapi=1&amp;autoplay=1&amp;loop=1&amp;playlist=__VIDEO_ID__&amp;playsinline=1"
      title="Background music" allow="autoplay; encrypted-media"></iframe>
  </div>
  <script>
    "use strict";
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

    function startFromGesture(event) {
      if (event.type === "keydown" &&
          (event.repeat || (event.key !== "Enter" && event.key !== " "))) return;
      startAudiblePlayback();
    }

    window.onYouTubeIframeAPIReady = function () {
      player = new YT.Player("player", {
        events: {
          onReady: startAudiblePlayback,
          onAutoplayBlocked: startMutedFallback
        }
      });
    };
    document.addEventListener("click", startFromGesture);
    document.addEventListener("keydown", startFromGesture);
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
    let canonical_url = format!("https://{canister_id}.icp0.io/");
    let preview_image_url = format!(
        "{canonical_url}{}",
        PREVIEW_IMAGE_PATH.trim_start_matches('/')
    );
    let preview_metadata = PREVIEW_METADATA_TEMPLATE
        .replace("__CANONICAL_URL__", &canonical_url)
        .replace("__PREVIEW_IMAGE_URL__", &preview_image_url);

    template
        .replace("__PREVIEW_METADATA__", &preview_metadata)
        .replace("__IMAGE_PATH__", IMAGE_PATH)
        .replace("__VIDEO_ID__", VIDEO_ID)
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
            ("Cache-Control".to_string(), "no-store".to_string()),
            (
                "Content-Security-Policy".to_string(),
                "default-src 'none'; img-src 'self'; style-src 'unsafe-inline'; script-src 'unsafe-inline' https://www.youtube.com; frame-src https://www.youtube-nocookie.com; connect-src https://www.youtube.com https://www.youtube-nocookie.com; frame-ancestors 'none'".to_string(),
            ),
            (
                "Permissions-Policy".to_string(),
                "autoplay=(self \"https://www.youtube-nocookie.com\")".to_string(),
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

        assert_eq!(response.status_code, 200);
        assert_eq!(response.body, PREVIEW_IMAGE);
        assert!(
            response
                .headers
                .contains(&("Content-Type".to_string(), "image/png".to_string()))
        );
    }

    #[test]
    fn normal_page_immediately_shows_wenzelroll_with_silent_sound_recovery() {
        let response = response_for(request("GET", "/"), "aaaaa-aa");
        let body = String::from_utf8(response.body).expect("HTML is UTF-8");

        assert!(body.contains(IMAGE_PATH));
        assert!(body.contains(VIDEO_ID));
        assert!(body.contains("aaaaa-aa"));
        assert!(body.contains("onAutoplayBlocked"));
        assert!(body.contains("startMutedFallback"));
        assert!(body.contains("document.addEventListener(\"click\""));
        assert!(body.contains("document.addEventListener(\"keydown\""));
        assert!(body.contains("<main id=\"experience\">"));
        assert!(!body.contains("id=\"gate\""));
        assert!(!body.contains("One moment"));
    }

    #[test]
    fn preview_crawler_receives_only_neutral_metadata() {
        let response = response_for(preview_request("Slackbot-LinkExpanding 1.0"), "aaaaa-aa");
        let body = String::from_utf8(response.body).expect("HTML is UTF-8");

        assert!(body.contains("Canic Playground"));
        assert!(body.contains("A small Internet Computer experiment."));
        assert!(body.contains("https://aaaaa-aa.icp0.io/"));
        assert!(body.contains("https://aaaaa-aa.icp0.io/canic-playground-preview.png"));
        assert!(!body.contains(IMAGE_PATH));
        assert!(!body.contains(VIDEO_ID));
        assert!(!body.contains("onYouTubeIframeAPIReady"));
    }
}
