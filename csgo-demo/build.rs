use std::{ffi::OsStr, fs};

fn main() {
    let proto_dir = "proto";
    println!("cargo:rerun-if-changed={proto_dir}");

    let proto_files = fs::read_dir(proto_dir)
        .unwrap()
        .filter_map(|res| res.map(|e| e.path()).ok())
        .filter(|p| p.extension() == Some(OsStr::new("proto")))
        .collect::<Vec<_>>();
    protobuf_codegen::Codegen::new()
        .pure()
        .include(proto_dir)
        .inputs(proto_files)
        .cargo_out_dir("proto")
        .run_from_script();
}
