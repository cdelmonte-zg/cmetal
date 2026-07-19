//! Packages the curriculum (exercises, obfuscated solutions, headers,
//! info.toml) into a compressed archive embedded in the binary, so that
//! `clings init` can materialize a workspace without the git repository.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Directories shipped in the curriculum archive, with the file
/// extension each one contributes.
const CURRICULUM_DIRS: &[(&str, &str)] =
    &[("exercises", "c"), ("solutions", "enc"), ("include", "h")];

fn collect_files(dir: &Path, ext: &str, out: &mut Vec<PathBuf>, skipped: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("curriculum dir must be readable") {
        let path = entry.expect("readable dir entry").path();
        if path.is_dir() {
            collect_files(&path, ext, out, skipped);
        } else if path.extension().and_then(|e| e.to_str()) == Some(ext) {
            out.push(path);
        } else {
            skipped.push(path);
        }
    }
}

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let archive_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("curriculum.tar.gz");

    let file = fs::File::create(&archive_path).expect("create curriculum archive");
    let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut builder = tar::Builder::new(encoder);

    let info = root.join("info.toml");
    builder
        .append_path_with_name(&info, "info.toml")
        .expect("append info.toml");
    println!("cargo:rerun-if-changed=info.toml");

    for (dir, ext) in CURRICULUM_DIRS {
        let mut files = Vec::new();
        let mut skipped = Vec::new();
        collect_files(&root.join(dir), ext, &mut files, &mut skipped);
        files.sort();
        // A curriculum file the archive silently drops would exist in
        // git clones but never reach init-created workspaces — scream.
        // (solutions/*.c are contributors' local plaintext, expected.)
        for path in skipped {
            let rel = path.strip_prefix(&root).unwrap();
            if *dir == "solutions" && rel.extension().and_then(|e| e.to_str()) == Some("c") {
                continue;
            }
            println!(
                "cargo:warning=curriculum file NOT packaged (unexpected extension): {}",
                rel.display()
            );
        }
        for path in files {
            let rel = path.strip_prefix(&root).unwrap();
            builder
                .append_path_with_name(&path, rel)
                .unwrap_or_else(|e| panic!("append {}: {e}", rel.display()));
            println!("cargo:rerun-if-changed={}", rel.display());
        }
        println!("cargo:rerun-if-changed={dir}");
    }

    builder
        .into_inner()
        .expect("finish tar")
        .finish()
        .expect("finish gzip");
}
