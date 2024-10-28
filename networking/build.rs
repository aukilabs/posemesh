fn main() {
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    if target_arch != "wasm32" && target_arch != "wasm64" {
        println!("cargo:rustc-cfg=feature=\"non_web_platform\"");
    } else {
        println!("cargo:rustc-cfg=feature=\"web_platform\"");
    }
}
