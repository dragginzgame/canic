//! Loopback-only viewer for one retained report; no directory or journal serving.
use canic_host::{Error, Result};
use std::{
    fs::File,
    io::Read as _,
    net::Ipv4Addr,
    path::Path,
    process::{Command, Stdio},
};
use tiny_http::{Header, Method, Request, Response, Server};

#[cfg(test)]
mod tests;

pub(crate) const MAX_REPORT_BYTES: usize = 4 * 1024 * 1024;

fn report(path: &Path) -> Result<Vec<u8>> {
    let file = File::open(path).map_err(|error| Error::Supplier(Box::new(error)))?;
    if !file
        .metadata()
        .map_err(|error| Error::Supplier(Box::new(error)))?
        .is_file()
    {
        return Err(Error::Invalid(
            "viewer requires a regular HTML report".into(),
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAX_REPORT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| Error::Supplier(Box::new(error)))?;
    if bytes.len() > MAX_REPORT_BYTES || std::str::from_utf8(&bytes).is_err() {
        return Err(Error::Invalid(
            "report must be UTF-8 and at most 4 MiB".into(),
        ));
    }
    Ok(bytes)
}

pub(crate) fn serve(path: &Path, port: u16, open: bool) -> Result<()> {
    let html = report(path)?;
    let viewer = Viewer::new(port, open, "Saved snapshot")?;
    for request in viewer.server.incoming_requests() {
        // Browser disconnects do not tear down the review session.
        if let Err(error) = respond(request, &html, viewer.port) {
            eprintln!("Viewer response interrupted: {error}");
        }
    }
    Ok(())
}

/// One loopback endpoint shared by saved and continuously observed reports.
pub(crate) struct Viewer {
    server: Server,
    port: u16,
}

impl Viewer {
    pub(crate) fn new(port: u16, open: bool, mode: &str) -> Result<Self> {
        let server = Server::http((Ipv4Addr::LOCALHOST, port)).map_err(Error::Supplier)?;
        let port = server.server_addr().to_ip().expect("TCP viewer").port();
        let url = format!("http://localhost:{port}/");
        eprintln!(
            "Fleet viewer: {url}\n{mode}; keep this terminal running. Ctrl-C stops the viewer (PID {}).",
            std::process::id()
        );
        if open {
            let launcher = if cfg!(target_os = "macos") {
                "open"
            } else {
                "xdg-open"
            };
            if let Err(error) = launch(&url, Command::new(launcher)) {
                eprintln!(
                    "Browser launch failed: {error}. Open {url} manually; the viewer is still running."
                );
            }
        }
        Ok(Self { server, port })
    }

    /// Drain a bounded set of queued requests without waiting for a new request.
    pub(crate) fn poll(&self, html: &[u8]) -> Result<()> {
        if html.len() > MAX_REPORT_BYTES {
            return Err(Error::Invalid("live report exceeds 4 MiB".into()));
        }
        for _ in 0..8 {
            let Some(request) = self
                .server
                .try_recv()
                .map_err(|error| Error::Supplier(Box::new(error)))?
            else {
                break;
            };
            if let Err(error) = respond(request, html, self.port) {
                eprintln!("Viewer response interrupted: {error}");
            }
        }
        Ok(())
    }
}

fn respond(request: Request, html: &[u8], port: u16) -> std::io::Result<()> {
    let local_host = format!("localhost:{port}");
    let local_ip = format!("127.0.0.1:{port}");
    let host = request
        .headers()
        .iter()
        .find(|header| header.field.equiv("Host"));
    let (status, body, content_type) = if !host.is_some_and(|header| {
        header.value.as_str() == local_host || header.value.as_str() == local_ip
    }) {
        (
            421,
            b"Local host required".as_slice(),
            "text/plain; charset=utf-8",
        )
    } else if !matches!(request.method(), Method::Get | Method::Head) {
        (
            405,
            b"Read-only viewer".as_slice(),
            "text/plain; charset=utf-8",
        )
    } else if !matches!(request.url(), "/" | "/index.html") {
        (404, b"Not found".as_slice(), "text/plain; charset=utf-8")
    } else {
        (200, html, "text/html; charset=utf-8")
    };
    let response = Response::from_data(body.to_vec())
        .with_status_code(status)
        .with_header(Header::from_bytes("Content-Type", content_type).expect("constant header"))
        .with_header(Header::from_bytes("Cache-Control", "no-store").expect("constant header"))
        .with_header(
            Header::from_bytes("X-Content-Type-Options", "nosniff").expect("constant header"),
        )
        .with_header(
            Header::from_bytes("Cross-Origin-Resource-Policy", "same-origin")
                .expect("constant header"),
        );
    request.respond(response)
}

pub(crate) fn launch(url: &str, mut command: Command) -> std::io::Result<()> {
    command
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    let mut child = ic_host_process::child::OwnedChild::spawn_direct(&mut command)?;
    if let Some(status) = child.relinquish()?
        && !status.success()
    {
        return Err(std::io::Error::other(format!(
            "desktop opener exited with {status}"
        )));
    }
    Ok(())
}
