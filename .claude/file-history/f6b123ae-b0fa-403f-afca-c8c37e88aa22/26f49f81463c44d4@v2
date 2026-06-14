/// Compile ans.proto at build time.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use vendored protoc so users don't need to install it separately.
    let protoc_path = protoc_bin_vendored::protoc_bin_path().map_err(|e| {
        Box::<dyn std::error::Error>::from(format!("protoc not found: {e}"))
    })?;
    std::env::set_var("PROTOC", protoc_path);

    tonic_build::configure()
        // Suppress clippy in generated code — we can't edit protobuf output.
        .type_attribute(
            ".",
            "#[allow(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]",
        )
        .build_server(true)
        .build_client(true)
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile_protos(&["../../proto/ans.proto"], &["../../proto"])?;
    println!("cargo:rerun-if-changed=../../proto/ans.proto");
    Ok(())
}
