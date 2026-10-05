use std::fmt;
use zeroize::Zeroizing;

/// An owned passphrase whose application-controlled storage is erased on drop.
pub struct SecretPassphrase(Zeroizing<String>);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidPassphrase;

impl SecretPassphrase {
    pub fn new(value: String) -> Result<Self, InvalidPassphrase> {
        let value = Zeroizing::new(value);
        if value.is_empty() || value.contains('\0') {
            return Err(InvalidPassphrase);
        }
        Ok(Self(value))
    }

    pub(super) fn expose(&self) -> &str {
        &self.0
    }
}
impl fmt::Debug for SecretPassphrase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretPassphrase([REDACTED])")
    }
}
impl fmt::Display for InvalidPassphrase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("passphrase must be nonempty and contain no NUL")
    }
}
impl std::error::Error for InvalidPassphrase {}
