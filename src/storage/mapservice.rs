use std::sync::Arc;
use lmdb::Environment;
use std::path::Path;

use crate::storage::{
    lmdb_store,
    *
};
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
    pub fn get_map(&self, name: &str) -> Result<lmdb_store::LmdbStorage, StorageError> {
        // Reuse LmdbStorage by building from shared environment
        lmdb_store::LmdbStorage::from_env(Arc::clone(&self.env), name)
    }
}

/*
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
*/