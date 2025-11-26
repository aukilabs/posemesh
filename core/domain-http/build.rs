fn main() {
    // Only generate scaffolding if the "uniffi" feature is enabled
    if std::env::var("CARGO_FEATURE_UNIFFI").is_ok() {
        println!("cargo:rerun-if-changed=src/domain-client.udl");
        uniffi::generate_scaffolding("src/domain-client.udl").expect("failed to generate scaffolding");
    }
}
