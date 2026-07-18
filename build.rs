fn main() -> std::io::Result<()> {
    tonic_prost_build::compile_protos("proto-files/backend.proto")
}