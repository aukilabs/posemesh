use std::{path::{Path, PathBuf}, fs};

fn main() {
    let out_dir = Path::new("./src").join("protobuf");

    let in_dir = PathBuf::from("../protobuf").join("disco");
    // Re-run this build.rs if the protos dir changes (i.e. a new file is added)
    println!("cargo:rerun-if-changed={}", in_dir.to_str().unwrap());

    // Find all *.proto files in the `in_dir` and add them to the list of files
    let mut protos = Vec::new();
    let proto_ext = Some(Path::new("proto").as_os_str());
    fs::create_dir_all(&in_dir).expect("cant create input dir");
    let dir = fs::read_dir(in_dir.clone()).unwrap();

    let mut mod_rs = String::new();
    for entry in dir {
        let path = entry.unwrap().path();
        if path.extension() == proto_ext {
            // Re-run this build.rs if any of the files in the protos dir change
            println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
            protos.push(path.clone());
            mod_rs.push_str(&format!("pub mod {}pb;\n", path.file_stem().unwrap().to_string_lossy().to_string()));
            // mod_rs.push_str(&format!("  include!(concat!(\"{}\", \"/{}.rs\"));\n", out_dir.as_os_str().to_string_lossy().to_string(), path.file_stem().unwrap().to_string_lossy().to_string()));
        }
    }

    // Add common proto files
    let common_dir = PathBuf::from("../protobuf").join("common");
    println!("cargo:rerun-if-changed={}", common_dir.to_str().unwrap());
    
    let common_dir_entries = fs::read_dir(common_dir.clone()).unwrap();
    for entry in common_dir_entries {
        let path = entry.unwrap().path();
        if path.extension() == proto_ext {
            println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
            protos.push(path.clone());
        }
    }
    mod_rs.push_str("pub mod common;\n");

    // Delete all old generated files before re-generating new ones
    if out_dir.exists() {
        std::fs::remove_dir_all(&out_dir).unwrap();
    }
    std::fs::DirBuilder::new().create(&out_dir).unwrap();

    // Configure and run prost-build
    let mut config = prost_build::Config::new();
    config.out_dir(&out_dir);
    config.compile_protos(&protos, &[in_dir, common_dir]).unwrap();

    fs::write(out_dir.join("mod.rs"), mod_rs).expect("Failed to write mod.rs");
}
