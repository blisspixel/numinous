use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const SEMANTIC_ROOTS: &[&str] = &[
    "crates/audio/src",
    "crates/broadcast/src",
    "crates/core/src",
    "crates/gpu/src",
    "faces/app/src",
    "faces/mcp/src",
];

const SEMANTIC_FILES: &[&str] = &[
    "Cargo.lock",
    "Cargo.toml",
    "rust-toolchain.toml",
    "assets/logo.ico",
    "assets/logo.png",
    "crates/audio/Cargo.toml",
    "crates/broadcast/Cargo.toml",
    "crates/broadcast/build.rs",
    "crates/broadcast/build_support.rs",
    "crates/broadcast/src/consent.rs",
    "crates/broadcast/src/fingerprint.rs",
    "crates/broadcast/src/framing.rs",
    "crates/broadcast/src/hex.rs",
    "crates/broadcast/src/lib.rs",
    "crates/broadcast/src/pairing.rs",
    "crates/broadcast/src/queue.rs",
    "crates/broadcast/src/wire.rs",
    "crates/core/Cargo.toml",
    "crates/gpu/Cargo.toml",
    "data/cairn.txt",
    "faces/app/Cargo.toml",
    "faces/app/build.rs",
    "faces/mcp/Cargo.toml",
];

fn collect_tree(root: &Path, output: &mut Vec<PathBuf>) -> io::Result<()> {
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_dir() {
            collect_tree(&path, output)?;
        } else if kind.is_file()
            && matches!(
                path.extension().and_then(|extension| extension.to_str()),
                Some("rs" | "wgsl")
            )
        {
            output.push(path);
        }
    }
    Ok(())
}

fn tracked_paths(workspace: &Path) -> Option<HashSet<String>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(workspace)
        .args(["ls-files", "-z"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(
        output
            .stdout
            .split(|byte| *byte == 0)
            .filter(|path| !path.is_empty())
            .map(|path| String::from_utf8_lossy(path).replace('\\', "/"))
            .collect(),
    )
}

pub(crate) fn collect_semantic_files(workspace: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = SEMANTIC_FILES
        .iter()
        .map(|relative| workspace.join(relative))
        .collect::<Vec<_>>();
    for root in SEMANTIC_ROOTS {
        collect_tree(&workspace.join(root), &mut files)?;
        println!("cargo:rerun-if-changed={}", workspace.join(root).display());
    }
    let tracked = tracked_paths(workspace);
    files.retain(|path| {
        if !path.is_file() {
            return false;
        }
        let relative = path
            .strip_prefix(workspace)
            .ok()
            .map(|path| path.to_string_lossy().replace('\\', "/"));
        match (&tracked, relative) {
            (Some(tracked), Some(relative)) => {
                tracked.contains(&relative) || SEMANTIC_FILES.contains(&relative.as_str())
            }
            (None, Some(_)) => true,
            _ => false,
        }
    });
    files.sort();
    files.dedup();
    Ok(files)
}

pub(crate) fn semantic_digest<I, P, B>(inputs: I) -> [u8; 32]
where
    I: IntoIterator<Item = (P, B)>,
    P: AsRef<str>,
    B: AsRef<[u8]>,
{
    let mut inputs = inputs
        .into_iter()
        .map(|(path, bytes)| (path.as_ref().to_owned(), bytes.as_ref().to_vec()))
        .collect::<Vec<_>>();
    inputs.sort_by(|left, right| left.0.cmp(&right.0));
    let mut digest = Sha256::new();
    digest.update(b"numinous-replay-build-v2\0");
    for (path, bytes) in inputs {
        digest.update((path.len() as u64).to_le_bytes());
        digest.update(path.as_bytes());
        digest.update((bytes.len() as u64).to_le_bytes());
        digest.update(bytes);
    }
    digest.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::{collect_semantic_files, semantic_digest};
    use std::path::{Path, PathBuf};

    fn workspace() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace layout")
            .to_path_buf()
    }

    #[test]
    fn current_manifest_covers_replay_assets_manifests_and_sources() {
        let workspace = workspace();
        let files = collect_semantic_files(&workspace).expect("collect semantic inputs");
        let relative = files
            .iter()
            .map(|path| {
                path.strip_prefix(&workspace)
                    .expect("workspace path")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect::<Vec<_>>();
        for required in [
            "Cargo.toml",
            "crates/core/Cargo.toml",
            "faces/app/Cargo.toml",
            "faces/mcp/Cargo.toml",
            "data/cairn.txt",
            "assets/logo.png",
            "crates/gpu/src/mandelbrot.wgsl",
            "crates/core/src/lib.rs",
        ] {
            assert!(relative.iter().any(|path| path == required), "{required}");
        }
        assert!(relative.iter().all(|path| !path.contains("/examples/")));
        assert!(relative.iter().all(|path| !path.contains("/tests/")));
    }

    #[test]
    fn digest_is_order_independent_and_sensitive_to_every_input_class() {
        let baseline = [
            ("crates/core/src/lib.rs", b"source".as_slice()),
            ("crates/core/Cargo.toml", b"manifest".as_slice()),
            ("data/cairn.txt", b"asset".as_slice()),
        ];
        let reversed = [baseline[2], baseline[1], baseline[0]];
        assert_eq!(semantic_digest(baseline), semantic_digest(reversed));
        for index in 0..baseline.len() {
            let mut changed = baseline;
            changed[index].1 = b"changed";
            assert_ne!(semantic_digest(baseline), semantic_digest(changed));
        }
    }
}
