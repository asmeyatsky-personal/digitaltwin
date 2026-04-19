fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = std::path::Path::new("../../../contracts");
    let proto_file = proto_root.join("digitaltwin/learning/v1/learning.proto");
    println!("cargo:rerun-if-changed={}", proto_file.display());
    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .compile_protos(
            &[proto_file.to_str().ok_or("bad path")?],
            &[proto_root.to_str().ok_or("bad path")?],
        )?;
    Ok(())
}
