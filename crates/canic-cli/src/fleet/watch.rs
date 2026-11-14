//! One read-only collector at a time, independent of browser request frequency.
use super::{declared_routes, escape, render};
use crate::view::{MAX_REPORT_BYTES, Viewer};
use canic_host::{Error, Fleet, Result};
use std::{future::Future, path::Path, time::Duration};

#[cfg(test)]
mod tests;

const INTERVAL: Duration = Duration::from_secs(30);

pub(crate) async fn collect(
    root: &Path,
    managers: &[String],
    wallet: Option<&str>,
) -> Result<Fleet> {
    let wallet = if let Some(principal) = wallet {
        Some(canic_host::wallet(principal).await?)
    } else {
        None
    };
    let mut snapshot = canic_host::fleet_with_managers(root, managers).await?;
    snapshot.wallet = wallet;
    Ok(snapshot)
}

fn report(snapshot: &Fleet, links: &[String]) -> Result<String> {
    let html = render(snapshot, &declared_routes(snapshot, links)?);
    // Leave bounded space for the live status and an escaped diagnostic.
    if html.len() > MAX_REPORT_BYTES - 8192 {
        return Err(Error::Invalid(
            "live fleet report exceeds its byte budget".into(),
        ));
    }
    Ok(html)
}

struct Page {
    last: String,
    current: String,
    revision: u64,
}

impl Page {
    fn new(html: String) -> Self {
        let mut page = Self {
            last: html,
            current: String::new(),
            revision: 0,
        };
        page.decorate(None);
        page
    }

    fn update(&mut self, result: Result<String>) {
        match result {
            Ok(html) => {
                self.last = html;
                self.decorate(None);
            }
            Err(error) => self.decorate(Some(
                &error.to_string().chars().take(500).collect::<String>(),
            )),
        }
    }

    fn decorate(&mut self, error: Option<&str>) {
        self.revision = self.revision.saturating_add(1);
        let seconds = INTERVAL.as_secs();
        let class = if error.is_some() { "warning" } else { "scope" };
        let notice = error.map_or_else(
            || "Automatic read-only observations are active.".into(),
            |error| {
                format!(
                    "<strong>Refresh failed — showing the last completed snapshot.</strong> {}",
                    escape(error)
                )
            },
        );
        let status = format!(
            "<aside id=watch-status class={class} data-revision=\"{}\"><strong>Live fleet</strong><p>{notice}</p><p>Checks start {seconds} seconds after each completed check. The page reloads every {seconds} seconds. Partial observations remain marked unknown. Ctrl-C in the viewer terminal stops monitoring.</p></aside>",
            self.revision
        );
        self.current = self
            .last
            .replacen("<main>", &format!("<main>{status}"), 1)
            .replacen(
                "</head>",
                &format!("<meta http-equiv=refresh content={seconds}></head>"),
                1,
            )
            .replacen(
                "Regenerate the report to refresh.",
                "Automatic observation; timestamps show when each result was sampled.",
                1,
            );
    }
}

async fn refresh_while_serving<F>(viewer: &Viewer, html: &str, refresh: F) -> Result<Result<String>>
where
    F: Future<Output = Result<String>>,
{
    tokio::pin!(refresh);
    loop {
        tokio::select! {
            result = &mut refresh => return Ok(result),
            () = tokio::time::sleep(Duration::from_millis(100)) => viewer.poll(html.as_bytes())?,
        }
    }
}

pub(crate) async fn watch(
    root: &Path,
    managers: &[String],
    wallet: Option<&str>,
    links: &[String],
    initial: &Fleet,
    port: u16,
    open: bool,
) -> Result<()> {
    let mut page = Page::new(report(initial, links)?);
    let viewer = Viewer::new(port, open, "Automatic read-only checks")?;
    loop {
        // Waiting is part of this single future: no overlapping scans or catch-up bursts.
        let next = async {
            tokio::time::sleep(INTERVAL).await;
            report(&collect(root, managers, wallet).await?, links)
        };
        let result = refresh_while_serving(&viewer, &page.current, next).await?;
        if let Err(error) = &result {
            eprintln!("Fleet refresh failed; last completed snapshot retained: {error}");
        }
        page.update(result);
    }
}
