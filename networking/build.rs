fn main() {
    cxx_build::bridge("src/lib.rs")
        .std("c++14")
        .compile("posemesh_networking");

    println!("cargo:rerun-if-changed=src/lib.rs");
}
