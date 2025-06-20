//! Simple KV store abstraction, multiple backends (LMDB, In-Memory, Sled) with WAL support

use lmdb::{Environment, Database, Transaction, WriteFlags, RoTransaction};
use sled::Db as SledDb;
use serde::{Serialize, Deserialize};
use thiserror::Error;
use std::path::Path;
use std::sync::RwLock;
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
/// LMDB-backed implementation of `Storage`.
pub struct LmdbStorage {
    /// Shared LMDB environment
    pub env: Arc<Environment>,
    /// Named database handle
    pub db: Database,
}

impl LmdbStorage {
    /// Open or create an LMDB environment at the given path.
    pub fn new(path: impl AsRef<Path>, max_dbs: u32) -> Result<Self, StorageError> {
        // ensure directory
        std::fs::create_dir_all(path.as_ref())?;
        let env = Environment::new()
            .set_max_dbs(max_dbs)
            .open(path.as_ref())?;
        let env = Arc::new(env);
        let db = env.create_db(Some("kv_store"), lmdb::DatabaseFlags::empty())?;
        Ok(LmdbStorage { env, db })
    }

    /// Create a storage from an existing shared environment and named database
    pub fn from_env(env: Arc<Environment>, name: &str) -> Result<Self, StorageError> {
        let db = env.create_db(Some(name), lmdb::DatabaseFlags::empty())?;
        Ok(LmdbStorage { env, db })
    }

    /// Change active named database in this environment (reuse struct)
    pub fn use_db(&self, name: &str) -> Result<Self, StorageError> {
        let db = Arc::clone(&self.env).create_db(Some(name), lmdb::DatabaseFlags::empty())?;
        Ok(LmdbStorage { env: Arc::clone(&self.env), db })
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
        // Attempt delete; ignore NotFound, but propagate other errors
        match wtxn.del(self.db, &key, None) {
            Ok(_) => (),
            Err(lmdb::Error::NotFound) => (),
            Err(e) => return Err(StorageError::Lmdb(e)),
        };
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
enum Operation {
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

//=== MapService (named LMDB databases) ===

use std::sync::Arc;

/// Holds a single LMDB environment and creates named maps (named databases).
pub struct MapService {
    env: Arc<Environment>,
}

impl MapService {
    /// Initialize the environment at `path`, allowing up to `max_maps` named DBs.
    pub fn new(path: impl AsRef<Path>, max_maps: u32) -> Result<Self, StorageError> {
        std::fs::create_dir_all(path.as_ref())?;
        let env = Environment::new()
            .set_max_dbs(max_maps)
            .open(path.as_ref())?;
        Ok(MapService { env: Arc::new(env) })
    }

    /// Get a per-map `LmdbStorage` for the given map name.
    pub fn get_map(&self, name: &str) -> Result<LmdbStorage, StorageError> {
        // Reuse LmdbStorage by building from shared environment
        LmdbStorage::from_env(Arc::clone(&self.env), name)
    }
}

/// Example of using MapService to manage multiple IMaps.
fn example_maps() -> Result<(), StorageError> {
    let service = MapService::new("/data/my-hazelcast", 16)?;
    
    // User map
    let user_map = service.get_map("user")?;
    user_map.put(b"noel::address", b"123 Maple St.")?;
    let addr = user_map.get(b"noel::address")?.unwrap();
    println!("Noel address: {}", String::from_utf8_lossy(&addr));

    // Orders map
    let orders_map = service.get_map("orders")?;
    orders_map.put(b"order123", b"{...}")?;
    Ok(())
}

//=== Async TCP Server with WAL-backed MapService ===

#[cfg(feature = "tcp-server")]
mod server {
    use super::{MapService, StorageError};
    use super::LmdbStorage;
    use super::WalStorage;
    use tokio::{net::TcpListener, io::{AsyncReadExt, AsyncWriteExt}};
    use std::{sync::Arc, path::Path};

    /// Start a TCP server serving multiple named maps with WAL durability.
    ///
    /// **On-the-wire frame format:**
    /// ```text
    /// +------------+-------------------+--------+-----------------------------------------+
    /// | map_len(1) | map_name(bytes)   | op(1)  | payload...                              |
    /// +------------+-------------------+--------+-----------------------------------------+
    /// ```
    ///
    /// - **map_len (1 byte):** length in bytes of the map_name string.
    /// - **map_name (map_len bytes):** UTF-8 encoded name of the map (e.g. "user").
    /// - **op (1 byte):** operation code:
    ///     - `0x01` = GET
    ///     - `0x02` = PUT
    ///     - `0x03` = DELETE
    ///
    /// **Payload by operation:**
    /// - **GET (0x01):**
    ///     4-byte BE key length + key bytes
    /// - **PUT (0x02):**
    ///     4-byte BE key length + 4-byte BE value length + key bytes + value bytes
    /// - **DELETE (0x03):**
    ///     4-byte BE key length + key bytes
    ///
    /// **Responses:**
    /// - GET success: `0x11` + 4-byte BE value length + value bytes
    /// - GET miss:    `0x12`
    /// - PUT success: `0x21`
    /// - DELETE ok:   `0x31`
    /// - Error:       `0xFF`
    ///
    /// **Example:** to GET from map "user" key "noel":
    /// ```text
    /// [0x04][0x75 0x73 0x65 0x72][0x01][0x00 0x00 0x00 0x04][0x6E 0x6F 0x65 0x6C]
    /// ```
    ///  breakdown:
    ///    map_len=4, map_name="user", op=GET, key_len=4, key="noel"
    pub async fn serve(
        addr: &str,
        db_dir: impl AsRef<Path>,
        wal_dir: impl AsRef<Path>,
        map_names: &[&str],
        max_maps: u32,
    ) -> Result<(), StorageError> {
        // Initialize MapService
        let service = MapService::new(db_dir, max_maps)?;
        // Prepare WAL-backed stores for each map
        let mut stores = std::collections::HashMap::new();
        for &name in map_names {
            let lmdb = service.get_map(name)?;
            let wal_path = wal_dir.as_ref().join(format!("{}{}.wal", name, ""));
            let store = WalStorage::new(lmdb, wal_path)?;
            stores.insert(name.to_string(), Arc::new(store));
        }

        let listener = TcpListener::bind(addr).await.expect("bind failed");
        println!("Serving on {}", addr);

        loop {
            let (mut socket, _) = listener.accept().await.expect("accept failed");
            let stores = stores.clone();
            tokio::spawn(async move {
                loop {
                    // Read map namespace
                    let map_len = match socket.read_u8().await {
                        Ok(len) => len as usize,
                        Err(_) => break,
                    };
                    let mut map_buf = vec![0u8; map_len];
                    if socket.read_exact(&mut map_buf).await.is_err() {
                        break;
                    }
                    let map_name = String::from_utf8_lossy(&map_buf);
                    let store = match stores.get(map_name.as_ref()) {
                        Some(s) => s.clone(),
                        None => { let _ = socket.write_u8(0xFE).await; break; }
                    };
                    // Read op code
                    let op = match socket.read_u8().await {
                        Ok(b) => b,
                        Err(_) => break,
                    };
                    match op {
                        0x01 => { // GET
                            let key_len = socket.read_u32().await.unwrap() as usize;
                            let mut key = vec![0u8; key_len];
                            if socket.read_exact(&mut key).await.is_err() { break; }
                            match store.get(&key) {
                                Ok(Some(v)) => {
                                    socket.write_u8(0x11).await.unwrap();
                                    socket.write_u32(v.len() as u32).await.unwrap();
                                    socket.write_all(&v).await.unwrap();
                                }
                                Ok(None) => { socket.write_u8(0x12).await.unwrap(); }
                                Err(_) => { socket.write_u8(0xFF).await.unwrap(); }
                            }
                        }
                        0x02 => { // PUT
                            let klen = socket.read_u32().await.unwrap() as usize;
                            let vlen = socket.read_u32().await.unwrap() as usize;
                            let mut key = vec![0u8; klen];
                            let mut val = vec![0u8; vlen];
                            socket.read_exact(&mut key).await.unwrap();
                            socket.read_exact(&mut val).await.unwrap();
                            if store.put(&key, &val).is_ok() {
                                socket.write_u8(0x21).await.unwrap();
                            } else {
                                socket.write_u8(0xFF).await.unwrap();
                            }
                        }
                        0x03 => { // DELETE
                            let klen = socket.read_u32().await.unwrap() as usize;
                            let mut key = vec![0u8; klen];
                            socket.read_exact(&mut key).await.unwrap();
                            if store.delete(&key).is_ok() {
                                socket.write_u8(0x31).await.unwrap();
                            } else {
                                socket.write_u8(0xFF).await.unwrap();
                            }
                        }
                        _ => {
                            socket.write_u8(0xFD).await.unwrap();
                            break;
                        }
                    }
                }
            });
        }
    }
}

//=== FlatBuffers Schema for TCP Protocol ===

/*
file: imap_protocol.fbs

namespace kvstore;

// Supported operations
enum Operation : byte {
  GET = 1,
  PUT = 2,
  DELETE = 3
}

// Request message
table Request {
  map_name: string;         // e.g. "user"
  op: Operation;
  key: [ubyte];             // key bytes
  value: [ubyte];           // value bytes, present only for PUT
}

// Response message
enum ResponseStatus : byte {
  OK = 1,
  NOT_FOUND = 2,
  ERROR = 255
}

table Response {
  status: ResponseStatus;
  value: [ubyte];           // present only for GET+OK
  message: string;          // optional error message
}

root_type Request;
root_type Response;
*/

//=== Tests ===

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_storage<S: Storage>(store: &S) -> Result<(), StorageError> {
        store.put(b"k1", b"v1")?;
        assert_eq!(store.get(b"k1")?.unwrap(), b"v1".to_vec());
        store.delete(b"k1")?;
        assert!(store.get(b"k1")?.is_none());
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
    fn wal_on_in_memory_basic_works() -> Result<(), StorageError> {
        let dir = tempdir()?;
        let mem = InMemoryStorage::new();
        let wal = WalStorage::new(mem, dir.path().join("wal_mem.log"))?;
        test_storage(&wal)
    }

    #[test]
    fn wal_on_sled_basic_works() -> Result<(), StorageError> {
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
            let mut store1 = WalStorage::new(mem1, &wal_path)?;
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
            let mut store1 = WalStorage::new(sled1, &wal_path)?;
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
