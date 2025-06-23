use std::fs::{ OpenOptions, File};
use std::path::Path;
use std::io::{Read, Seek, SeekFrom, Write, IoSlice};

use crate::storage::StorageError;
use anyhow::Ok;
use serde::{Serialize, Deserialize};

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
        let len: [u8; _] = (payload.len() as u32).to_le_bytes();
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