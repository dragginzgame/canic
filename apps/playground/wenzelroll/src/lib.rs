#![expect(clippy::unused_async)]

use candid::{CandidType, Deserialize};
use canic::prelude::*;
use ic_cdk::api::canister_self;

const IMAGE: &[u8] = include_bytes!("../assets/wenzelroll.png");
const IMAGE_PATH: &str = "/wenzelroll.png";
const VIDEO_ID: &str = "dQw4w9WgXcQ";
const HTML_TEMPLATE: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Definitely not a rickroll</title>
  <style>
    :root { color-scheme: dark; font-family: ui-rounded, system-ui, sans-serif; }
    * { box-sizing: border-box; }
    [hidden] { display: none !important; }
    body { min-height: 100vh; margin: 0; overflow: hidden; background: #08050d; color: white; }
    main, main img { width: 100%; height: 100vh; }
    main img { display: block; object-fit: cover; }
    .gate { position: fixed; z-index: 2; inset: 0; display: grid; place-items: center;
      padding: 1.5rem; text-align: center; background: #08050d; }
    .gate-card { width: min(34rem, 100%); padding: 2rem; border: 1px solid #ffffff24;
      border-radius: 1.5rem; background: #160f20; box-shadow: 0 1.5rem 5rem #000a; }
    .gate h1 { margin: 0 0 .75rem; font-size: clamp(2rem, 8vw, 4rem); }
    .gate p { margin: 0 0 1.25rem; color: #dbcfe4; }
    .gate button { width: 100%; padding: 1rem 1.25rem; border: 0; border-radius: 999px;
      background: #ff3e9d; color: #16000b; font: inherit; font-weight: 800; cursor: pointer; }
    .gate button:disabled { cursor: wait; opacity: .55; }
    .gate a { display: inline-block; margin-top: 1rem; color: #ff9bd5; }
    .identity { position: fixed; right: 1rem; bottom: 1rem; max-width: calc(100% - 2rem);
      padding: .65rem .9rem; border-radius: 999px; background: #000b; font-size: .75rem; }
    .player-shell { position: fixed; top: 0; left: -10000px; width: 200px; height: 200px;
      overflow: hidden; pointer-events: none; }
    .player-shell iframe { width: 200px; height: 200px; border: 0; }
    code { overflow-wrap: anywhere; }
  </style>
</head>
<body>
  <section id="gate" class="gate" aria-live="polite">
    <div class="gate-card">
      <h1>One moment…</h1>
      <p id="gate-status">Starting the essential background music.</p>
      <button id="enter" type="button" disabled>Start the rickroll</button>
      <a href="https://www.youtube.com/watch?v=__VIDEO_ID__" target="_blank" rel="noreferrer">
        Open the official video if the player is unavailable
      </a>
    </div>
  </section>
  <main id="experience" hidden>
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
    const gate = document.getElementById("gate");
    const gateStatus = document.getElementById("gate-status");
    const enter = document.getElementById("enter");
    const experience = document.getElementById("experience");
    let player;
    let entered = false;

    function revealWhenAudible() {
      if (entered || !player || player.getPlayerState() !== YT.PlayerState.PLAYING) return;
      if (player.isMuted() || player.getVolume() === 0) return;
      entered = true;
      gate.hidden = true;
      experience.hidden = false;
    }

    function startMusic() {
      if (!player) return;
      player.setVolume(100);
      player.unMute();
      player.playVideo();
      window.setTimeout(revealWhenAudible, 250);
    }

    function playbackBlocked() {
      gateStatus.textContent = "Your browser blocked audible autoplay. Start it to enter.";
      enter.disabled = false;
    }

    function playbackFailed() {
      gateStatus.textContent = "The background player could not load.";
      enter.disabled = true;
    }

    window.onYouTubeIframeAPIReady = function () {
      player = new YT.Player("player", {
        events: {
          onReady: function () {
            enter.disabled = false;
            startMusic();
          },
          onStateChange: revealWhenAudible,
          onAutoplayBlocked: playbackBlocked,
          onError: playbackFailed
        }
      });
    };
    enter.addEventListener("click", startMusic);
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

    let path = request.url.split('?').next().unwrap_or("/");
    if path == IMAGE_PATH {
        return response_with_head(request.method == "HEAD", "image/png", IMAGE);
    }

    let body = HTML_TEMPLATE
        .replace("__IMAGE_PATH__", IMAGE_PATH)
        .replace("__VIDEO_ID__", VIDEO_ID)
        .replace("__CANISTER_ID__", canister_id);
    let _ = (request.headers, request.body);
    response_with_head(
        request.method == "HEAD",
        "text/html; charset=utf-8",
        body.as_bytes(),
    )
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
    fn child_page_embeds_the_image_and_official_player() {
        let response = response_for(request("GET", "/"), "aaaaa-aa");
        let body = String::from_utf8(response.body).expect("HTML is UTF-8");

        assert!(body.contains(IMAGE_PATH));
        assert!(body.contains(VIDEO_ID));
        assert!(body.contains("aaaaa-aa"));
        assert!(body.contains("onAutoplayBlocked"));
        assert!(body.contains("revealWhenAudible"));
        assert!(body.contains("experience\" hidden"));
    }
}
