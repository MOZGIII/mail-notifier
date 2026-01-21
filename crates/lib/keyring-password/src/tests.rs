use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use keyring_core::api::{CredentialApi, CredentialStoreApi};
use keyring_core::{Credential, CredentialStore, Entry};

use super::*;

/// Map of stored secrets keyed by (service, user).
type SecretMap = HashMap<(String, String), Vec<u8>>;

/// Shared secret map for test credentials.
type SharedSecretMap = Arc<Mutex<SecretMap>>;

/// Serialize keyring tests because they mutate the global default store.
static KEYRING_TEST_LOCK: Mutex<()> = Mutex::new(());

/// In-memory credential store for keyring tests.
#[derive(Debug)]
struct InMemoryStore {
    /// Stored secrets keyed by (service, user).
    entries: SharedSecretMap,
}

impl InMemoryStore {
    /// Create a new in-memory store.
    fn new() -> Arc<Self> {
        Arc::new(Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Insert or replace a password for a service/user pair.
    fn insert_password(&self, service: &str, user: &str, password: &str) {
        let mut entries = self
            .entries
            .lock()
            .expect("in-memory keyring mutex poisoned");
        entries.insert(
            (service.to_string(), user.to_string()),
            password.as_bytes().to_vec(),
        );
    }
}

impl CredentialStoreApi for InMemoryStore {
    fn vendor(&self) -> String {
        "in-memory".to_string()
    }

    fn id(&self) -> String {
        "in-memory-store".to_string()
    }

    fn build(
        &self,
        service: &str,
        user: &str,
        _modifiers: Option<&HashMap<&str, &str>>,
    ) -> keyring_core::Result<Entry> {
        let credential = InMemoryCredential {
            entries: Arc::clone(&self.entries),
            service: service.to_string(),
            user: user.to_string(),
        };
        Ok(Entry::new_with_credential(Arc::new(credential)))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// In-memory credential implementation for tests.
#[derive(Clone, Debug)]
struct InMemoryCredential {
    /// Shared credential storage.
    entries: SharedSecretMap,
    /// Service name for this credential.
    service: String,
    /// Account name for this credential.
    user: String,
}

impl CredentialApi for InMemoryCredential {
    fn set_secret(&self, secret: &[u8]) -> keyring_core::Result<()> {
        let mut entries = self
            .entries
            .lock()
            .expect("in-memory keyring mutex poisoned");
        entries.insert((self.service.clone(), self.user.clone()), secret.to_vec());
        Ok(())
    }

    fn get_secret(&self) -> keyring_core::Result<Vec<u8>> {
        let entries = self
            .entries
            .lock()
            .expect("in-memory keyring mutex poisoned");
        entries
            .get(&(self.service.clone(), self.user.clone()))
            .cloned()
            .ok_or(keyring_core::Error::NoEntry)
    }

    fn delete_credential(&self) -> keyring_core::Result<()> {
        let mut entries = self
            .entries
            .lock()
            .expect("in-memory keyring mutex poisoned");
        match entries.remove(&(self.service.clone(), self.user.clone())) {
            Some(_) => Ok(()),
            None => Err(keyring_core::Error::NoEntry),
        }
    }

    fn get_credential(&self) -> keyring_core::Result<Option<Arc<Credential>>> {
        Ok(Some(Arc::new(self.clone())))
    }

    fn get_specifiers(&self) -> Option<(String, String)> {
        Some((self.service.clone(), self.user.clone()))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Guard that restores the previous default keyring store on drop.
struct DefaultStoreGuard {
    /// Previously configured default store.
    previous: Option<Arc<CredentialStore>>,
}

impl DefaultStoreGuard {
    /// Replace the default store with the provided one.
    fn install(store: Arc<CredentialStore>) -> Self {
        let previous = keyring_core::get_default_store();
        keyring_core::set_default_store(store);
        Self { previous }
    }
}

impl Drop for DefaultStoreGuard {
    fn drop(&mut self) {
        if let Some(previous) = self.previous.take() {
            keyring_core::set_default_store(previous);
        } else {
            let _ = keyring_core::unset_default_store();
        }
    }
}

#[test]
fn gets_password_from_keyring() {
    let _lock = KEYRING_TEST_LOCK
        .lock()
        .expect("keyring test lock poisoned");

    let store = InMemoryStore::new();
    store.insert_password("mail-notifier", "user@example.com", "secret");
    let store: Arc<CredentialStore> = store;
    let _guard = DefaultStoreGuard::install(store);

    let password = get("mail-notifier", "user@example.com").expect("password should resolve");

    assert_eq!(password, "secret");
}

#[test]
fn sets_password_in_keyring() {
    let _lock = KEYRING_TEST_LOCK
        .lock()
        .expect("keyring test lock poisoned");

    let store = InMemoryStore::new();
    let store: Arc<CredentialStore> = store;
    let _guard = DefaultStoreGuard::install(store);

    set("mail-notifier", "user@example.com", "secret").expect("password should store");

    let password = get("mail-notifier", "user@example.com").expect("password should resolve");

    assert_eq!(password, "secret");
}

#[test]
fn errors_when_keyring_entry_missing() {
    let _lock = KEYRING_TEST_LOCK
        .lock()
        .expect("keyring test lock poisoned");

    let store = InMemoryStore::new();
    let store: Arc<CredentialStore> = store;
    let _guard = DefaultStoreGuard::install(store);

    let error = get("mail-notifier", "user@example.com").expect_err("missing entry should error");

    match error {
        GetError::Resolve {
            service,
            account,
            source,
        } => {
            assert_eq!(service, "mail-notifier");
            assert_eq!(account, "user@example.com");
            assert!(matches!(source, keyring_core::Error::NoEntry));
        }
    }
}
