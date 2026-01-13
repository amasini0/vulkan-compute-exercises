use std::env;
use std::process::Command;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    // GLSL shader
    /*
    println!("cargo::rerun-if-changed=shaders/hello.comp");
    Command::new("glslangValidator")
        .args(&["-V", "shaders/hello.comp", "-o"])
        .arg(&format!("{}/hello.spv", out_dir))
        .status().unwrap();
     */

    // Slang shader
    println!("cargo::rerun-if-changed=shaders/hello.slang");
    Command::new("slangc")
        .args(&["shaders/hello.slang", "-target", "spirv", "-o"])
        .arg(&format!("{}/hello.spv", out_dir))
        .status()
        .unwrap();
}
