use std::process::Command;
use std::{env, fs, path::PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));

    let script = manifest_dir.join("tools").join("extract_items.js");
    let output = Command::new("node")
        .arg(script)
        .current_dir(&manifest_dir)
        .output()
        .expect("failed to run extract_items.js");
    if !output.status.success() {
        panic!(
            "extract_items.js failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let gen_path = out_dir.join("items_gen.rs");
    fs::write(&gen_path, output.stdout).expect("write items_gen.rs");
    println!("cargo:rerun-if-changed=tools/extract_items.js");
    println!("cargo:rerun-if-changed=pokemon-showdown/data/items.ts");
    println!("cargo:rustc-env=ITEMS_GEN={}", gen_path.display());
}
