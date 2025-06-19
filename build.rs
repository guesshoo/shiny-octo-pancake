use std::process::Command;

fn main() {
  prost_build::compile_protos(
    &["proto/uc.msg.proto","proto/snazy.items.proto"], 
  &["proto"]).unwrap();

  compile_fbs()
}



// Run FlatBuffers compiler. (TODO: )
fn compile_fbs() {
    Command::new("flatc")
        .args(&["--rust", "-o", "src/", "schema/monster.fbs"])
        .status()
        .expect("Failed to run flatc");
}