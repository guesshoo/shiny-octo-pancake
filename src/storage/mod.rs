// Simple KV store abstraction, multiple backends (LMDB, In-Memory, Sled) with WAL support

use lmdb::{Environment, Database, Transaction, WriteFlags};
use sled::Db as SledDb;
use serde::{Serialize, Deserialize};
use thiserror::Error;
use std::path::Path;
use std::sync::{ RwLock, Arc};
use std::fs::{OpenOptions, File};
use std::io::{self, Read, Seek, SeekFrom, Write, IoSlice};
use std::collections::HashMap;
use std::cell::RefCell;

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

/// Generic storage trait, simple owned buffer API.
pub trait Storage {
    /// Get a value by key. Returns Ok(None) if key not found.
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError>;
    /// Put a key-value pair into storage.
    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), StorageError>;
    /// Delete a key (no-op if missing).
    fn delete(&self, key: &[u8]) -> Result<(), StorageError>;
}

//=== LMDB Backend ===

/// LMDB-backed implementation of `Storage`.
#[derive(Clone)]
pub struct LmdbStorage {
    env: Arc<Environment>,
    db: Database,
}

use std::fs;

impl LmdbStorage {
    /// Open or create an LMDB environment at the given path.
    pub fn new(path: impl AsRef<Path>, max_dbs: u32) -> Result<Self, StorageError> {
        fs::create_dir_all(&path).unwrap();
        let env = Environment::new()
            .set_max_dbs(max_dbs)
            .set_map_size(1 << 30) // 1 GiB
            .open(path.as_ref())?;
        let db = env.create_db(Some("kv_store"), lmdb::DatabaseFlags::empty())?;
        Ok(LmdbStorage { env: Arc::new(env), db })
    }
}

impl Storage for LmdbStorage {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        let txn = self.env.begin_ro_txn()?;
        match txn.get(self.db, &key) {
            Ok(slice) => Ok(Some(slice.to_vec())),
            Err(lmdb::Error::NotFound) => Ok(None),
            Err(e) => Err(StorageError::Lmdb(e)),
        }
    }

    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), StorageError> {
        let mut wtxn = self.env.begin_rw_txn()?;
        wtxn.put(self.db, &key, &value, WriteFlags::empty())?;
        wtxn.commit()?;
        Ok(())
    }

    fn delete(&self, key: &[u8]) -> Result<(), StorageError> {
        let mut wtxn = self.env.begin_rw_txn()?;
        match wtxn.del(self.db, &key, None){
            Ok(_) => (),
            Err(lmdb::Error::NotFound) => (),
            Err(e) => return Err( StorageError::Lmdb(e)),
        }
        wtxn.commit()?;
        Ok(())
    }
}

//=== In-Memory Backend ===

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
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        let guard = self.map.read().unwrap();
        Ok(guard.get(key).cloned())
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

//=== Sled Backend ===

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
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        Ok(self.db.get(key)?.map(|ivec| ivec.to_vec()))
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

//=== WAL Operation ===

#[derive(Serialize, Deserialize, Debug)]
pub enum Operation {
    Put { key: Vec<u8>, value: Vec<u8> },
    Delete { key: Vec<u8> },
}

/// Append-only write-ahead log using vectored I/O.
pub struct WriteAheadLog {
    file: File,
}

impl WriteAheadLog {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(path.as_ref())?;
        Ok(WriteAheadLog { file })
    }

    pub fn append_op(&mut self, op: &Operation) -> Result<(), StorageError> {
        let payload = bincode::serialize(op)?;
        let len = (payload.len() as u32).to_le_bytes();
        let bufs = [IoSlice::new(&len), IoSlice::new(&payload)];
        self.file.write_vectored(&bufs)?;
        self.file.sync_data()?;
        Ok(())
    }

    pub fn replay(&mut self) -> Result<Vec<Operation>, StorageError> {
        self.file.seek(SeekFrom::Start(0))?;
        let mut ops = Vec::new();
        loop {
            let mut lenbuf = [0; 4];
            if self.file.read_exact(&mut lenbuf).is_err() { break; }
            let len = u32::from_le_bytes(lenbuf) as usize;
            let mut buf = vec![0; len];
            self.file.read_exact(&mut buf)?;
            ops.push(bincode::deserialize(&buf)?);
        }
        Ok(ops)
    }
}

//=== WAL Wrapper ===

/// WAL + inner storage wrapper.
pub struct WalStorage<S: Storage> {
    inner: S,
    wal: RefCell<WriteAheadLog>,
}

impl<S: Storage> WalStorage<S> {
    pub fn new(inner: S, wal_path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let mut log = WriteAheadLog::open(wal_path)?;
        for op in log.replay()? {
            match op {
                Operation::Put { key, value } => inner.put(&key, &value)?,
                Operation::Delete { key } => inner.delete(&key)?,
            }
        }
        Ok(WalStorage { inner, wal: RefCell::new(log) })
    }
}

impl<S: Storage> Storage for WalStorage<S> {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        self.inner.get(key)
    }

    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), StorageError> {
        let op = Operation::Put { key: key.to_vec(), value: value.to_vec() };
        self.wal.borrow_mut().append_op(&op)?;
        self.inner.put(key, value)
    }

    fn delete(&self, key: &[u8]) -> Result<(), StorageError> {
        let op = Operation::Delete { key: key.to_vec() };
        self.wal.borrow_mut().append_op(&op)?;
        self.inner.delete(key)
    }
}

//=== Tests ===

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_storage<S: Storage>(store: &S) -> Result<(), StorageError> {
        // get on missing key
        assert_eq!(store.get(b"foo").unwrap(), None);

        // put and get
        store.put(b"foo", b"bar")?;
        assert_eq!(store.get(b"foo")?.unwrap(), b"bar".to_vec());

        // delete and get
        store.delete(b"foo").unwrap();
        assert_eq!(store.get(b"foo").unwrap(), None);
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
        let lmdb = LmdbStorage::new(dir.path().join("lmdb"), 1)
            .expect("open lmdb store");
        test_storage(&lmdb)
    }

    #[ignore]
    #[test]
    fn test_lmdb_data_dir() ->  Result<(), StorageError>{
        let lmdb = LmdbStorage::new(Path::new("data/lmdb"), 1).expect("create lmdb");
        test_storage(&lmdb)
    }

    #[test]
    fn wal_on_in_memory_works() -> Result<(), StorageError> {
        let dir = tempdir()?;
        let mem = InMemoryStorage::new();
        let wal = WalStorage::new(mem, dir.path().join("wal_mem.log"))?;
        test_storage(&wal)
    }

    #[test]
    fn wal_on_sled_works() -> Result<(), StorageError> {
        let dir = tempdir()?;
        let sled = SledStorage::new(dir.path().join("sled2"))?;
        let wal = WalStorage::new(sled, dir.path().join("wal_sled.log"))?;
        test_storage(&wal)
    }

    #[test]
    fn wal_on_in_memory_replay_works() -> Result<(), StorageError> {
        let dir = tempdir()?;
        let wal_path = dir.path().join("wal_mem_replay.log");
        // Phase 1: initial writes
        {
            let mem1 = InMemoryStorage::new();
            let store1 = WalStorage::new(mem1, &wal_path)?;
            store1.put(b"a", b"1")?;
            store1.put(b"b", b"2")?;
        }
        // Phase 2: replay into a fresh in-memory
        let mem2 = InMemoryStorage::new();
        let store2 = WalStorage::new(mem2, &wal_path)?;
        assert_eq!(store2.get(b"a")?.unwrap(), b"1".to_vec());
        assert_eq!(store2.get(b"b")?.unwrap(), b"2".to_vec());
        Ok(())
    }

    #[test]
    fn wal_on_sled_replay_works() -> Result<(), StorageError> {
        let dir = tempdir()?;
        let wal_path = dir.path().join("wal_sled_replay.log");
        // Phase 1: initial writes
        {
            let sled1 = SledStorage::new(dir.path().join("sled3"))?;
            let store1 = WalStorage::new(sled1, &wal_path)?;
            store1.put(b"x", b"7")?;
            store1.put(b"y", b"8")?;
        }
        // Phase 2: replay into a fresh sled
        let sled2 = SledStorage::new(dir.path().join("sled4"))?;
        let store2 = WalStorage::new(sled2, &wal_path)?;
        assert_eq!(store2.get(b"x")?.unwrap(), b"7".to_vec());
        assert_eq!(store2.get(b"y")?.unwrap(), b"8".to_vec());
        Ok(())
    }

}
