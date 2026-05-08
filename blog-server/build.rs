fn main() {
    println!("cargo:rerun-if-changed=proto/blog.proto");

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["proto/blog.proto"], &["proto"])
        .expect("failed to compile proto files for blog-server");
}
