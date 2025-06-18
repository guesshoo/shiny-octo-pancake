#[async_trait::async_trait]
trait Storage: Send + Sync + 'static {
    async fn get(&self, cache: &str, key: Vec<u8>) -> Result<Option<Vec<u8>>, Box<dyn Error>>;
    async fn put(&self, cache: &str, key: Vec<u8>, value: Vec<u8>) -> Result<(), Box<dyn Error>>;
    async fn evict(&self, cache: &str, key: Vec<u8>) -> Result<(), Box<dyn Error>>;
    async fn stats(&self, cache: &str) -> Result<u64, Box<dyn Error>>;
}


/// LMDB-based storage implementation.
struct LmdbStorage {
    env: lmdb::Environment,
    db: lmdb::Database,
}

impl LmdbStorage {
    fn new(path: &str) -> Result<Self, Box<dyn Error>> {
        let env = lmdb::Environment::new()
            .set_max_dbs(1)
            .open(path)?;
        let db = env.create_db(Some("cache"), lmdb::DatabaseFlags::empty())?;
        Ok(Self { env, db })
    }
}

#[async_trait::async_trait]
impl Storage for LmdbStorage {
    async fn get(&self, _cache: &str, key: Vec<u8>) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
        let txn = lmdb::RoTransaction::new(&self.env)?;
        match txn.get(self.db, &key) {
            Ok(v) => Ok(Some(v.to_vec())),
            Err(lmdb::Error::NotFound) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn put(&self, _cache: &str, key: Vec<u8>, value: Vec<u8>) -> Result<(), Box<dyn Error>> {
        let mut wtxn = lmdb::RwTransaction::new(&self.env)?;
        wtxn.put(self.db, &key, &value, lmdb::WriteFlags::empty())?;
        wtxn.commit()?;
        Ok(())
    }

    async fn evict(&self, _cache: &str, key: Vec<u8>) -> Result<(), Box<dyn Error>> {
        let mut wtxn = lmdb::RwTransaction::new(&self.env)?;
        let _ = wtxn.del(self.db, &key, None);
        wtxn.commit()?;
        Ok(())
    }

    async fn stats(&self, _cache: &str) -> Result<u64, Box<dyn Error>> {
        let txn = lmdb::RoTransaction::new(&self.env)?;
        let mut cursor = txn.open_ro_cursor(self.db)?;
        Ok(cursor.iter_start().count() as u64)
    }
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test_lmdb_storage() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_str().unwrap();
        let storage = LmdbStorage::new(path).unwrap();

        // initially empty
        assert_eq!(storage.get("default", b"k1".to_vec()).await.unwrap(), None);
        // put and get
        storage.put("default", b"k1".to_vec(), b"v1".to_vec()).await.unwrap();
        assert_eq!(storage.get("default", b"k1".to_vec()).await.unwrap(), Some(b"v1".to_vec()));
        // stats should be 1
        assert_eq!(storage.stats("default").await.unwrap(), 1);
        // miss
        assert!(storage.get("default", b"no".to_vec()).await.unwrap().is_none());
        // evict and stats
        storage.evict("default", b"k1".to_vec()).await.unwrap();
        assert!(storage.get("default", b"k1".to_vec()).await.unwrap().is_none());
        assert_eq!(storage.stats("default").await.unwrap(), 0);
    }
}