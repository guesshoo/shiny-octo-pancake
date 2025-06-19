use lmdb::{Environment, Database, Transaction, WriteFlags, RoTransaction};

/// LMDB-backed implementation of `Storage`.
pub struct LmdbStorage {
    pub env: Environment,
    db: Database,
}

impl LmdbStorage {
    pub fn new(path: impl AsRef<Path>, max_dbs: u32) -> Result<Self, StorageError> {
        let env = Environment::new()
            .set_max_dbs(max_dbs)
            .open(path.as_ref())?;
        let db = env.create_db(Some("kv_store"), lmdb::DatabaseFlags::empty())?;
        Ok(LmdbStorage { env, db })
    }
}

impl Storage for LmdbStorage {
    type RoTxn<'txn> = RoTransaction<'txn>;
    type Value<'txn> = &'txn [u8];

    fn begin_read(&self) -> Result<Self::RoTxn<'_>, StorageError> {
        Ok(self.env.begin_ro_txn()?)
    }

    fn get<'txn>(
        &self,
        txn: &'txn Self::RoTxn<'txn>,
        key: &[u8],
    ) -> Result<Option<Self::Value<'txn>>, StorageError> {
        match txn.get(self.db, key) {
            Ok(slice) => Ok(Some(slice)),
            Err(lmdb::Error::NotFound) => Ok(None),
            Err(e) => Err(StorageError::Lmdb(e)),
        }
    }

    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), StorageError> {
        let mut wtxn = self.env.begin_rw_txn()?;
        wtxn.put(self.db, key, value, WriteFlags::empty())?;
        wtxn.commit()?;
        Ok(())
    }

    fn delete(&self, key: &[u8]) -> Result<(), StorageError> {
        let mut wtxn = self.env.begin_rw_txn()?;
        let _ = wtxn.del(self.db, key, None);
        wtxn.commit()?;
        Ok(())
    }
}