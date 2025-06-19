use sled::Db as SledDb;
use serde::{Serialize, Deserialize};
use thiserror::Error;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::fs::{OpenOptions, File};
use std::io::{self, Read, Seek, SeekFrom, Write, IoSlice};
use std::collections::HashMap;

/// Sled database implementation.
pub struct SledStorage {
    db: SledDb,
}

impl SledStorage {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let db = sled::open(path)?;
        Ok(SledStorage { db })
    }
}

impl Storage for SledStorage {
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
        Ok(self.db.get(key)?.map(|ivec| ivec.as_ref()))
    }

    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), StorageError> {
        self.db.insert(key, value)?;
        self.db.flush()?;
        Ok(())
    }

    fn delete(&self, key: &[u8]) -> Result<(), StorageError> {
        self.db.remove(key)?;
        self.db.flush()?;
        Ok(())
    }
}
