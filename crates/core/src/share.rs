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

#[cfg(test)]
mod tests {
    use super::{ShareKind, ShareMeta, sidecar_path, write_share_sidecar};
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
}
