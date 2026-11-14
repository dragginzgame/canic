use canic_host::{Error, Fleet, Result};
use ic_host_fs::durable::{self, PublicationMode, WriteOptions};
use std::{fmt::Write as _, num::NonZeroUsize, path::Path};

#[cfg(test)]
mod tests;
mod watch;
pub(crate) use watch::{collect, watch};

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn declared_routes(snapshot: &Fleet, links: &[String]) -> Result<String> {
    if links.is_empty() {
        return Ok(String::new());
    }
    if links.len() > 32 {
        return Err(Error::Invalid("at most 32 declared routes".into()));
    }
    let mut html = String::from(
        "<h2>Declared application routes</h2><p>Operator-supplied topology, not an on-chain route or readiness observation. Each endpoint is known in this journal under the same network trust binding.</p><div class=grid>",
    );
    for link in links {
        let (entry, worker) = link
            .split_once(':')
            .ok_or_else(|| Error::Invalid("route must be ENTRY:WORKER".into()))?;
        let candidates: Vec<_> = snapshot
            .canisters
            .iter()
            .filter(|source| {
                source.canister == entry
                    && entry != worker
                    && snapshot.canisters.iter().any(|target| {
                        target.canister == worker && target.network_id == source.network_id
                    })
            })
            .collect();
        if candidates.len() != 1 {
            return Err(Error::Invalid("declared route requires two distinct known targets on one unambiguous network; select a smaller state directory".into()));
        }
        write!(html, "<article><span class=badge>Declared · not verified</span><p>{}</p><div class=topology><span class=label>Profile entry</span><code>{}</code><span class=arrow>↓ discovers / checks</span><span class=label>Profile worker · owns private data</span><code>{}</code></div></article>", escape(&candidates[0].network), escape(entry), escape(worker)).expect("writing to a String");
    }
    html.push_str("</div>");
    Ok(html)
}

fn render(snapshot: &Fleet, routes: &str) -> String {
    let mut html = include_str!("header.html").to_owned();
    write!(html,
        "<header><p class=eyebrow>CANIC / FLEET OBSERVATION</p><h1>Your deployment fleet</h1><p>Snapshot collected <time data-ns=\"{}\">{} Unix seconds</time>. Regenerate the report to refresh.</p></header><div class=summary><strong>{} canisters</strong><strong>{} operations</strong><strong>{} inventory issues</strong></div><p class=scope>Scope: local deployment journal and explicitly selected observations. Certified module and controller reads are sampled separately. Canister cycle balances and running/stopped status are not observed.</p>",
        snapshot.finished_at_ns, snapshot.finished_at_ns / 1_000_000_000,
        snapshot.canisters.len(), snapshot.operations.len(), snapshot.issues.len()
    ).expect("writing to a String");
    if let Some(wallet) = &snapshot.wallet {
        write!(html, "<h2>Mainnet wallet</h2><article><span class=badge>Uncertified balance queries</span><span class=label>Owner principal · default subaccount</span><code>{}</code><span class=label>Receive ICP · Internet Computer mainnet</span><code>{}</code><p>Wallet balances exclude cycles already held by canisters. This view makes no transfers or conversions.</p><div class=grid>", escape(&wallet.principal), escape(&wallet.icp_address)).expect("writing to String");
        for (label, balance) in [("ICP", &wallet.icp), ("Cycles ledger", &wallet.cycles)] {
            write!(html, "<section><h3>{label}</h3><strong>{}</strong><p class=muted>Observed <time data-ns=\"{}\">{} Unix seconds</time></p>", escape(balance.display.as_deref().unwrap_or("Unknown")), balance.observed_at_ns, balance.observed_at_ns / 1_000_000_000).expect("writing to String");
            if let Some(error) = &balance.observation_error {
                write!(html, "<p class=warning>{}</p>", escape(error)).expect("writing to String");
            }
            html.push_str("</section>");
        }
        html.push_str("</div></article>");
    }
    html.push_str("<h2>Canisters &amp; controllers</h2><div class=grid>");
    if snapshot.canisters.is_empty() {
        html.push_str("<p>No admitted target is known yet. Pending plans remain below.</p>");
    }
    for canister in &snapshot.canisters {
        let observed = canister.observation_error.is_none();
        write!(html, "<article><div class=topline><span>{}</span><span class=\"badge {}\">{}</span></div><h3>{}</h3>",
            escape(&canister.network), if observed {"observed"} else {"unknown"},
            if observed {"Certified metadata"} else {"Unknown / unavailable"}, escape(&canister.canister)
        ).expect("writing to a String");
        if let Some(controllers) = &canister.controllers {
            html.push_str("<div class=topology><span class=label>Observed controllers</span>");
            for controller in controllers {
                write!(html, "<code>{}</code>", escape(controller)).expect("writing to a String");
            }
            if controllers.is_empty() {
                html.push_str("<span>No controllers</span>");
            } else {
                html.push_str("<span class=arrow>↓ controls</span>");
            }
            html.push_str("<strong>Deployment target</strong></div>");
        }
        if let Some(module) = &canister.module_hash {
            write!(
                html,
                "<span class=label>Observed module SHA-256</span><code>{}</code>",
                escape(module)
            )
            .expect("writing to a String");
        }
        if let Some(error) = &canister.observation_error {
            write!(html, "<p class=warning>{}</p><p>No health or effect outcome is inferred from this observation failure.</p>", escape(error)).expect("writing to a String");
        }
        write!(html, "<p class=muted>Observed <time data-ns=\"{}\">{} Unix seconds</time></p><details><summary>Network trust binding</summary><code>{}</code></details></article>",
            canister.observed_at_ns, canister.observed_at_ns / 1_000_000_000, escape(&canister.network_id)
        ).expect("writing to a String");
    }
    html.push_str("</div>");
    html.push_str(routes);
    if !snapshot.managers.is_empty() {
        html.push_str("<h2>Autonomous worker operations</h2><p>Manager journals below are uncertified application reports. Module and controller observations above are independently certified. Observation does not resume or retry work.</p><div class=grid>");
        for manager in &snapshot.managers {
            write!(
                html,
                "<article><span class=badge>Uncertified manager report</span><h3>{}</h3>",
                escape(&manager.manager)
            )
            .expect("writing to String");
            if let Some(status) = &manager.reported {
                write!(html, "<p>Phase: <strong>{:?}</strong></p><p>Create / upload / install attempts: {} / {} / {}</p><div class=topology><code>{}</code><span class=arrow>↓ reports worker</span><code>{}</code></div>", status.phase, status.attempts[0], status.attempts[1], status.attempts[2], escape(&manager.manager), escape(status.worker.as_deref().unwrap_or("No retained principal"))).expect("writing to String");
                if status.phase == canic_host::ProfileGrowthPhase::CreateIssued {
                    html.push_str("<p class=warning>Creation outcome unresolved. The original 1T attachment remains uncertain unless a correlated refund is retained. No second creation is authorized.</p>");
                }
                if let Some(error) = &status.failure {
                    write!(html, "<p class=warning>{error:?}</p>").expect("writing to String");
                }
            }
            if let Some(error) = &manager.observation_error {
                write!(html, "<p class=warning>{}</p>", escape(error)).expect("writing to String");
            }
            html.push_str("</article>");
        }
        html.push_str("</div>");
    }
    html.push_str("<h2>Deployment &amp; recovery operations</h2><p>Receipts are retained historical evidence. An approved plan is not proof of deployment; an old receipt need not match a later upgrade.</p><div class=grid>");
    for operation in &snapshot.operations {
        let observed = snapshot.canisters.iter().find(|item| {
            item.network_id == operation.network_id
                && operation.canister.as_ref() == Some(&item.canister)
        });
        let binding = match observed {
            Some(item) if item.module_hash.is_some() && item.controllers.is_some() => {
                if item.module_hash.as_ref() == Some(&operation.approved_module)
                    && item.controllers.as_ref().is_some_and(|values| {
                        values == std::slice::from_ref(&operation.approved_controller)
                    })
                {
                    "Current metadata matches this approval"
                } else {
                    "Current metadata differs from this approval (historical or changed)"
                }
            }
            _ => "Current binding unknown",
        };
        write!(html, "<article><div class=topline><span>{}</span><span class=badge>{}</span></div><h3>{}</h3><p>{}</p><code>{}</code><p>{}</p><span class=label>Approved module SHA-256</span><code>{}</code><dl>",
            escape(&operation.network), if operation.receipt_retained {"Receipt retained"} else {"Unfinished"},
            escape(&operation.operation), binding,
            escape(operation.canister.as_deref().unwrap_or("Target unknown — creation result not retained")),
            if operation.approval_expired {"Original approval expired; this does not erase retained effects."} else {"Within original approval window."},
            escape(&operation.approved_module)
        ).expect("writing to a String");
        for (effect, evidence) in &operation.effects {
            write!(
                html,
                "<dt>{}</dt><dd>{}</dd>",
                escape(effect),
                escape(match evidence.as_str() {
                    "signed_attempt_retained_outcome_unknown" =>
                        "Signed attempt retained · outcome unknown",
                    "reply_retained" => "Reply retained",
                    "rejection_retained" => "Rejection retained",
                    "not_submitted" => "Not submitted",
                    other => other,
                })
            )
            .expect("writing to a String");
        }
        html.push_str("</dl><p class=muted>Use <code>canic status DIGEST</code> for retained details. This report never resumes or retries an operation.</p></article>");
    }
    html.push_str("</div>");
    if !snapshot.issues.is_empty() {
        html.push_str("<section class=issues><h2>Partial inventory</h2><p>These records were not admitted; the snapshot is incomplete.</p><ul>");
        for issue in &snapshot.issues {
            write!(html, "<li>{}</li>", escape(issue)).expect("writing to a String");
        }
        html.push_str("</ul></section>");
    }
    html.push_str("<footer>Canic · Read-only snapshot · No keys, signed attempts or artifact bytes exported</footer></main><script>document.querySelectorAll('time[data-ns]').forEach(t=>{t.textContent=new Date(Number(t.dataset.ns)/1e6).toISOString()})</script></body></html>");
    html
}

fn write(path: &Path, snapshot: &Fleet, routes: &str) -> Result<()> {
    durable::write_bytes(
        path,
        WriteOptions {
            mode: PublicationMode::CreateNew,
            permissions: 0o600,
        },
        render(snapshot, routes).as_bytes(),
    )
    .map_err(|error| Error::Supplier(Box::new(error)))
}

pub(crate) fn publish(
    snapshot: &Fleet,
    html: Option<&Path>,
    open: bool,
    serve: bool,
    port: u16,
    links: &[String],
) -> Result<()> {
    let routes = declared_routes(snapshot, links)?;
    let automatic;
    let path = if let Some(path) = html {
        path
    } else if open || serve {
        let root = Path::new("target/fleet");
        durable::prepare_private_parents(root).map_err(|error| Error::Supplier(Box::new(error)))?;
        automatic = ic_host_fs::directory::create_unique_directory(
            root,
            "snapshot",
            0o700,
            NonZeroUsize::new(32),
        )
        .map_err(|error| Error::Supplier(Box::new(error)))?
        .join("index.html");
        &automatic
    } else {
        return Ok(());
    };
    write(path, snapshot, &routes)?;
    eprintln!("Fleet report saved: {}", path.display());
    if open || serve {
        crate::view::serve(path, port, open)?;
    }
    Ok(())
}
