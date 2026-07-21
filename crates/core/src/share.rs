//! Share packaging helpers (panel Share v1 polish).
//!
//! Still PNG and short looping APNG remain the binary formats. This module
//! writes a small sidecar text note so a shared file carries room id, era,
//! kind, and version without inventing a container format.

use std::io::Write;
use std::path::{Path, PathBuf};

/// What kind of share artifact was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShareKind {
    /// Still PNG postcard.
    Postcard,
    /// Short looping APNG.
    Loop,
}

impl ShareKind {
    /// Stable label for the sidecar.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Postcard => "postcard",
            Self::Loop => "loop",
        }
    }
}

/// Metadata written next to a share export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareMeta {
    /// Catalog room id.
    pub room_id: String,
    /// Visual era name.
    pub era: String,
    /// Postcard or loop.
    pub kind: ShareKind,
    /// Package version string (usually `CARGO_PKG_VERSION`).
    pub version: String,
}

impl ShareMeta {
    /// Sidecar text body.
    #[must_use]
    pub fn to_text(&self) -> String {
        format!(
            "numinous-share 1\nroom {}\nera {}\nkind {}\nversion {}\n",
            self.room_id,
            self.era,
            self.kind.label(),
            self.version
        )
    }
}

/// Path of the sidecar file for a share binary path.
#[must_use]
pub fn sidecar_path(share_path: &Path) -> PathBuf {
    let mut path = share_path.to_path_buf();
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("bin");
    path.set_extension(format!("{ext}.share.txt"));
    path
}

/// Write a sidecar next to `share_path`. Best-effort: returns the sidecar path.
///
/// # Errors
/// Propagates filesystem errors from create/write.
pub fn write_share_sidecar(share_path: &Path, meta: &ShareMeta) -> std::io::Result<PathBuf> {
    let path = sidecar_path(share_path);
    let mut file = std::fs::File::create(&path)?;
    file.write_all(meta.to_text().as_bytes())?;
    Ok(path)
}

/// A share bundle: one directory holding still, loop, and a human README.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareBundleMeta {
    /// Catalog room id.
    pub room_id: String,
    /// Visual era name.
    pub era: String,
    /// Package version string.
    pub version: String,
    /// Optional variation seed recorded for replay.
    pub variation: u64,
}

impl ShareBundleMeta {
    /// Manifest body for `README.share.txt` inside a share folder.
    #[must_use]
    pub fn readme_text(&self, has_postcard: bool, has_loop: bool) -> String {
        let mut lines = vec![
            "numinous-share-bundle 1".to_string(),
            format!("room {}", self.room_id),
            format!("era {}", self.era),
            format!("variation {}", self.variation),
            format!("version {}", self.version),
            String::new(),
            "Contents:".to_string(),
        ];
        if has_postcard {
            lines.push("- postcard.png  still frame of the visit".to_string());
        }
        if has_loop {
            lines.push("- loop.png      short looping APNG of one phase cycle".to_string());
        }
        lines.push("- README.share.txt  this note".to_string());
        lines.push(String::new());
        lines.push("Open the PNG files in any browser or image viewer.".to_string());
        lines.push("APNG loops in modern browsers and some image hosts.".to_string());
        lines.join("\n") + "\n"
    }
}

/// Sanitized folder name for a share bundle under `parent`.
#[must_use]
pub fn share_bundle_dir(parent: &Path, room_id: &str, stamp: u64) -> PathBuf {
    let safe: String = room_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let safe = if safe.is_empty() {
        "room".to_string()
    } else {
        safe
    };
    parent.join(format!("numinous-share-{safe}-{stamp}"))
}

/// Write the bundle README into an existing share directory.
///
/// # Errors
/// Propagates filesystem errors.
pub fn write_share_bundle_readme(
    dir: &Path,
    meta: &ShareBundleMeta,
    has_postcard: bool,
    has_loop: bool,
) -> std::io::Result<PathBuf> {
    let path = dir.join("README.share.txt");
    let mut file = std::fs::File::create(&path)?;
    file.write_all(meta.readme_text(has_postcard, has_loop).as_bytes())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::{
        ShareBundleMeta, ShareKind, ShareMeta, share_bundle_dir, sidecar_path,
        write_share_bundle_readme, write_share_sidecar,
    };
    use std::path::PathBuf;

    #[test]
    fn sidecar_path_appends_share_txt() {
        let path = PathBuf::from("out/room-loop.png");
        assert_eq!(
            sidecar_path(&path),
            PathBuf::from("out/room-loop.png.share.txt")
        );
    }

    #[test]
    fn sidecar_round_trips_on_disk() {
        let dir =
            std::env::temp_dir().join(format!("numinous-share-sidecar-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let share = dir.join("mandelbrot.png");
        std::fs::write(&share, b"fake").expect("share file");
        let meta = ShareMeta {
            room_id: "mandelbrot".into(),
            era: "Modern".into(),
            kind: ShareKind::Postcard,
            version: "0.2.0-alpha.1".into(),
        };
        let side = write_share_sidecar(&share, &meta).expect("sidecar");
        let text = std::fs::read_to_string(&side).expect("read");
        assert!(text.contains("room mandelbrot"));
        assert!(text.contains("kind postcard"));
        assert!(text.contains("era Modern"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn share_bundle_dir_sanitizes_room_ids() {
        let parent = PathBuf::from("/tmp");
        let dir = share_bundle_dir(&parent, "times tables!", 42);
        assert!(
            dir.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .contains("times-tables-")
        );
        assert!(dir.to_string_lossy().ends_with("42") || dir.to_string_lossy().contains("-42"));
    }

    #[test]
    fn bundle_readme_lists_contents() {
        let dir =
            std::env::temp_dir().join(format!("numinous-share-bundle-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let meta = ShareBundleMeta {
            room_id: "lorenz".into(),
            era: "Phosphor".into(),
            version: "0.2.0-alpha.1".into(),
            variation: 3,
        };
        let readme = write_share_bundle_readme(&dir, &meta, true, true).expect("readme");
        let text = std::fs::read_to_string(readme).expect("read");
        assert!(text.contains("postcard.png"));
        assert!(text.contains("loop.png"));
        assert!(text.contains("room lorenz"));
        assert!(text.contains("variation 3"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
