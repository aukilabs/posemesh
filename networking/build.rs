fn main() {
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    if target_arch == "wasm32" || target_arch == "wasm64" {
        return;
    }

    cxx_build::bridge("src/lib.rs")
        .std("c++14")
        .compile("posemesh_networking");

    println!("cargo:rerun-if-changed=src/lib.rs");
}
