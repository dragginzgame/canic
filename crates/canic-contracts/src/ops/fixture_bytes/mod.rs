//! Lossless conversion at the fixture transport boundary, without storage re-encoding.
use crate::dto::{fixture_bytes as native, fixture_provisioning as stored};
impl From<stored::FixtureChunkRead> for native::ReadRequest {
    fn from(value: stored::FixtureChunkRead) -> Self {
        Self {
            grant: value.grant.into(),
            index: value.index,
        }
    }
}
impl From<native::ReadRequest> for stored::FixtureChunkRead {
    fn from(value: native::ReadRequest) -> Self {
        Self {
            grant: value.grant.into(),
            index: value.index,
        }
    }
}
impl From<stored::FixtureChunkUpload> for native::UploadRequest {
    fn from(value: stored::FixtureChunkUpload) -> Self {
        Self {
            content_id: value.content_id,
            index: value.index,
            bytes: value.bytes,
        }
    }
}
impl From<native::UploadRequest> for stored::FixtureChunkUpload {
    fn from(value: native::UploadRequest) -> Self {
        Self {
            content_id: value.content_id,
            index: value.index,
            bytes: value.bytes,
        }
    }
}
