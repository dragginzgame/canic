//! Module: deploy::output_format
//!
//! Responsibility: select maintained deployment output representations.
//! Does not own: command parsing, report construction, or output rendering.

/// The common JSON-or-human-readable deployment output choice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum JsonTextOutputFormat {
    Json,
    Text,
}

impl JsonTextOutputFormat {
    pub(super) const fn from_text_flag(text: bool) -> Self {
        if text { Self::Text } else { Self::Json }
    }

    pub(super) const fn from_json_flag(json: bool) -> Self {
        if json { Self::Json } else { Self::Text }
    }
}

/// The deployment-check output choice, including its evidence envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CheckOutputFormat {
    Json,
    EnvelopeJson,
    Text,
}
