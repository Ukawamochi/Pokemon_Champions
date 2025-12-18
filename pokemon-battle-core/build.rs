use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = Path::new(&manifest_dir)
        .parent()
        .unwrap_or_else(|| Path::new(&manifest_dir));
    let extract_script = workspace_root.join("tools").join("extract_data.js");
    println!("cargo:rerun-if-changed={}", extract_script.display());
    let showdown_data = workspace_root
        .join("pokemon-showdown")
        .join("data");
    for file in &[
        "pokedex.ts",
        "moves.ts",
        "abilities.ts",
        "items.ts",
        "typechart.ts",
    ] {
        println!(
            "cargo:rerun-if-changed={}",
            showdown_data.join(file).display()
        );
    }
    let output_dir = Path::new(&manifest_dir).join("src").join("data");
    let status = Command::new("node")
        .arg(extract_script.as_os_str())
        .arg(output_dir.as_os_str())
        .status()
        .expect("failed to run tools/extract_data.js");
    if !status.success() {
        panic!("tools/extract_data.js failed");
    }
}
