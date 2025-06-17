fn main() {
  prost_build::compile_protos(
    &["proto/ucmsg.proto","proto/snazy.items.proto"], 
  &["proto"]).unwrap();
}
