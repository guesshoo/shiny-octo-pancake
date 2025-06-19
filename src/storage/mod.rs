//! Zero-copy KV store abstraction, multiple backends (LMDB, In-Memory, Sled) with WAL support

pub use self::backend::*;
pub use self::lmdb::LmdbStorage;
pub use self::sled::SledStorage;
pub use self::memory::InMemoryStorage;
pub use self::wal::WalStorage;

mod lmdb;
mod sled;
mod memory;
mod wal;


/// Errors that can occur in storage operations.
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("LMDB error: {0}")]
    Lmdb(#[from] lmdb::Error),
    #[error("Sled error: {0}")]
    Sled(#[from] sled::Error),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] bincode::Error),
}

/// Generic storage trait, not tied to any backend's transaction type.
pub trait Storage {
    /// Associated read-transaction handle type.
    type RoTxn<'txn>: 'txn;
    /// Associated zero-copy value slice type.
    type Value<'txn>: AsRef<[u8]> + 'txn;

    /// Begin a read-only transaction/context.
    fn begin_read(&self) -> Result<Self::RoTxn<'_>, StorageError>;
    /// Get a value by key within a read transaction.
    fn get<'txn>(
        &self,
        txn: &'txn Self::RoTxn<'txn>,
        key: &[u8],
    ) -> Result<Option<Self::Value<'txn>>, StorageError>;
    /// Put a key-value pair into storage.
    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), StorageError>;
    /// Delete a key (no-op if missing).
    fn delete(&self, key: &[u8]) -> Result<(), StorageError>;
}


//=== Tests ===

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_storage<S: Storage>(store: &S) -> Result<(), StorageError> {
        store.put(b"k1", b"v1")?;
        let txn = store.begin_read()?;
        assert_eq!(store.get(&txn, b"k1")?.unwrap().as_ref(), b"v1");
        store.delete(b"k1")?;
        let txn2 = store.begin_read()?;
        assert!(store.get(&txn2, b"k1")?.is_none());
        Ok(())
    }

    #[test]
    fn in_memory_works() -> Result<(), StorageError> {
        let mem = InMemoryStorage::new();
        test_storage(&mem)
    }

    #[test]
    fn sled_storage_works() -> Result<(), StorageError> {
        let dir = tempdir()?;
        let sled = SledStorage::new(dir.path().join("sled_db"))?;
        test_storage(&sled)
    }

    #[test]
    fn lmdb_storage_works() -> Result<(), StorageError> {
        let dir = tempdir()?;
        let lmdb = LmdbStorage::new(dir.path().join("lmdb"), 1)?;
        test_storage(&lmdb)
    }

    #[test]
    fn wal_on_in_memory_works() -> Result<(), StorageError> {
        let dir = tempdir()?;
        let mem = InMemoryStorage::new();
        let mut wal = WalStorage::new(mem, dir.path().join("wal_mem.log"))?;
        test_storage(&wal)
    }

    #[test]
    fn wal_on_sled_works() -> Result<(), StorageError> {
        let dir = tempdir()?;
        let sled = SledStorage::new(dir.path().join("sled2"))?;
        let mut wal = WalStorage::new(sled, dir.path().join("wal_sled.log"))?;
        test_storage(&wal)
    }
}
