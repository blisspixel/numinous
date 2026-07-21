//! Generates a reproducible identity for every declared replay-semantic input.

mod build_support;

use build_support::{collect_semantic_files, semantic_digest};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn main() -> io::Result<()> {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "CARGO_MANIFEST_DIR is unavailable")
    })?);
    let workspace = manifest_dir
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid workspace layout"))?;
    let files = collect_semantic_files(workspace)?;
    let mut inputs = Vec::with_capacity(files.len());
    for path in files {
        println!("cargo:rerun-if-changed={}", path.display());
        let relative = path.strip_prefix(workspace).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("build input escaped workspace: {error}"),
            )
        })?;
        inputs.push((
            relative.to_string_lossy().replace('\\', "/"),
            fs::read(path)?,
        ));
    }
    let bytes = semantic_digest(inputs);
    let array = bytes
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let output = PathBuf::from(
        env::var_os("OUT_DIR")
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "OUT_DIR is unavailable"))?,
    )
    .join("build_semantic_id.rs");
    fs::write(
        output,
        format!(
            "/// SHA-256 identity of replay-semantic source and asset inputs.\n\
             pub const BUILD_SEMANTIC_ID: [u8; 32] = [{array}];\n"
        ),
    )
}
