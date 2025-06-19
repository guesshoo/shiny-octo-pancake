
use serde::{Serialize, Deserialize};
use thiserror::Error;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::fs::{OpenOptions, File};
use std::io::{self, Read, Seek, SeekFrom, Write, IoSlice};
use std::collections::HashMap;


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

pub struct WalStorage<S: Storage> {
    inner: S,
    wal: WriteAheadLog,
}

impl<S: Storage> WalStorage<S> {
    pub fn new(inner: S, wal_path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let mut wal = WriteAheadLog::open(wal_path)?;
        for op in wal.replay()? {
            match op {
                Operation::Put { key, value } => { inner.put(&key, &value)?; }
                Operation::Delete { key }    => { inner.delete(&key)?; }
            }
        }
        Ok(WalStorage { inner, wal })
    }
}

impl<S: Storage> Storage for WalStorage<S> {
    type RoTxn<'txn> = S::RoTxn<'txn>;
    type Value<'txn> = S::Value<'txn>;

    fn begin_read(&self) -> Result<Self::RoTxn<'_>, StorageError> {
        self.inner.begin_read()
    }

    fn get<'txn>(
        &self,
        txn: &'txn Self::RoTxn<'txn>,
        key: &[u8],
    ) -> Result<Option<Self::Value<'txn>>, StorageError> {
        self.inner.get(txn, key)
    }

    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), StorageError> {
        let op = Operation::Put { key: key.to_vec(), value: value.to_vec() };
        self.wal.append_op(&op)?;
        self.inner.put(key, value)
    }

    fn delete(&self, key: &[u8]) -> Result<(), StorageError> {
        let op = Operation::Delete { key: key.to_vec() };
        self.wal.append_op(&op)?;
        self.inner.delete(key)
    }
}