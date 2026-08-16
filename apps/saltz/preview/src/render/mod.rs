//! Module: saltz_preview::render
//!
//! Responsibility: render the exact compiled Saltz trace as a T2-style browser graph.
//! Does not own: source-image reuse, HTTP routing, live IC metrics, or burn execution.
//! Boundary: presentation consumes only build-verified generated waveform constants.

include!(concat!(env!("OUT_DIR"), "/waveform.rs"));

const TRACE_SHA256: &str = "c0b281f64e6f07e65ca6efd121919d8023f8640b6d429b54e0b739f3c84b6d50";
const PAGE: &str = r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <meta name="theme-color" content="#050607">
  <meta name="description" content="Inert canister preview of a proposed, unqualified cycle-burn-rate waveform. No intentional burn capability is compiled.">
  <title>Das Domrestaurantwandkunstzyklusbrenngraphnachbildung</title>
  <style>
    :root{color-scheme:dark;font-family:"IBM Plex Mono","SFMono-Regular",Consolas,monospace;--void:#030405;--panel:#090c0d;--line:#243033;--red:#f23838;--red-dim:#7e2020;--amber:#ffdf72;--green:#8cff9b;--text:#d7dedc;--muted:#778481}
    *{box-sizing:border-box}html{background:var(--void)}body{min-height:100vh;margin:0;background:radial-gradient(circle at 50% -15%,#361010 0,transparent 38rem),linear-gradient(#050708ed,#030405),repeating-linear-gradient(0deg,#ffffff05 0,#ffffff05 1px,transparent 1px,transparent 4px);color:var(--text)}
    body:after{position:fixed;inset:0;z-index:10;pointer-events:none;content:"";background:linear-gradient(90deg,#ff000008,transparent 30%,#00ff1a05 65%,#0000ff08);mix-blend-mode:screen}.shell{width:min(100rem,100%);margin:auto;padding:1.25rem}.top{display:flex;justify-content:space-between;gap:2rem;align-items:flex-start;padding:1.3rem 0 1.5rem;border-bottom:1px solid var(--red-dim)}
    .kicker,.label{color:var(--red);font-size:.68rem;font-weight:800;letter-spacing:.16em;text-transform:uppercase}.kicker{margin:0 0 .5rem}.top h1{max-width:72rem;margin:0;font:800 clamp(1.7rem,4.2vw,4rem)/.88 system-ui,sans-serif;letter-spacing:-.065em;overflow-wrap:anywhere}.top h1 span{color:var(--red)}.subtitle{max-width:64rem;margin:.8rem 0 0;color:#9eaaa7;font-size:.82rem;line-height:1.55}
    .signal{min-width:18rem;padding:.75rem 1rem;border:1px solid #36503b;background:#08130b;color:var(--green);font-size:.7rem;text-align:right;box-shadow:inset 0 0 1.2rem #00ff1a0d}.signal b{display:block;margin-bottom:.3rem;font-size:.95rem}.stats{display:grid;grid-template-columns:repeat(4,1fr);gap:.65rem;margin:1rem 0}.stat,.panel{border:1px solid var(--line);background:linear-gradient(145deg,#0c1011ee,#070909ee);box-shadow:0 .8rem 2rem #0008}.stat{min-height:6.3rem;padding:.8rem}.stat strong{display:block;margin:.55rem 0 .35rem;color:#f2f5f4;font:700 clamp(1rem,2vw,1.55rem)/1.05 system-ui,sans-serif}.stat small{color:var(--muted);font-size:.64rem}
    .panel{margin-bottom:.8rem;overflow:hidden}.panel>header{display:flex;justify-content:space-between;gap:1rem;align-items:center;padding:.75rem .9rem;border-bottom:1px solid var(--line);background:#0f1415}.panel h2{margin:0;font:800 .77rem system-ui,sans-serif;letter-spacing:.08em;text-transform:uppercase}.panel header span{color:var(--muted);font-size:.62rem}.graph{position:relative;padding:.8rem;background:#020303}.graph svg{display:block;width:100%;height:auto}.gridline{stroke:#405053;stroke-width:1}.axis{fill:#9aa6a3;font-size:13px}.axis-title{fill:#7f8c89;font-size:11px;letter-spacing:.08em}.band{fill:#5d171f;opacity:.32}.band-label{fill:#f08b92;font-size:12px;letter-spacing:.04em}.wave-glow{fill:none;stroke:#ffdf7270;stroke-width:10;stroke-linejoin:round;filter:blur(4px)}.wave{fill:none;stroke:var(--amber);stroke-width:2.2;stroke-linejoin:round;stroke-linecap:round}.target{stroke:#e07880;stroke-width:1;stroke-dasharray:6 7}.target-label{fill:#f08b92;font-size:11px;letter-spacing:.04em}.caption{display:grid;grid-template-columns:repeat(2,1fr);gap:1rem;padding:.8rem .9rem;border-top:1px solid var(--line);color:#8d9996;font-size:.65rem;line-height:1.5}.caption strong{display:block;margin-bottom:.2rem;color:#d7dedc;font-size:.64rem;letter-spacing:.06em;text-transform:uppercase}.caption span:first-child strong{color:var(--amber)}.caption span:nth-child(2) strong{color:#f08b92}
    .evidence{display:grid;grid-template-columns:1fr 1fr;gap:.8rem}.body{padding:.9rem}.fact{padding:.7rem;border-left:.18rem solid var(--red);background:#07090a}.fact+ .fact{margin-top:.5rem}.fact strong{display:block;margin:.35rem 0;color:#e7ecea;font-size:.72rem;overflow-wrap:anywhere}.fact small{color:var(--muted);font-size:.6rem}.actions{display:flex;flex-wrap:wrap;gap:.5rem;margin:0 0 .8rem}.button{padding:.55rem .72rem;border:1px solid var(--red-dim);background:#190909;color:#ff9a9a;font-size:.63rem;text-decoration:none;text-transform:uppercase}.button:hover{border-color:var(--red)}footer{display:grid;grid-template-columns:1fr auto 1fr;gap:1rem;align-items:center;padding:1rem 0;color:#5f6a68;font-size:.58rem;overflow-wrap:anywhere}footer>span:last-child{text-align:right}.made{padding:.42rem .72rem;border:1px solid #40282b;border-radius:999px;background:#12090a;color:#aeb7b5;font:600 .66rem system-ui,sans-serif;letter-spacing:.01em;white-space:nowrap}.made a{color:#d7dedc;text-decoration:none}.made a:hover{text-decoration:underline}.heart{color:#ff4545}
    @media(max-width:800px){.top{flex-direction:column}.signal{width:100%;text-align:left}.stats{grid-template-columns:repeat(2,1fr)}.evidence{grid-template-columns:1fr}.graph{padding:.2rem}.caption{grid-template-columns:1fr;gap:.65rem}footer{grid-template-columns:1fr;text-align:center}footer>span:last-child{text-align:center}.made{justify-self:center}}@media(max-width:430px){.shell{padding:.7rem}.stats{grid-template-columns:1fr}.top h1{font-size:2.6rem}}
  </style>
</head>
<body>
  <main class="shell">
    <header class="top">
      <div>
        <p class="kicker">INERT CANISTER PREVIEW // STATIC MODEL</p>
        <h1>Das <span>Domrestaurantwandkunstzyklusbrenngraphnachbildung</span></h1>
        <p class="subtitle">This canister maps one selected 860-point source trace into a hypothetical 24-hour global Cycle Burn Rate profile. It does not read live IC metrics, schedule a run or burn cycles.</p>
      </div>
      <div class="signal"><b>PREVIEW RESPONSE // SERVED</b>BURN CAPABILITY: NOT COMPILED<br>HTTP: RAW / UNCERTIFIED</div>
    </header>

    <section class="stats" aria-label="Waveform summary">
      <article class="stat"><span class="label">Source trace</span><strong>__POINT_COUNT__ points</strong><small>one source-image column per point; no horizontal smoothing</small></article>
      <article class="stat"><span class="label">Proposed duration</span><strong>24 hours</strong><small>hypothetical elapsed time; no start time or armed run exists</small></article>
      <article class="stat"><span class="label">Proposed Dashboard total</span><strong>100–150B/s</strong><small>global background plus controlled burn; not yet qualified</small></article>
      <article class="stat"><span class="label">Burn execution</span><strong>Disabled</strong><small>no cycles_burn path; ordinary query execution still consumes cycles</small></article>
    </section>

    <section class="panel">
      <header><h2>Proposed Dashboard Total</h2><span>STATIC MODEL // NOT LIVE OR QUALIFIED</span></header>
      <div class="graph">
        <svg viewBox="0 0 1280 340" role="img" aria-labelledby="graph-title graph-description">
          <title id="graph-title">Proposed global cycle-burn-rate profile over 24 hypothetical hours</title>
          <desc id="graph-description">The yellow line is a static proposal for a global Dashboard total ranging from 100 to 150 billion cycles per second. The red band is a dated global observation from 2026-08-15 through 2026-08-16. This data-only graph contains no restaurant image. This canister does not perform an intentional cycle burn.</desc>
          <rect class="band" x="80" y="186.574" width="1120" height="23.077"/>
          <text class="band-label" x="94" y="202">DATED GLOBAL SAMPLE // 31.7–49.9B/s // 2026-08-15/16</text>
          <line class="gridline" x1="80" y1="60" x2="1200" y2="60"/><line class="gridline" x1="80" y1="97.941" x2="1200" y2="97.941"/><line class="gridline" x1="80" y1="135.882" x2="1200" y2="135.882"/><line class="gridline" x1="80" y1="173.823" x2="1200" y2="173.823"/><line class="gridline" x1="80" y1="211.764" x2="1200" y2="211.764"/><line class="gridline" x1="80" y1="249.705" x2="1200" y2="249.705"/>
          <line class="gridline" x1="80" y1="60" x2="80" y2="249.705"/><line class="gridline" x1="360" y1="60" x2="360" y2="249.705"/><line class="gridline" x1="640" y1="60" x2="640" y2="249.705"/><line class="gridline" x1="920" y1="60" x2="920" y2="249.705"/><line class="gridline" x1="1200" y1="60" x2="1200" y2="249.705"/>
          <line class="target" x1="80" y1="123.235" x2="1200" y2="123.235"/>
          <text class="target-label" x="94" y="117">PROPOSED WAVEFORM FLOOR // 100B/s</text>
          <text class="axis-title" x="80" y="34">GLOBAL CYCLE BURN RATE // BILLION CYCLES PER SECOND</text>
          <text class="axis" x="18" y="65">150B</text><text class="axis" x="18" y="103">120B</text><text class="axis" x="24" y="141">90B</text><text class="axis" x="24" y="179">60B</text><text class="axis" x="24" y="217">30B</text><text class="axis" x="31" y="255">0B</text>
          <text class="axis" x="78" y="290">+00h</text><text class="axis" x="340" y="290">+06h</text><text class="axis" x="620" y="290">+12h</text><text class="axis" x="900" y="290">+18h</text><text class="axis" x="1156" y="290">+24h</text>
          <text class="axis-title" x="80" y="323">HYPOTHETICAL ELAPSED TIME // NO START IS ARMED</text>
          <polyline class="wave-glow" points="__WAVEFORM_POINTS__"/><polyline class="wave" points="__WAVEFORM_POINTS__"/>
        </svg>
      </div>
      <div class="caption"><span><strong>Yellow line</strong>Exact source-trace geometry mapped to a proposed 100–150B/s global total.</span><span><strong>Red band</strong>Frozen 2026-08-15/16 global sample, not a current baseline guarantee.</span></div>
    </section>

    <div class="actions"><a class="button" href="/waveform.csv">Download proposed waveform CSV</a><a class="button" href="/api/status.json">Inspect machine-readable status</a></div>

    <div class="evidence">
      <section class="panel"><header><h2>Artifact Provenance</h2><span>BUILD-VERIFIED</span></header><div class="body"><div class="fact"><span class="label">Proposed waveform CSV SHA-256</span><strong>__CSV_SHA256__</strong><small>860 contiguous rational buckets covering exactly 24 hypothetical hours</small></div><div class="fact"><span class="label">Numeric trace SHA-256</span><strong>__TRACE_SHA256__</strong><small>Exact anonymous geometry used by the yellow line; no source image is retained</small></div></div></section>
      <section class="panel"><header><h2>Runtime Truth</h2><span>INERT BY CONSTRUCTION</span></header><div class="body"><div class="fact"><span class="label">Actual canister behavior</span><strong>Static raw HTTP preview</strong><small>One query method; no update, timer, stable state, operator command or live metric fetch</small></div><div class="fact"><span class="label">Intentional cycle burn</span><strong>Impossible in this Wasm</strong><small>No cycles_burn call or active run exists; serving queries still incurs ordinary execution cost</small></div><div class="fact"><span class="label">Qualification state</span><strong>Still open</strong><small>This preview does not qualify Dashboard aggregation, execution timing, funding or authorization</small></div><div class="fact"><span class="label">Zero-background scale</span><strong>10,464.206 Tcycles</strong><small>Hypothetical 24-hour integral of the proposed total; not a funding amount or approved spend</small></div></div></section>
    </div>

    <footer><span>RESPONDING CANISTER // __CANISTER_ID__</span><span class="made">Made with <span class="heart">❤️</span> by <a href="https://github.com/dragginzgame/canic" rel="noreferrer">Canic</a></span><span>INERT PREVIEW // RAW HTTP // SCHEMA 1</span></footer>
  </main>
  <!-- stop being a paranoid dickhead -->
</body>
</html>"##;

pub fn page(canister_id: &str) -> String {
    PAGE.replace("__WAVEFORM_POINTS__", WAVEFORM_SVG_POINTS)
        .replace("__POINT_COUNT__", &WAVEFORM_POINT_COUNT.to_string())
        .replace("__CSV_SHA256__", CSV_SHA256)
        .replace("__TRACE_SHA256__", TRACE_SHA256)
        .replace("__CANISTER_ID__", &escape_html(canister_id))
}

pub fn status_json(canister_id: &str) -> String {
    format!(
        concat!(
            "{{\n",
            "  \"schema_version\": 1,\n",
            "  \"canister_id\": \"{}\",\n",
            "  \"mode\": \"inert_preview\",\n",
            "  \"response\": {{\n",
            "    \"served\": true,\n",
            "    \"certification\": \"uncertified_query_response\",\n",
            "    \"raw_gateway_required\": true\n",
            "  }},\n",
            "  \"runtime\": {{\n",
            "    \"query_methods\": [\"http_request\"],\n",
            "    \"update_methods\": [],\n",
            "    \"timers\": false,\n",
            "    \"stable_state\": false,\n",
            "    \"operator_commands\": false,\n",
            "    \"live_metric_fetch\": false\n",
            "  }},\n",
            "  \"intentional_burn\": {{\n",
            "    \"capability_compiled\": false,\n",
            "    \"active_run\": false,\n",
            "    \"cycles_burned_by_this_wasm\": 0,\n",
            "    \"ordinary_query_execution_consumes_cycles\": true\n",
            "  }},\n",
            "  \"waveform\": {{\n",
            "    \"status\": \"proposed_unqualified\",\n",
            "    \"point_count\": {},\n",
            "    \"hypothetical_run_duration_ns\": {},\n",
            "    \"proposed_dashboard_total_minimum_cycles_per_second\": {},\n",
            "    \"proposed_dashboard_total_maximum_cycles_per_second\": {},\n",
            "    \"csv_sha256\": \"{}\",\n",
            "    \"trace_sha256\": \"{}\"\n",
            "  }},\n",
            "  \"dated_global_observation\": {{\n",
            "    \"purpose\": \"orientation_only\",\n",
            "    \"first_sample_timestamp_seconds\": 1786812500,\n",
            "    \"last_sample_timestamp_seconds\": 1786898900,\n",
            "    \"requested_step_seconds\": 100,\n",
            "    \"point_count\": 865,\n",
            "    \"minimum_cycles_per_second\": 31671060640.008118,\n",
            "    \"maximum_cycles_per_second\": 49918117789.45853\n",
            "  }},\n",
            "  \"presentation\": {{\n",
            "    \"raster_images_served\": false,\n",
            "    \"image_pipeline_present\": false\n",
            "  }}\n",
            "}}\n"
        ),
        escape_json(canister_id),
        WAVEFORM_POINT_COUNT,
        RUN_DURATION_NS,
        WAVEFORM_MIN_RATE,
        WAVEFORM_MAX_RATE,
        CSV_SHA256,
        TRACE_SHA256,
    )
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
