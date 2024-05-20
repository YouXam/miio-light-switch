use std::env;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

fn copy_dir(
    src: impl AsRef<Path>,
    dst: impl AsRef<Path>,
    mime_types: &mut std::collections::HashSet<(String, String)>,
) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir(
                entry.path(),
                dst.as_ref().join(entry.file_name()),
                mime_types,
            )?;
        } else {
            println!("cargo:rerun-if-changed={}", entry.path().to_str().unwrap());
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;

            if let Some(ext) = entry.path().extension() {
                if let Some(ext_str) = ext.to_str() {
                    let kind = mime_guess::from_path(entry.path());
                    if let Some(mime) = kind.first() {
                        mime_types.insert((ext_str.to_string(), mime.to_string()));
                    }
                }
            }
        }
    }
    Ok(())
}

fn main() {
    embuild::espidf::sysenv::output();

    let status = Command::new("pnpm")
        .args(["run", "build"])
        .current_dir("frontend")
        .status()
        .expect("Failed to build frontend project");

    if !status.success() {
        panic!("Frontend build failed");
    }

    let out_dir = env::var("OUT_DIR").unwrap();
    let dist_dir = Path::new("frontend/dist");
    let out_dist_dir = Path::new(&out_dir).join("frontend");

    let mut mime_types = std::collections::HashSet::new();

    copy_dir(dist_dir, &out_dist_dir, &mut mime_types)
        .expect("Failed to copy frontend dist directory");

    let mime_rs_path = Path::new(&out_dir).join("mime.rs");
    let mut file = File::create(mime_rs_path).expect("Failed to create mime.rs file");

    writeln!(file, "const MIME_TYPES: &[(&str, &str)] = &[").unwrap();
    for (ext, mime) in mime_types {
        println!("mime: {} -> {}", ext, mime);
        writeln!(file, r#"    ("{}", "{}"),"#, ext, mime).unwrap();
    }
    writeln!(file, "];").unwrap();
}
