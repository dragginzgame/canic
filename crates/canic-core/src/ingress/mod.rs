//! Module: ingress
//!
//! Responsibility: ingress boundary helpers for macro-generated entry points.
//! Does not own: endpoint authorization, dispatch, or DTO decoding.
//! Boundary: exposes ingress-time guards and payload limit metadata.

pub mod payload;
