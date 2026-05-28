use std::path::Path;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let target_dir = Path::new(&out_dir)
        .parent().unwrap()
        .parent().unwrap()
        .parent().unwrap();

    // Copy these files to output dir for dev convenience
    for file in &["config.toml", "README.md"] {
        let src = Path::new(file);
        if src.exists() {
            let dst = target_dir.join(file);
            if let Err(e) = std::fs::copy(src, &dst) {
                if e.kind() != std::io::ErrorKind::PermissionDenied {
                    println!("cargo:warning=Failed to copy {}: {}", file, e);
                }
            }
        }
    }
}
