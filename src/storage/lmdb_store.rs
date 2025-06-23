use super::Storage;
use super::StorageError;
use lmdb::{Environment, Database, WriteFlags, Transaction};
use std::path::Path;
use std::sync::Arc;

/// LMDB-backed implementation of `Storage`.
pub struct LmdbStorage {
    /// Shared LMDB environment (memory-mapped).
    pub env: Arc<Environment>,
    /// Handle to the named database within the environment.
    pub db: Database,
}

impl LmdbStorage {
    /// Create or open an LMDB environment at `path`, allowing `max_dbs` named DBs.
    pub fn new(path: impl AsRef<Path>, max_dbs: u32) -> Result<Self, StorageError> {
        std::fs::create_dir_all(path.as_ref())?;
        let env = Arc::new(
            Environment::new().set_max_dbs(max_dbs).open(path.as_ref())?
        );
        let db = env.create_db(Some("kv_store"), lmdb::DatabaseFlags::empty())?;
        Ok(LmdbStorage { env, db })
    }

    /// Create a new handle for an existing environment and named DB.
    pub fn from_env(env: Arc<Environment>, name: &str) -> Result<Self, StorageError> {
        let db = env.create_db(Some(name), lmdb::DatabaseFlags::empty())?;
        Ok(LmdbStorage { env, db })
    }
}

impl Storage for LmdbStorage {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        let txn = self.env.begin_ro_txn()?; // Begin read-only transaction
        match txn.get(self.db, &key) {
            Ok(slice) => Ok(Some(slice.to_vec())), // Return owned Vec
            Err(lmdb::Error::NotFound) => Ok(None),
            Err(e) => Err(StorageError::Lmdb(e)),
        }
    }

    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), StorageError> {
        let mut wtxn = self.env.begin_rw_txn()?; // Begin read-write txn
        wtxn.put(self.db, &key, &value, WriteFlags::empty())?;
        wtxn.commit()?; // Commit ensures durability
        Ok(())
    }

    fn delete(&self, key: &[u8]) -> Result<(), StorageError> {
        let mut wtxn = self.env.begin_rw_txn()?;
        let _ = wtxn.del(self.db, &key, None); // Delete or no-op
        wtxn.commit()?;
        Ok(())
    }

    fn put_if_absent(&self, key: &[u8], value: &[u8]) -> Result<bool, StorageError> {
        let mut wtxn = self.env.begin_rw_txn()?;
        // NO_OVERWRITE flag ensures atomic insert if absent
        match wtxn.put(self.db, &key,&value, WriteFlags::NO_OVERWRITE) {
            Ok(()) => { wtxn.commit()?; Ok(true) },
            Err(lmdb::Error::KeyExist) => Ok(false),
            Err(e) => Err(StorageError::Lmdb(e)),
        }
    }
}

// Unit tests for LmdbStorage
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// Test basic put/get/delete operations on LMDB backend.
    #[test]
    fn lmdb_basic_operations() -> Result<(), StorageError> {
        let dir = tempdir()?;
        let db_path = dir.path().join("testdb");
        let store = LmdbStorage::new(&db_path, 1)?;

        // Key should not exist initially
        assert!(store.get(b"foo")?.is_none());

        // Put and get
        store.put(b"foo", b"bar")?;
        assert_eq!(store.get(b"foo")?.as_deref(), Some(&b"bar"[..]));

        // Delete and verify removal
        store.delete(b"foo")?;
        assert!(store.get(b"foo")?.is_none());
        Ok(())
    }

    /// Test atomic put_if_absent with NO_OVERWRITE flag.
    #[test]
    fn lmdb_put_if_absent() -> Result<(), StorageError> {
        let dir = tempdir()?;
        let db_path = dir.path().join("testdb2");
        let store = LmdbStorage::new(&db_path, 1)?;

        // First insert should succeed
        assert!(store.put_if_absent(b"key", b"value1")?);
        assert_eq!(store.get(b"key")?.unwrap(), b"value1".to_vec());

        // Second insert should fail (key exists)
        assert!(!store.put_if_absent(b"key", b"value2")?);
        // Value remains unchanged
        assert_eq!(store.get(b"key")?.unwrap(), b"value1".to_vec());
        Ok(())
    }
}