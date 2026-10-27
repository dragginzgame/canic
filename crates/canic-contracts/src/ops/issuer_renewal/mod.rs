//! Lossless Root renewal reporting projections.
use crate::dto::{auth as stored, issuer_renewal as native};
impl From<stored::RootIssuerRenewalTemplateView> for native::RootIssuerRenewalTemplateView {
    fn from(value: stored::RootIssuerRenewalTemplateView) -> Self {
        Self {
            issuer_pid: value.issuer_pid.into(),
            enabled: value.enabled,
            aud: value.aud,
            grants: value.grants,
            cert_ttl_ns: value.cert_ttl_ns,
        }
    }
}
impl From<native::RootIssuerRenewalTemplateView> for stored::RootIssuerRenewalTemplateView {
    fn from(value: native::RootIssuerRenewalTemplateView) -> Self {
        Self {
            issuer_pid: value.issuer_pid.into(),
            enabled: value.enabled,
            aud: value.aud,
            grants: value.grants,
            cert_ttl_ns: value.cert_ttl_ns,
        }
    }
}
impl From<stored::RootIssuerRenewalStatusRequest> for native::RootIssuerRenewalStatusRequest {
    fn from(value: stored::RootIssuerRenewalStatusRequest) -> Self {
        Self {
            issuer_pid: value.issuer_pid.into(),
        }
    }
}
impl From<native::RootIssuerRenewalStatusRequest> for stored::RootIssuerRenewalStatusRequest {
    fn from(value: native::RootIssuerRenewalStatusRequest) -> Self {
        Self {
            issuer_pid: value.issuer_pid.into(),
        }
    }
}
impl From<stored::RootIssuerRenewalStateView> for native::RootIssuerRenewalStateView {
    fn from(value: stored::RootIssuerRenewalStateView) -> Self {
        Self {
            issuer_pid: value.issuer_pid.into(),
            template_fingerprint: value.template_fingerprint,
            last_installed_cert_hash: value.last_installed_cert_hash,
            last_installed_expires_at_ns: value.last_installed_expires_at_ns,
            last_installed_refresh_after_ns: value.last_installed_refresh_after_ns,
            next_attempt_after_ns: value.next_attempt_after_ns,
            updated_at_ns: value.updated_at_ns,
        }
    }
}
impl From<native::RootIssuerRenewalStateView> for stored::RootIssuerRenewalStateView {
    fn from(value: native::RootIssuerRenewalStateView) -> Self {
        Self {
            issuer_pid: value.issuer_pid.into(),
            template_fingerprint: value.template_fingerprint,
            last_installed_cert_hash: value.last_installed_cert_hash,
            last_installed_expires_at_ns: value.last_installed_expires_at_ns,
            last_installed_refresh_after_ns: value.last_installed_refresh_after_ns,
            next_attempt_after_ns: value.next_attempt_after_ns,
            updated_at_ns: value.updated_at_ns,
        }
    }
}
impl From<stored::RootIssuerRenewalStatusResponse> for native::RootIssuerRenewalStatusResponse {
    fn from(value: stored::RootIssuerRenewalStatusResponse) -> Self {
        Self {
            template: value.template.map(Into::into),
            state: value.state.map(Into::into),
            latest_batch: value.latest_batch,
        }
    }
}
impl From<native::RootIssuerRenewalStatusResponse> for stored::RootIssuerRenewalStatusResponse {
    fn from(value: native::RootIssuerRenewalStatusResponse) -> Self {
        Self {
            template: value.template.map(Into::into),
            state: value.state.map(Into::into),
            latest_batch: value.latest_batch,
        }
    }
}
