//! Curated public projection, escaped HTML and framework-neutral HTTP responses.

use crate::observatory::{ObservatoryError, model::ObservatoryProfile, policy, view::*};
use std::io::{self, Write};

/// Strip private Fleet authority before passing data to a downstream renderer.
pub fn public_view(
    snapshot: &ObservatorySnapshotView,
    profile: &ObservatoryProfile,
    now_ms: u64,
) -> Result<PublicObservatoryView, ObservatoryError> {
    policy::validate_profile(profile)?;
    Ok(PublicObservatoryView {
        schema_version: 1,
        title: profile.title.clone(),
        collected_at_unix_ms: snapshot.collected_at_unix_ms,
        freshness_secs: snapshot.freshness_secs,
        authority_available: matches!(&snapshot.authority, Observation::Observed { observed_at_unix_ms, .. } if policy::is_fresh(*observed_at_unix_ms, now_ms, snapshot.freshness_secs)),
        roles: snapshot
            .roles
            .iter()
            .enumerate()
            .map(|(key, role)| PublicObservatoryRoleView {
                key,
                label: profile
                    .role_labels
                    .get(&role.role)
                    .unwrap_or(&role.role)
                    .clone(),
                role: role.role.clone(),
                overview: fresh(&role.overview, now_ms, snapshot.freshness_secs),
                store: fresh(&role.store, now_ms, snapshot.freshness_secs),
            })
            .collect(),
    })
}

fn fresh<T: Clone>(observation: &Observation<T>, now: u64, ttl: u32) -> Observation<T> {
    match observation {
        Observation::Observed {
            observed_at_unix_ms,
            ..
        } if !policy::is_fresh(*observed_at_unix_ms, now, ttl) => Observation::Unavailable {
            observed_at_unix_ms: *observed_at_unix_ms,
            failure: ObservationFailure::Stale,
        },
        _ => observation.clone(),
    }
}

/// Serialize through a finite writer, including private operator JSON when explicitly selected.
pub fn json_bytes(
    value: &impl serde::Serialize,
    maximum_bytes: usize,
) -> Result<Vec<u8>, ObservatoryError> {
    let mut writer = LimitedWriter {
        bytes: Vec::new(),
        maximum: maximum_bytes,
    };
    if let Err(error) = serde_json::to_writer_pretty(&mut writer, value) {
        if error.is_io() {
            return Err(ObservatoryError::Bound("rendered bytes"));
        }
        return Err(error.into());
    }
    Ok(writer.bytes)
}

/// Serve only curated public data. The caller owns authentication, scheduling and the server.
pub fn http_response(
    snapshot: &ObservatorySnapshotView,
    profile: &ObservatoryProfile,
    path: &str,
    now_ms: u64,
    maximum_bytes: usize,
) -> Result<ObservatoryHttpView, ObservatoryError> {
    let public = public_view(snapshot, profile, now_ms)?;
    let (status, content_type, body) = match path {
        "/snapshot.json" => (
            200,
            "application/json; charset=utf-8",
            json_bytes(&public, maximum_bytes)?,
        ),
        "/" => (
            200,
            "text/html; charset=utf-8",
            html(&public, maximum_bytes)?,
        ),
        _ => (404, "text/plain; charset=utf-8", b"Not found".to_vec()),
    };
    if body.len() > maximum_bytes {
        return Err(ObservatoryError::Bound("rendered bytes"));
    }
    Ok(ObservatoryHttpView {
        status,
        content_type,
        cache_control: "no-store",
        content_security_policy: "default-src 'none'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'",
        body,
    })
}

fn html(public: &PublicObservatoryView, maximum: usize) -> Result<Vec<u8>, ObservatoryError> {
    let json = json_bytes(public, maximum)?;
    let mut writer = LimitedWriter {
        bytes: Vec::new(),
        maximum,
    };
    writer.write_all(b"<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><title>Fleet observatory</title><body><h1>").map_err(render_bound)?;
    escaped(&mut writer, public.title.as_bytes())?;
    writer.write_all(b"</h1><p>Each observation records its source and time. Unavailable values are unknown.</p>").map_err(render_bound)?;
    writer.write_all(b"<table><caption>Role observations</caption><tr><th>Label</th><th>Role</th><th>Bootstrap</th><th>Store bytes</th></tr>").map_err(render_bound)?;
    for role in &public.roles {
        writer.write_all(b"<tr>").map_err(render_bound)?;
        let readiness = match &role.overview {
            Observation::Observed { value, .. } if value.bootstrap_ready => "ready",
            Observation::Observed { .. } => "not ready",
            Observation::Unavailable { .. } => "unavailable",
        };
        let occupancy = match &role.store {
            Observation::Observed { value, .. } => format!(
                "{} occupied / {} maximum",
                value.occupied_bytes, value.maximum_bytes
            ),
            Observation::Unavailable { .. } => "unavailable".into(),
        };
        for value in [&role.label, &role.role, readiness, &occupancy] {
            writer.write_all(b"<td>").map_err(render_bound)?;
            escaped(&mut writer, value.as_bytes())?;
            writer.write_all(b"</td>").map_err(render_bound)?;
        }
        writer.write_all(b"</tr>").map_err(render_bound)?;
    }
    writer.write_all(b"</table><details><summary>Sources, times and complete public observations</summary><pre>").map_err(render_bound)?;
    escaped(&mut writer, &json)?;
    writer
        .write_all(b"</pre></details></body></html>")
        .map_err(render_bound)?;
    Ok(writer.bytes)
}

fn escaped(writer: &mut LimitedWriter, bytes: &[u8]) -> Result<(), ObservatoryError> {
    for byte in bytes {
        let replacement = match byte {
            b'&' => b"&amp;".as_slice(),
            b'<' => b"&lt;",
            b'>' => b"&gt;",
            b'\"' => b"&quot;",
            b'\'' => b"&#39;",
            _ => std::slice::from_ref(byte),
        };
        writer.write_all(replacement).map_err(render_bound)?;
    }
    Ok(())
}

fn render_bound(_: io::Error) -> ObservatoryError {
    ObservatoryError::Bound("rendered bytes")
}

struct LimitedWriter {
    bytes: Vec<u8>,
    maximum: usize,
}
impl Write for LimitedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.maximum.saturating_sub(self.bytes.len()) {
            return Err(io::Error::other("render budget exceeded"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
