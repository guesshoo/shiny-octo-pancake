use lmdb::{Environment, Database, Transaction, WriteFlags};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;


#[derive(Debug, Error)]
pub enum KvsError {
    #[error("LMDB error: {0}")]
    Lmdb(#[from] lmdb::Error),
}

#[derive(Clone)]
pub struct KvStore {
    env: Arc<Environment>,
    db: Database,
}

impl KvStore {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, KvsError> {
        let env = Environment::new()
            .set_max_dbs(1)
            .set_map_size(1 << 30) // 1 GiB
            .open(path.as_ref())?;
        let db = env.create_db(Some("kv"), lmdb::DatabaseFlags::empty())?;
        Ok(KvStore { env: Arc::new(env), db })
    }

    pub fn put(&self, key: &[u8], value: &[u8]) -> Result<(), KvsError> {
        let mut wtxn = self.env.begin_rw_txn()?;
        wtxn.put(self.db, &key, &value, WriteFlags::empty())?;
        wtxn.commit()?;
        Ok(())
    }

    pub fn delete(&self, key: &[u8]) -> Result<(), KvsError> {
        let mut wtxn = self.env.begin_rw_txn()?;
        wtxn.del(self.db, &key, None)?;
        wtxn.commit()?;
        Ok(())
    }

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

        // putting it again
        store.put(b"foo", b"value").unwrap();
        let val = store.get(b"foo").unwrap();
        assert_eq!(val, Some(b"value".to_vec()));

        // delete and get
        store.delete(b"foo").unwrap();
        assert_eq!(store.get(b"foo").unwrap(), None);
    }


    #[test]
    fn test_delete_nonexistent_key() {
        let dir = tempdir().expect("create temp dir");
        let db_path = dir.path().join("db_nonexistent");
        fs::create_dir_all(&db_path).unwrap();

        let store = KvStore::open(&db_path).expect("open store");
        // deleting a non-existent key should return an error
        assert!(store.delete(b"no_key").is_err());
        // after the failed delete, the key should still be absent
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
