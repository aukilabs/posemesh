use std::process::Command;

fn main() {
    let output = Command::new("git").args(&["log", "-1", "--format=%H"]).output().unwrap();
    let commit_id = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=COMMIT_ID={}", commit_id.trim());

    // let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    // cbindgen::Builder::new()
    //   .with_crate(crate_dir.clone())
    //   .with_language(cbindgen::Language::C)
    //   .with_autogen_warning("/* Warning, this file is autogenerated by cbindgen. Don't modify this manually. */")
    //   .with_cpp_compat(true)
    //   .generate()
    //   .map_or_else(
    //     |error| match error {
    //         cbindgen::Error::ParseSyntaxError { .. } => {}
    //         e => panic!("{:?}", e),
    //     },
    //     |bindings| {
    //         bindings.write_to_file("../include/Posemesh/Networking/API.h");
    //     },
    // );
}
