//! Module: skynet_console::render
//!
//! Responsibility: render one sanitized Skynet observation as a responsive HTML console.
//! Does not own: HTTP routing, runtime observation, JSON serialization, or access policy.
//! Boundary: consumes only framework-independent presentation models.

use crate::{
    CanisterNode, Capability, ConsoleSnapshot, Endpoint, Fact, MetricRow, NetworkMember,
    NetworkRoot, NetworkService,
};
use std::fmt::{self, Write};

const STYLE: &str = r#"
:root{color-scheme:dark;font-family:"IBM Plex Mono","SFMono-Regular",Consolas,monospace;--void:#030405;--panel:#0a0d0e;--line:#243033;--red:#f23838;--red-dim:#7e2020;--green:#8cff9b;--text:#d7dedc;--muted:#778481}
*{box-sizing:border-box}html{background:var(--void)}body{min-height:100vh;margin:0;background:radial-gradient(circle at 50% -10%,#311010 0,transparent 35rem),linear-gradient(#050708e8,#030405),repeating-linear-gradient(0deg,#ffffff05 0,#ffffff05 1px,transparent 1px,transparent 4px);color:var(--text)}
body:after{position:fixed;inset:0;z-index:10;pointer-events:none;content:"";background:linear-gradient(90deg,#ff000008,transparent 30%,#00ff1a05 65%,#0000ff08);mix-blend-mode:screen}
a{color:var(--green);text-decoration:none}a:hover{text-decoration:underline}.shell{width:min(96rem,100%);margin:auto;padding:1.25rem}.top{display:flex;justify-content:space-between;gap:2rem;align-items:flex-start;padding:1.3rem 0 1.5rem;border-bottom:1px solid var(--red-dim)}
.kicker,.label{color:var(--red);font-size:.68rem;font-weight:800;letter-spacing:.16em;text-transform:uppercase}.kicker{margin:0 0 .5rem}.top h1{margin:0;font:800 clamp(1.8rem,5vw,4.4rem)/.9 system-ui,sans-serif;letter-spacing:-.07em;text-transform:uppercase}.top h1 span{color:var(--red)}.subtitle{max-width:55rem;margin:.7rem 0 0;color:#9eaaa7;font-size:.82rem;line-height:1.55}
.signal{min-width:14rem;padding:.75rem 1rem;border:1px solid #36503b;background:#08130b;color:var(--green);font-size:.72rem;text-align:right;box-shadow:inset 0 0 1.2rem #00ff1a0d}.signal b{display:block;margin-bottom:.25rem;font-size:1rem}.signal.offline{border-color:#6b2929;background:#180909;color:#ff7777}
.status-grid{display:grid;grid-template-columns:repeat(6,1fr);gap:.65rem;margin:1rem 0}.stat,.panel{border:1px solid var(--line);background:linear-gradient(145deg,#0c1011ee,#070909ee);box-shadow:0 .8rem 2rem #0008}.stat{min-height:6.3rem;padding:.8rem}.stat strong{display:block;margin:.55rem 0 .35rem;color:#f2f5f4;font:700 clamp(1rem,2vw,1.55rem)/1.05 system-ui,sans-serif;overflow-wrap:anywhere}.stat small{color:var(--muted);font-size:.64rem}.ok{color:var(--green)!important}.warn{color:#ff7979!important}
.grid{display:grid;grid-template-columns:1.35fr .65fr;gap:.8rem}.panel{margin-bottom:.8rem;overflow:hidden}.panel>header{display:flex;justify-content:space-between;gap:1rem;align-items:center;padding:.75rem .9rem;border-bottom:1px solid var(--line);background:#0f1415}.panel h2{margin:0;font:800 .77rem system-ui,sans-serif;letter-spacing:.08em;text-transform:uppercase}.panel header span{color:var(--muted);font-size:.62rem}.body{padding:.9rem}
.root-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(14rem,1fr));gap:.6rem}.root{position:relative;padding:.7rem;border:1px solid #263234;background:#070a0b}.root.current{border-color:var(--red);box-shadow:inset 0 0 1.5rem #ff000014}.root:before{position:absolute;top:-.3rem;left:1rem;width:.6rem;height:.6rem;border-radius:50%;background:var(--green);content:"";box-shadow:0 0 .8rem var(--green)}.root strong,.member strong{display:block;margin:.3rem 0;font-size:.75rem;overflow-wrap:anywhere}.root small,.member small{color:var(--muted);font-size:.58rem}.service{margin-top:.75rem;padding-top:.75rem;border-top:1px dashed #283436}.service:first-child{margin-top:0;padding-top:0;border-top:0}.service-head{display:flex;justify-content:space-between;gap:1rem;margin-bottom:.6rem}.service-head strong{color:#fff;font-size:.78rem}.service-head span{color:var(--red);font-size:.62rem}.members{display:grid;grid-template-columns:repeat(auto-fit,minmax(13rem,1fr));gap:.45rem}.member{padding:.6rem;border-left:.18rem solid #4a5553;background:#080b0c}.member.authority{border-color:var(--red)}.member.current{outline:1px solid var(--green)}
.facts{display:grid;grid-template-columns:repeat(2,1fr);gap:.45rem}.fact{padding:.6rem;border:1px solid #202a2c;background:#07090a}.fact strong{display:block;margin-top:.3rem;color:#e9efed;font-size:.72rem;overflow-wrap:anywhere}.fact small{color:var(--muted);font-size:.58rem}.capabilities{display:grid;gap:.45rem}.cap{padding:.65rem;border-left:.18rem solid var(--green);background:#070a08}.cap.disabled{border-color:#5a6160}.cap strong{font-size:.7rem}.cap p{margin:.28rem 0 0;color:#899491;font-size:.62rem;line-height:1.45}
.table-wrap{overflow:auto}table{width:100%;border-collapse:collapse;font-size:.66rem}th,td{padding:.62rem .75rem;border-bottom:1px solid #1d2527;text-align:left;vertical-align:top}th{position:sticky;top:0;background:#0e1213;color:var(--red);font-size:.58rem;letter-spacing:.08em;text-transform:uppercase}td{overflow-wrap:anywhere}tr:last-child td{border-bottom:0}.pill{display:inline-block;padding:.13rem .35rem;border:1px solid #394446;color:#b9c2c0;font-size:.55rem;text-transform:uppercase}.terminal{margin:0;padding:.8rem;overflow:auto;background:#020303;color:#a4ffae;font-size:.64rem;line-height:1.55;white-space:pre-wrap}.empty{padding:1.2rem;border:1px dashed #3a4241;color:var(--muted);font-size:.7rem;text-align:center}
.actions{display:flex;flex-wrap:wrap;gap:.5rem;margin:0 0 .8rem}.button{padding:.55rem .72rem;border:1px solid var(--red-dim);background:#190909;color:#ff9a9a;font-size:.63rem;text-transform:uppercase}.button:hover{border-color:var(--red);text-decoration:none}footer{display:flex;justify-content:space-between;gap:1rem;padding:1rem 0;color:#5f6a68;font-size:.58rem}
@media(max-width:950px){.status-grid{grid-template-columns:repeat(3,1fr)}.grid{grid-template-columns:1fr}}@media(max-width:580px){.shell{padding:.75rem}.top{flex-direction:column}.signal{width:100%;text-align:left}.status-grid{grid-template-columns:repeat(2,1fr)}.facts{grid-template-columns:1fr}footer{flex-direction:column}}
"#;

pub fn page(snapshot: &ConsoleSnapshot) -> String {
    let mut output = String::with_capacity(48 * 1024);
    append(
        &mut output,
        format_args!(
            "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta name=\"theme-color\" content=\"#050607\"><title>{} · Skynet Fleet Console</title><style>{STYLE}</style></head><body><main class=\"shell\">",
            escape(&snapshot.identity.codename)
        ),
    );
    render_header(&mut output, snapshot);
    render_status(&mut output, snapshot);
    output.push_str("<div class=\"actions\"><a class=\"button\" href=\"/api/status.json\">Raw JSON</a><a class=\"button\" href=\"/\">Refresh observation</a></div>");
    output.push_str("<div class=\"grid\"><div>");
    render_network(&mut output, snapshot);
    render_endpoints(&mut output, &snapshot.endpoints);
    render_metrics(&mut output, &snapshot.metrics);
    output.push_str("</div><aside>");
    render_facts(&mut output, "Environment matrix", &snapshot.environment);
    render_facts(&mut output, "Deployment parameters", &snapshot.deployment);
    render_capabilities(&mut output, &snapshot.capabilities);
    render_children(&mut output, &snapshot.children);
    output.push_str("</aside></div>");
    render_footer(&mut output, snapshot);
    output.push_str("</main></body></html>");
    output
}

fn render_header(output: &mut String, snapshot: &ConsoleSnapshot) {
    let signal_class = if snapshot.runtime.ready {
        "signal"
    } else {
        "signal offline"
    };
    let signal = if snapshot.runtime.ready {
        "ONLINE"
    } else {
        "BOOTSTRAPPING"
    };
    append(
        output,
        format_args!(
            "<header class=\"top\"><div><p class=\"kicker\">SKYNET // CANIC FLEET OBSERVATORY</p><h1>{} <span>console</span></h1><p class=\"subtitle\">Live read-only projection of this canister, its protected deployment context, Canic endpoint surface, metrics, descendants, physical Subnets, and published Fleet services.</p></div><div class=\"{signal_class}\"><b>{signal}</b>{}<br>{}</div></header>",
            escape(&snapshot.identity.codename),
            escape(&snapshot.identity.role),
            short_id(&snapshot.identity.canister_id)
        ),
    );
}

fn render_status(output: &mut String, snapshot: &ConsoleSnapshot) {
    let member_count = snapshot
        .network
        .services
        .iter()
        .map(|service| service.members.len())
        .sum::<usize>();
    output.push_str("<section class=\"status-grid\" aria-label=\"Runtime summary\">");
    stat(
        output,
        "Runtime",
        &snapshot.runtime.phase,
        &snapshot.runtime.observation,
    );
    stat(
        output,
        "Cycle reserve",
        &format_cycles(snapshot.runtime.cycles),
        "live balance",
    );
    stat(
        output,
        "Fleet roots",
        &snapshot.network.roots.len().to_string(),
        "physical Subnets",
    );
    stat(
        output,
        "Service nodes",
        &member_count.to_string(),
        "published members",
    );
    stat(
        output,
        "Local children",
        &snapshot.children.len().to_string(),
        "direct descendants",
    );
    stat(
        output,
        "Registry revision",
        &snapshot
            .network
            .registry_revision
            .map_or_else(|| "n/a".to_string(), |revision| revision.to_string()),
        &snapshot.network.authority,
    );
    output.push_str("</section>");
}

fn stat(output: &mut String, label: &str, value: &str, detail: &str) {
    append(
        output,
        format_args!(
            "<article class=\"stat\"><span class=\"label\">{}</span><strong>{}</strong><small>{}</small></article>",
            escape(label),
            escape(value),
            escape(detail)
        ),
    );
}

fn render_network(output: &mut String, snapshot: &ConsoleSnapshot) {
    panel_start(
        output,
        "Global neural-net topology",
        &format!(
            "{} roots · {} services",
            snapshot.network.roots.len(),
            snapshot.network.services.len()
        ),
    );
    if snapshot.network.roots.is_empty() {
        append(
            output,
            format_args!(
                "<div class=\"empty\">Fleet Directory unavailable in this runtime: {}</div>",
                escape(&snapshot.network.authority)
            ),
        );
    } else {
        output.push_str("<div class=\"root-grid\">");
        for root in &snapshot.network.roots {
            render_root(output, root);
        }
        output.push_str("</div>");
    }
    if !snapshot.network.services.is_empty() {
        output.push_str("<div style=\"margin-top:1rem\">");
        for service in &snapshot.network.services {
            render_service(output, service);
        }
        output.push_str("</div>");
    }
    panel_end(output);
}

fn render_root(output: &mut String, root: &NetworkRoot) {
    let current = if root.current { " current" } else { "" };
    append(
        output,
        format_args!(
            "<a class=\"root{current}\" href=\"{}\"><span class=\"label\">{} subnet</span><strong>{}</strong><small>root {} · {}</small></a>",
            escape(&root.url),
            escape(&root.status),
            escape(&root.subnet_id),
            short_id(&root.root_canister_id),
            if root.current { "local" } else { "remote" }
        ),
    );
}

fn render_service(output: &mut String, service: &NetworkService) {
    append(
        output,
        format_args!(
            "<section class=\"service\"><div class=\"service-head\"><strong>{}</strong><span>{} · {} members · ≤ {}/root · spread ≥ {}</span></div><div class=\"members\">",
            escape(&service.service),
            escape(&service.mode),
            service.members.len(),
            service.maximum_members_per_root,
            service.minimum_distinct_roots
        ),
    );
    for member in &service.members {
        render_member(output, member);
    }
    output.push_str("</div></section>");
}

fn render_member(output: &mut String, member: &NetworkMember) {
    let authority = if member.purpose.eq_ignore_ascii_case("authority") {
        " authority"
    } else {
        ""
    };
    let current = if member.current { " current" } else { "" };
    append(
        output,
        format_args!(
            "<a class=\"member{authority}{current}\" href=\"{}\"><span class=\"label\">{}</span><strong>{}</strong><small>{} · root {}</small></a>",
            escape(&member.url),
            escape(&member.purpose),
            escape(&member.canister_id),
            escape(&member.placement),
            short_id(&member.root_canister_id)
        ),
    );
}

fn render_facts(output: &mut String, title: &str, facts: &[Fact]) {
    panel_start(output, title, &format!("{} observed fields", facts.len()));
    if facts.is_empty() {
        output.push_str("<div class=\"empty\">No fields published by this role.</div>");
    } else {
        output.push_str("<div class=\"facts\">");
        for fact in facts {
            fact_card(output, fact);
        }
        output.push_str("</div>");
    }
    panel_end(output);
}

fn fact_card(output: &mut String, fact: &Fact) {
    append(
        output,
        format_args!(
            "<div class=\"fact\"><span class=\"label\">{}</span><strong>{}</strong><small>{}</small></div>",
            escape(&fact.name),
            escape(&fact.value),
            escape(&fact.source)
        ),
    );
}

fn render_capabilities(output: &mut String, capabilities: &[Capability]) {
    panel_start(
        output,
        "Capability matrix",
        &format!("{} compile/config/runtime checks", capabilities.len()),
    );
    output.push_str("<div class=\"capabilities\">");
    for capability in capabilities {
        let disabled = if capability.status.eq_ignore_ascii_case("disabled") {
            " disabled"
        } else {
            ""
        };
        append(
            output,
            format_args!(
                "<div class=\"cap{disabled}\"><span class=\"label\">{}</span><strong>{}</strong><p>{}</p></div>",
                escape(&capability.status),
                escape(&capability.name),
                escape(&capability.detail)
            ),
        );
    }
    output.push_str("</div>");
    panel_end(output);
}

fn render_children(output: &mut String, children: &[CanisterNode]) {
    panel_start(
        output,
        "Direct descendants",
        &format!("{} current", children.len()),
    );
    if children.is_empty() {
        output.push_str("<div class=\"empty\">No direct child registered at this node.</div>");
    } else {
        output.push_str("<div class=\"capabilities\">");
        for child in children {
            append(
                output,
                format_args!(
                    "<a class=\"cap\" href=\"{}\"><span class=\"label\">{}</span><strong>{}</strong><p>{}</p></a>",
                    escape(&child.url),
                    escape(&child.relation),
                    escape(&child.role),
                    escape(&child.canister_id)
                ),
            );
        }
        output.push_str("</div>");
    }
    panel_end(output);
}

fn render_endpoints(output: &mut String, endpoints: &[Endpoint]) {
    panel_start(
        output,
        "Candid endpoint highlights",
        &format!("{} representative methods", endpoints.len()),
    );
    output.push_str("<div class=\"table-wrap\"><table><thead><tr><th>Method</th><th>Mode</th><th>Access</th><th>Purpose</th></tr></thead><tbody>");
    for endpoint in endpoints {
        append(
            output,
            format_args!(
                "<tr><td>{}</td><td><span class=\"pill\">{}</span></td><td>{}</td><td>{}</td></tr>",
                escape(&endpoint.name),
                escape(&endpoint.mode),
                escape(&endpoint.access),
                escape(&endpoint.purpose)
            ),
        );
    }
    output.push_str("</tbody></table></div>");
    panel_end(output);
}

fn render_metrics(output: &mut String, metrics: &[MetricRow]) {
    panel_start(
        output,
        "Canic metrics",
        &format!("{} current rows", metrics.len()),
    );
    if metrics.is_empty() {
        output.push_str("<div class=\"empty\">This tier has no recorded samples yet.</div>");
    } else {
        output.push_str("<div class=\"table-wrap\"><table><thead><tr><th>Tier</th><th>Labels</th><th>Principal</th><th>Value</th></tr></thead><tbody>");
        for metric in metrics {
            render_metric(output, metric);
        }
        output.push_str("</tbody></table></div>");
    }
    panel_end(output);
}

fn render_metric(output: &mut String, metric: &MetricRow) {
    append(
        output,
        format_args!(
            "<tr><td><span class=\"pill\">{}</span></td><td>{}</td><td>{}</td><td>{}</td></tr>",
            escape(&metric.tier),
            escape(&metric.labels.join(" / ")),
            escape(metric.principal.as_deref().unwrap_or("—")),
            escape(&metric.value)
        ),
    );
}

fn render_footer(output: &mut String, snapshot: &ConsoleSnapshot) {
    append(
        output,
        format_args!(
            "<footer><span>Model {} · Canic {} · package {} {}</span><span>Observation {} ns · <a href=\"https://{}.raw.icp0.io/\">canonical console</a></span></footer>",
            snapshot.schema_version,
            escape(&snapshot.identity.canic_version),
            escape(&snapshot.identity.package_name),
            escape(&snapshot.identity.package_version),
            snapshot.generated_at_ns,
            escape(&snapshot.identity.canister_id)
        ),
    );
}

fn panel_start(output: &mut String, title: &str, detail: &str) {
    append(
        output,
        format_args!(
            "<section class=\"panel\"><header><h2>{}</h2><span>{}</span></header><div class=\"body\">",
            escape(title),
            escape(detail)
        ),
    );
}

fn panel_end(output: &mut String) {
    output.push_str("</div></section>");
}

fn append(output: &mut String, args: fmt::Arguments<'_>) {
    output
        .write_fmt(args)
        .expect("writing formatted text to a String cannot fail");
}

fn escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn short_id(value: &str) -> String {
    if value.chars().count() <= 18 {
        return escape(value);
    }
    let prefix = value.chars().take(8).collect::<String>();
    let suffix = value
        .chars()
        .rev()
        .take(6)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    escape(&format!("{prefix}…{suffix}"))
}

fn format_cycles(cycles: u128) -> String {
    if cycles >= 1_000_000_000_000 {
        let whole = cycles / 1_000_000_000_000;
        let tenths = (cycles % 1_000_000_000_000) / 100_000_000_000;
        format!("{whole}.{tenths}T")
    } else if cycles >= 1_000_000_000 {
        format!("{}B", cycles / 1_000_000_000)
    } else {
        cycles.to_string()
    }
}
