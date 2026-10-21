#![allow(dead_code)]

use thiserror::Error;

const SERVICE: &str = "SteamPipe GUI";

pub const BUILD_ACCOUNT_NOTE: &str = "Use a dedicated build account with only the permissions needed to edit app metadata and publish changes, not your personal Steam login.";

pub const MASK: &str = "********";

/// Keyring account name for a Steam login password, namespaced so it never
/// collides with other stored secrets.
pub fn login_key(username: &str) -> String {
    format!("login:{username}")
}

#[derive(Debug, Error)]
pub enum SecretError {
    #[error("credential store error: {0}")]
    Store(String),
}

type Result<T> = std::result::Result<T, SecretError>;

fn entry(account: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, account).map_err(|e| SecretError::Store(e.to_string()))
}

pub fn get_password(account: &str) -> Result<Option<String>> {
    match entry(account)?.get_password() {
        Ok(p) => Ok(Some(p)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(SecretError::Store(e.to_string())),
    }
}

pub fn set_password(account: &str, password: &str) -> Result<()> {
    entry(account)?
        .set_password(password)
        .map_err(|e| SecretError::Store(e.to_string()))
}

pub fn delete_password(account: &str) -> Result<()> {
    match entry(account)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(SecretError::Store(e.to_string())),
    }
}

pub fn redact(text: &str, secret: &str) -> String {
    if secret.is_empty() {
        text.to_string()
    } else {
        text.replace(secret, MASK)
    }
}
