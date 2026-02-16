use prost_build;

fn main() {
    prost_build::compile_protos(&["proto/intersect.proto"], &["proto/"]).unwrap();
}
