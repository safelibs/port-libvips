use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const SONAME: &str = "libvips.so.42";
const GENERATED_DIR: &str = "generated";
const EXPORT_MAP_NAME: &str = "export.map";
const CORE_BOOTSTRAP_PATH: &str = "reference/abi/core-bootstrap.symbols";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={CORE_BOOTSTRAP_PATH}");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let generated_dir = manifest_dir.join(GENERATED_DIR);
    fs::create_dir_all(&generated_dir).expect("create generated dir");

    let export_map_path = generated_dir.join(EXPORT_MAP_NAME);
    let export_map = render_export_map(&manifest_dir.join(CORE_BOOTSTRAP_PATH));
    fs::write(&export_map_path, export_map).expect("write export map");

    println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,{SONAME}");
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,--version-script={}",
        export_map_path.display()
    );
    println!("cargo:rustc-cdylib-link-arg=-Wl,--gc-sections");
    println!("cargo:rustc-cdylib-link-arg=-Wl,--no-undefined");
    println!("cargo:rustc-cdylib-link-arg=-Wl,--exclude-libs,ALL");
}

fn render_export_map(symbols_path: &Path) -> String {
    let mut lines = vec!["VIPS_42 {".to_owned()];

    let symbols = read_symbols(symbols_path);
    if !symbols.is_empty() {
        lines.push("  global:".to_owned());
        for symbol in symbols {
            lines.push(format!("    {symbol};"));
        }
    }

    lines.push("  local:".to_owned());
    lines.push("    *;".to_owned());
    lines.push("};".to_owned());
    lines.push(String::new());
    lines.join("\n")
}

fn read_symbols(path: &Path) -> Vec<String> {
    let Ok(contents) = fs::read_to_string(path) else {
        return Vec::new();
    };

    contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .collect()
}

