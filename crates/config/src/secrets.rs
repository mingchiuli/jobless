use std::collections::HashMap;
use std::sync::Mutex;

use secrecy::{ExposeSecret, SecretBox};
use thiserror::Error;
use zeroize::Zeroize;

pub type SecretString = SecretBox<str>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SecretKey(String);

impl SecretKey {
    pub fn new(value: impl Into<String>) -> Result<Self, SecretError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= 128
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));

        if !valid {
            return Err(SecretError::InvalidKey(value));
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Error)]
pub enum SecretError {
    #[error("invalid secret key `{0}`")]
    InvalidKey(String),
    #[error("the system credential store is unavailable: {0}")]
    Unavailable(String),
    #[error("secret store operation failed: {0}")]
    Backend(String),
}

pub trait SecretStore: Send + Sync {
    fn get(&self, key: &SecretKey) -> Result<Option<SecretString>, SecretError>;
    fn set(&self, key: &SecretKey, value: SecretString) -> Result<(), SecretError>;
    fn delete(&self, key: &SecretKey) -> Result<(), SecretError>;
}

pub struct KeyringSecretStore {
    service: String,
}

impl KeyringSecretStore {
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }
}

impl SecretStore for KeyringSecretStore {
    fn get(&self, key: &SecretKey) -> Result<Option<SecretString>, SecretError> {
        let entry = keyring::Entry::new(&self.service, key.as_str())
            .map_err(|error| SecretError::Unavailable(error.to_string()))?;

        match entry.get_password() {
            Ok(value) => Ok(Some(value.into_boxed_str().into())),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(SecretError::Backend(error.to_string())),
        }
    }

    fn set(&self, key: &SecretKey, value: SecretString) -> Result<(), SecretError> {
        let entry = keyring::Entry::new(&self.service, key.as_str())
            .map_err(|error| SecretError::Unavailable(error.to_string()))?;

        entry
            .set_password(value.expose_secret())
            .map_err(|error| SecretError::Backend(error.to_string()))
    }

    fn delete(&self, key: &SecretKey) -> Result<(), SecretError> {
        let entry = keyring::Entry::new(&self.service, key.as_str())
            .map_err(|error| SecretError::Unavailable(error.to_string()))?;

        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(SecretError::Backend(error.to_string())),
        }
    }
}

#[derive(Default)]
pub struct MemorySecretStore {
    values: Mutex<HashMap<String, SecretString>>,
}

impl MemorySecretStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SecretStore for MemorySecretStore {
    fn get(&self, key: &SecretKey) -> Result<Option<SecretString>, SecretError> {
        let values = self.values.lock().expect("secret store lock poisoned");
        Ok(values.get(key.as_str()).cloned())
    }

    fn set(&self, key: &SecretKey, value: SecretString) -> Result<(), SecretError> {
        let mut values = self.values.lock().expect("secret store lock poisoned");
        values.insert(key.as_str().to_string(), value);
        Ok(())
    }

    fn delete(&self, key: &SecretKey) -> Result<(), SecretError> {
        let mut values = self.values.lock().expect("secret store lock poisoned");
        if let Some(mut value) = values.remove(key.as_str()) {
            value.zeroize();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_key_rejects_path_like_values() {
        assert!(SecretKey::new("proxy/password").is_err());
        assert!(SecretKey::new("proxy.password").is_ok());
    }

    #[test]
    fn memory_store_round_trips() {
        let store = MemorySecretStore::new();
        let key = SecretKey::new("proxy.password").unwrap();
        store
            .set(&key, "secret".to_owned().into_boxed_str().into())
            .unwrap();

        let value = store.get(&key).unwrap().unwrap();
        assert_eq!(value.expose_secret(), "secret");

        store.delete(&key).unwrap();
        assert!(store.get(&key).unwrap().is_none());
    }
}
