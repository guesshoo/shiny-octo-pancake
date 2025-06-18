fn main() {
  prost_build::compile_protos(
    &["proto/uc.msg.proto","proto/snazy.items.proto"], 
  &["proto"]).unwrap();
}
