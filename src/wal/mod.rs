#![allow(unused_imports)]


use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

// Generated FlatBuffers module (assumes `flatc --rust` output)
mod wal_fb_generated {
    #![allow(dead_code, unused_imports)]
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/schema/wal_generated.rs"));
}

use wal_fb_generated::cache::{ root_as_wal_record, WalRecord, WalRecordArgs};

/// Write-ahead log writer
pub struct WalWriter {
    writer: BufWriter<File>,
    path: String,
    /// Monotonic index counter
    next_index: AtomicU64,
}

impl WalWriter {
    /// Open or create WAL at `path`, initializing next_index by reading last entry
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let path_str = path.as_ref().to_string_lossy().into_owned();
        let file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&path)?;
        let mut reader = BufReader::new(file.try_clone()?);
        // Determine last index by replaying
        let last_index = WalReader::latest_index(&mut reader).unwrap_or(0);
        let writer = BufWriter::new(file);
        Ok(Self {
            writer,
            path: path_str,
            next_index: AtomicU64::new(last_index + 1),
        })
    }

    /// Append a new record, returning its assigned index
    pub fn append(&mut self, term: u64, command: flatbuffers::WIPOffset<WalRecord>) -> io::Result<u64> {
        let idx = self.next_index.fetch_add(1, Ordering::SeqCst);
        // Build FlatBuffer
        let mut fbb = flatbuffers::FlatBufferBuilder::new();
        let record = WalRecord::create(&mut fbb, &WalRecordArgs {
            term,
            index: idx,
            op: command.
        });
        fbb.finish(record, None);
        let data = fbb.finished_data();
        // Write length prefix
        let len = data.len() as u32;
        self.writer.write_all(&len.to_be_bytes())?;
        // Write buffer
        self.writer.write_all(data)?;
        // Persist to disk
        self.writer.flush()?;
        self.writer.get_ref().sync_all()?;
        Ok(idx)
    }

    /// Truncate WAL by rewriting entries after `min_index` into a new file
    pub fn truncate_up_to(&self, min_index: u64) -> io::Result<()> {
        let tmp_path = format!("{}.tmp", self.path);
        let mut reader = BufReader::new(File::open(&self.path)?);
        let mut tmp = BufWriter::new(File::create(&tmp_path)?);

        // Replay entries > min_index
        loop {
            let mut len_buf = [0u8; 4];
            if reader.read_exact(&mut len_buf).is_err() { break; }
            let len = u32::from_be_bytes(len_buf) as usize;
            let mut buf = vec![0u8; len];
            reader.read_exact(&mut buf)?;
            if let Ok(record) = get_root_as_wal_record(&buf) {
                if record.index() > min_index {
                    tmp.write_all(&len_buf)?;
                    tmp.write_all(&buf)?;
                }
            }
        }
        tmp.flush()?;
        tmp.get_ref().sync_all()?;
        std::fs::rename(tmp_path, &self.path)?;
        Ok(())
    }
}

/// Read and replay WAL
pub struct WalReader;

impl WalReader {
    /// Replay all records from the file and invoke the callback
    pub fn replay<P, F>(path: P, mut apply: F) -> io::Result<()>
    where
        P: AsRef<Path>,
        F: FnMut(u64, &WalRecord) -> io::Result<()>,
    {
        let mut reader = BufReader::new(File::open(path)?);
        loop {
            let mut len_buf = [0u8; 4];
            if reader.read_exact(&mut len_buf).is_err() { break; }
            let len = u32::from_be_bytes(len_buf) as usize;
            let mut buf = vec![0u8; len];
            reader.read_exact(&mut buf)?;
            let record = root_as_wal_record(&buf);
            apply(record.index(), &record)?;
        }
        Ok(())
    }

    /// Get latest index in WAL by scanning file (for open)
    pub fn latest_index<R: Read + Seek>(reader: &mut R) -> io::Result<u64> {
        let mut max_idx = 0;
        reader.seek(SeekFrom::Start(0))?;
        loop {
            let mut len_buf = [0u8; 4];
            if reader.read_exact(&mut len_buf).is_err() { break; }
            let len = u32::from_be_bytes(len_buf) as usize;
            let mut buf = vec![0u8; len];
            reader.read_exact(&mut buf)?;
            if let Ok(record) = get_root_as_wal_record(&buf) {
                max_idx = record.index().max(max_idx);
            }
        }
        Ok(max_idx)
    }
}