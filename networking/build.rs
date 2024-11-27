use std::process::Command;

fn main() {
    let output = Command::new("git").args(&["log", "-1", "--format=%H"]).output().unwrap();
    let commit_id = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=COMMIT_ID={}", commit_id.trim());
}
