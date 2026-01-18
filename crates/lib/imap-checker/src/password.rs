//! Password wrapper type.

/// Wrapper for sensitive passwords.
#[derive(Clone, Eq, PartialEq)]
pub struct Password(String);

impl Password {
    /// Create a new password wrapper.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Expose the inner password value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for Password {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for Password {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl std::fmt::Debug for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Password(***redacted***)")
    }
}
