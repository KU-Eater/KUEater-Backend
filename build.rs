use std::path::PathBuf;
use std::env;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    tonic_build::configure()
        .compile_well_known_types(true)
        .compile_protos(&["protocol/domain.proto"], &["protocol"])
        .unwrap();
    tonic_build::configure()
        .build_client(false)
        .compile_protos(&["protocol/data/main.proto"], &["protocol"])
        .unwrap();
    tonic_build::configure()
        .build_client(false)
        .compile_protos(&["protocol/debug/main.proto"], &["protocol"])
        .unwrap();
}