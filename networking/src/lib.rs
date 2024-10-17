#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
#[cxx::bridge(namespace = "psm::net")]
mod posemesh_networking {
    extern "Rust" {
        fn foo(); // TODO: Change ...
    }
}

// TODO: Change ...
pub fn foo() {
    println!("foo");
}

#[cfg(any(target_arch = "wasm32", target_arch = "wasm64"))]
#[no_mangle]
pub extern "C" fn psm_net_foo() { // TODO: Change ...
    foo();
}
