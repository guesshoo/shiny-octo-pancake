use lmdb::{Environment, Database, Transaction, WriteFlags};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

// Errors returned by KvStore operations.
#[derive(Debug, Error)]
pub enum KvsError {
    ///An underlying LMDB error occured.
    #[error("LMDB error: {0}")]
    Lmdb(#[from] lmdb::Error),
}

// LMDB-backed key-value store
#[derive(Clone)]
pub struct KvStore {
    env: Arc<Environment>,
    db: Database,
}

impl KvStore {
    /// Opens (or creates) an LMDB environment at the specified path.
    ///
    /// `path` is the directory where the LMDB data files will reside.
    /// We set the map size to 1 GiB (`1 << 30` bytes) to cap
    /// the maximum database size.
    /// Creates a LDMB database called `kv`
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, KvsError> {
        let env = Environment::new()
            .set_max_dbs(1)
            .set_map_size(1 << 30) // 1 GiB
            .open(path.as_ref())?;
        let db = env.create_db(Some("kv"), lmdb::DatabaseFlags::empty())?;
        Ok(KvStore { env: Arc::new(env), db })
    }

    /// Inserts or updates a value under the given `key`.
    ///
    /// Both `key` and `value` are borrowed as `&[u8]` to avoid forcing
    /// the caller to allocate a `Vec<u8>` when they already have
    /// a contiguous byte buffer. Internally, LMDB will copy the data
    /// into its own memory-mapped region.
    pub fn put(&self, key: &[u8], value: &[u8]) -> Result<(), KvsError> {
        let mut wtxn = self.env.begin_rw_txn()?;
        wtxn.put(self.db, &key, &value, WriteFlags::empty())?;
        wtxn.commit()?;
        Ok(())
    }

    /// Deletes the entry for the specified `key`, if it exists.
    ///
    /// Returns the number of entries removed (0 or 1). We return a
    /// count rather than a unit type so callers can distinguish
    /// between "key not found" (0) and successful deletion (1).
    pub fn delete(&self, key: &[u8]) -> Result<usize, KvsError> {
        let mut wtxn = self.env.begin_rw_txn()?;
        match wtxn.del(self.db, &key, None){
            Ok(_) => {
                wtxn.commit()?;
                Ok(1)
            }
            Err(lmdb::Error::NotFound) => {
                wtxn.abort();
                Ok(0)
            }
            Err(e) => Err(KvsError::Lmdb(e)),
        }
    }

    /// Retrieves the value for `key`, if present.
    ///
    /// Uses a lightweight read-only transaction (`begin_ro_txn`) under the hood.
    /// Returns `Ok(Some(value))` on success, `Ok(None)` if the key is missing,
    /// or an error if any other LMDB issue occurs.
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, KvsError> {
        let rtxn = self.env.begin_ro_txn()?;
        match rtxn.get(self.db, &key) {
            Ok(val) => Ok(Some(val.to_vec())),
            Err(lmdb::Error::NotFound) => Ok(None),
            Err(e) => Err(KvsError::Lmdb(e)),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_put_get_delete() {
        let dir = tempdir().expect("create temp dir");
        let db_path = dir.path().join("db");
        fs::create_dir_all(&db_path).unwrap();

        let store = KvStore::open(&db_path).expect("open store");
        // get on missing key
        assert_eq!(store.get(b"foo").unwrap(), None);

        // put and get
        store.put(b"foo", b"bar").unwrap();
        let val = store.get(b"foo").unwrap();
        assert_eq!(val, Some(b"bar".to_vec()));

        // // putting it again
        // store.put(b"foo", b"value").unwrap();
        // let val = store.get(b"foo").unwrap();
        // assert_eq!(val, Some(b"value".to_vec()));

        // delete and get
        let deleted = store.delete(b"foo").unwrap();
        assert_eq!(deleted, 1); // deleted 1 entry
        assert_eq!(store.get(b"foo").unwrap(), None);
    }


    #[test]
    fn test_delete_nonexistent_key() {
        let dir = tempdir().expect("create temp dir");
        let db_path = dir.path().join("db_nonexistent");
        fs::create_dir_all(&db_path).unwrap();

        let store = KvStore::open(&db_path).expect("open store");
        let deleted = store.delete(b"no_key").unwrap();
        assert_eq!(deleted, 0);
        assert_eq!(store.get(b"no_key").unwrap(), None);
    }


    #[test]
    fn test_persistence_across_reopen() {
        let dir = tempdir().expect("create temp dir");
        let db_path = dir.path().join("db_persist");
        fs::create_dir_all(&db_path).unwrap();

        // First open: insert keys
        {
            let store = KvStore::open(&db_path).expect("open store first time");
            store.put(b"alpha", b"one").unwrap();
            store.put(b"beta", b"two").unwrap();
            assert_eq!(store.get(b"alpha").unwrap(), Some(b"one".to_vec()));
            assert_eq!(store.get(b"beta").unwrap(), Some(b"two".to_vec()));
        }
        // Drop env and reopen
        {
            let store = KvStore::open(&db_path).expect("open store second time");
            // Ensure previously inserted values are still there
            assert_eq!(store.get(b"alpha").unwrap(), Some(b"one".to_vec()));
            assert_eq!(store.get(b"beta").unwrap(), Some(b"two".to_vec()));
        }
    }
}
