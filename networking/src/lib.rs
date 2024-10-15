#[cxx::bridge(namespace = "posemesh::networking")]
mod posemesh_networking {
    extern "Rust" {
        fn foo(); // TODO: Change ...
    }
}

// TODO: Change ...
pub fn foo() {
    println!("foo");
}
