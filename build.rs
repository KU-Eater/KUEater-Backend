use std::path::PathBuf;
use std::env;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    tonic_build::configure()
        .compile_protos(&["protocol/domain.proto"], &["protocol"])
        .unwrap();
}