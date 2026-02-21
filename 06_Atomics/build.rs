use std::process::Command;
use std::env;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let shaders = ["primes"];

    // Recompile if any shader is updated
    shaders
        .iter()
        .for_each(|name| println!("cargo::rerun-if-changed=shaders/{}.slang", name));

    // Closure to compile shaders and place spir-v files in out_dir
    let compile_shader = |name| {
        Command::new("slangc")
            .arg(&format!("shaders/{}.slang", name))
            .args(&["-target", "spirv"])
            .args(&["-o", &format!("{}/{}.spv", out_dir, name)])
            .output()
            .map_err(|_| println!("cargo::error=Failed to run shader compiler."))
    };

    // Compile shaders and print error on failure
    shaders.iter().for_each(|name| {
        let output = compile_shader(name).unwrap();
        if !output.status.success() {
            let stderr = String::from_utf8(output.stderr).unwrap();
            let pattern = format!("{}.slang", name);
            let indices: Vec<_> = stderr.match_indices(&pattern).collect();

            let mut iter = indices.iter().peekable();
            while let Some((i, _)) = iter.next() {
                match iter.peek() {
                    Some((j, _)) => println!("cargo::error={}", &stderr[*i..*j]),
                    None => println!("cargo::error={}", &stderr[*i..]),
                }
            }
        }
    });
}
