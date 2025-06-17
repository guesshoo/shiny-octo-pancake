use cache_rs::*;

use crate::snazzy::items;

fn main() {
    let get_req = create_large_shirt("Green".to_string());
    println!("{:?}", get_req);

    let mut anotherShirt = items::Shirt::default();
    anotherShirt.color = "black".to_string();
    println!("{:?}",  anotherShirt);
    println!("Hello, world!");
}
