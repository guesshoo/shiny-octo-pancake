use std::sync::RwLock;
use std::io::{ Read, Seek, SeekFrom, Write, IoSlice};
use std::collections::HashMap;

/// In-memory HashMap implementation. Not durable.
pub struct InMemoryStorage {
    map: RwLock<HashMap<Vec<u8>, Vec<u8>>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        InMemoryStorage { map: RwLock::new(HashMap::new()) }
    }
}

impl Storage for InMemoryStorage {
    type RoTxn<'txn> = ();
    type Value<'txn> = &'txn [u8];

    fn begin_read(&self) -> Result<Self::RoTxn<'_>, StorageError> {
        Ok(())
    }

    fn get<'txn>(
        &self,
        _txn: &'txn Self::RoTxn<'txn>,
        key: &[u8],
    ) -> Result<Option<Self::Value<'txn>>, StorageError> {
        let guard = self.map.read().unwrap();
        Ok(guard.get(key).map(|v| v.as_slice()))
    }

    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), StorageError> {
        let mut guard = self.map.write().unwrap();
        guard.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    fn delete(&self, key: &[u8]) -> Result<(), StorageError> {
        let mut guard = self.map.write().unwrap();
        guard.remove(key);
        Ok(())
    }
}