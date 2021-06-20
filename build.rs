fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::compile_protos(&["src/proto/init.proto"], &["src/"])?;
    prost_build::compile_protos(&["src/proto/client.proto"], &["src/"])?;
    prost_build::compile_protos(&["src/proto/server.proto"], &["src/"])?;
    Ok(())
}
