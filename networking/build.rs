fn main() {
    if std::env::var("CARGO_CFG_TARGET_ARCH").unwrap() == "wasm32" {
        return;
    }
    cxx_build::bridge("src/lib.rs")
        .std("c++14")
        .compile("networking");

    println!("cargo:rerun-if-changed=src/lib.rs");
}
