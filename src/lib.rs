pub mod frame;
pub mod wal;
// pub mod storage;
pub mod kv;
pub mod snazzy {
    pub mod items {
        include!(concat!(env!("OUT_DIR"), "/snazzy.items.rs"));
    }
}


pub mod uc {
    pub mod proto {
         include!(concat!(env!("OUT_DIR"), "/uc.msg.rs"));
    }
}
use snazzy::items;


/// Returns a large shirt of the specified color
pub fn create_large_shirt(color: String) -> items::Shirt {
    let mut shirt = items::Shirt::default();
    shirt.color = color;
    shirt.set_size(items::shirt::Size::Large);
    shirt
}

