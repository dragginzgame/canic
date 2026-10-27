//! Fixed metadata emitted by the compiled Wire adapter, independent of payload codecs.

/// Private Wasm metadata; never an application endpoint or storage encoding.
pub const SECTION: &str = "icp:private canic:wire";
/// Version, kind, name length, method identity and eight u64 codec allowances.
pub const HEADER_BYTES: usize = 76;

/// Actual IC export kind, retained independently of browser query transport.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MethodKind {
    Query,
    CompositeQuery,
    Update,
}

/// Architecture-independent compiled codec allowances.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MethodLimits {
    pub bytes: u64,
    pub allocation: u64,
    pub nodes: u64,
    pub depth: u64,
}

/// The declaration adjacent to an actual compiled Wire entrypoint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompiledMethod<'a> {
    pub name: &'a str,
    pub kind: MethodKind,
    pub id: u64,
    pub request: MethodLimits,
    pub response: MethodLimits,
}

/// Invalid declaration bytes; no partial record is admitted.
#[derive(Clone, Copy, Debug, Eq, thiserror::Error, PartialEq)]
pub enum DeclarationError {
    #[error("truncated Wire declaration")]
    Framing,
    #[error("unsupported Wire declaration version")]
    Version,
    #[error("invalid Wire export kind")]
    Kind,
    #[error("invalid Wire method name")]
    Name,
}

impl CompiledMethod<'_> {
    /// Emit an exact constant custom-section record from an endpoint declaration.
    ///
    /// # Panics
    /// Panics if the authored name or destination length cannot fit the format.
    #[must_use]
    pub const fn encode<const N: usize>(&self) -> [u8; N] {
        let name = self.name.as_bytes();
        assert!(!name.is_empty() && name.len() <= u16::MAX as usize);
        assert!(N == HEADER_BYTES + name.len());
        let mut bytes = [0; N];
        bytes[0] = 1;
        bytes[1] = match self.kind {
            MethodKind::Query => 1,
            MethodKind::Update => 2,
            MethodKind::CompositeQuery => 3,
        };
        let length = name.len().to_le_bytes();
        bytes[2] = length[0];
        bytes[3] = length[1];
        let values = [
            self.id,
            self.request.bytes,
            self.request.allocation,
            self.request.nodes,
            self.request.depth,
            self.response.bytes,
            self.response.allocation,
            self.response.nodes,
            self.response.depth,
        ];
        let mut field = 0;
        while field < values.len() {
            let value = values[field].to_le_bytes();
            let mut byte = 0;
            while byte < 8 {
                bytes[4 + field * 8 + byte] = value[byte];
                byte += 1;
            }
            field += 1;
        }
        let mut index = 0;
        while index < name.len() {
            bytes[HEADER_BYTES + index] = name[index];
            index += 1;
        }
        bytes
    }
}

impl<'a> CompiledMethod<'a> {
    /// Consume exactly one record, advancing the cursor only after full admission.
    pub fn decode(input: &mut &'a [u8]) -> Result<Self, DeclarationError> {
        let header = input.get(..HEADER_BYTES).ok_or(DeclarationError::Framing)?;
        if header[0] != 1 {
            return Err(DeclarationError::Version);
        }
        let kind = match header[1] {
            1 => MethodKind::Query,
            2 => MethodKind::Update,
            3 => MethodKind::CompositeQuery,
            _ => return Err(DeclarationError::Kind),
        };
        let length = usize::from(u16::from_le_bytes([header[2], header[3]]));
        let end = HEADER_BYTES + length;
        let name = input
            .get(HEADER_BYTES..end)
            .ok_or(DeclarationError::Framing)?;
        let name = std::str::from_utf8(name).map_err(|_| DeclarationError::Name)?;
        if name.is_empty() || name.chars().any(char::is_control) {
            return Err(DeclarationError::Name);
        }
        let value = |field: usize| {
            let mut bytes = [0; 8];
            bytes.copy_from_slice(&header[4 + field * 8..12 + field * 8]);
            u64::from_le_bytes(bytes)
        };
        let limits = |field| MethodLimits {
            bytes: value(field),
            allocation: value(field + 1),
            nodes: value(field + 2),
            depth: value(field + 3),
        };
        let method = Self {
            name,
            kind,
            id: value(0),
            request: limits(1),
            response: limits(5),
        };
        *input = &input[end..];
        Ok(method)
    }
}

#[cfg(test)]
mod tests;
