use super::Storage;
use super::StorageError;
use std::collections::HashMap;
use std::sync::RwLock;

/// In-memory HashMap implementation. Not durable across restarts.
pub struct InMemoryStorage {
    map: RwLock<HashMap<Vec<u8>, Vec<u8>>>,
}

impl InMemoryStorage {
    /// Create a new, empty in-memory store.
    pub fn new() -> Self {
        InMemoryStorage { map: RwLock::new(HashMap::new()) }
    }
}

impl Storage for InMemoryStorage {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        Ok(self.map.read().unwrap().get(key).cloned())
    }

    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), StorageError> {
        self.map.write().unwrap().insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    fn delete(&self, key: &[u8]) -> Result<(), StorageError> {
        self.map.write().unwrap().remove(key);
        Ok(())
    }

    // Uses default put_if_absent (non-atomic)
}