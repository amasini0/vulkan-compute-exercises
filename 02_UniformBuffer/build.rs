use std::env;
use std::process::Command;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    // Slang shader
    println!("cargo::rerun-if-changed=shaders/print.slang");
    Command::new("slangc")
        .args(&["shaders/print.slang", "-target", "spirv", "-o"])
        .arg(&format!("{}/print.spv", out_dir))
        .status()
        .unwrap();
}
