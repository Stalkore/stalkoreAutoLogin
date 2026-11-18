//! Client-independent action names and credential buffer bounds.

/// Keep encoded credentials within the supported native login fields.
pub fn valid_credentials(username: &[u8], password: &[u8]) -> bool {
    [username, password]
        .into_iter()
        .all(|value| !value.is_empty() && value.len() < 64 && !value.contains(&0))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginAction {
    OpenAccountSelection,
    SelectAccount,
    SubmitCredentials,
    SelectServer,
    SelectCharacter,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_reject_empty_nul_and_oversized_fields() {
        assert!(valid_credentials(b"fixture@example.org", &[b'p'; 63]));
        for (username, password) in [
            (b"".as_slice(), b"fixture".as_slice()),
            (b"fixture".as_slice(), b"".as_slice()),
            (b"fixture\0user".as_slice(), b"fixture".as_slice()),
            (&[b'u'; 64], b"fixture".as_slice()),
            (b"fixture".as_slice(), &[b'p'; 64]),
        ] {
            assert!(!valid_credentials(username, password));
        }
    }
}
