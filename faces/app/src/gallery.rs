//! The local Gallery: a wall of saved creations discovered from disk, drawn
//! as exact thumbnails, so opening one is a keystroke rather than a filename.
//!
//! Local-first by design (see `docs/CREATOR.md`): the wall is a bounded scan
//! of one folder, the same folder the share keys already write into, so it
//! works before any server exists. Every thumbnail is the creation's own
//! curve over its own saved window at its own saved knob; a wall of previews
//! that drew some other window would be advertising files it cannot deliver.

use std::path::{Path, PathBuf};

use numinous_core::{Expr, Raster, StudioCreation, Surface};

/// The most creations one wall shows. Discovery is newest first, so the cap
/// keeps the wall recent rather than complete; the folder stays the archive.
pub(crate) const MAX_GALLERY_ENTRIES: usize = 24;
/// Fixed columns keep tiles readable at the default window width.
const COLUMNS: usize = 4;

/// One discovered creation: where it lives and what it is.
pub(crate) struct GalleryEntry {
    /// The `.num` file this tile reopens.
    pub path: PathBuf,
    /// The validated creation, exactly as the file holds it.
    pub creation: StudioCreation,
    /// Parsed once at discovery so a wall of tiles does not reparse per frame.
    expr: Expr,
    modified: std::time::SystemTime,
}

fn entry_at(path: PathBuf) -> Option<GalleryEntry> {
    // The shared bounded loader: an oversized or invalid file is skipped, not
    // shown as a broken tile. Symlinks were already skipped by the caller.
    let creation = StudioCreation::from_num_path(&path).ok()?;
    let expr = numinous_core::parse(creation.source()).ok()?;
    // A filesystem that cannot answer for the timestamp must not hide the
    // creation itself: the file opened and parsed, so it belongs on the
    // wall, merely sorted as oldest.
    let modified = std::fs::metadata(&path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
    Some(GalleryEntry {
        path,
        creation,
        expr,
        modified,
    })
}

/// Bounded discovery below one parent folder: top-level `*.num` files plus
/// `creation.num` inside `numinous-share-studio-*` bundle folders. One level,
/// no symlinks, newest first, capped at [`MAX_GALLERY_ENTRIES`].
pub(crate) fn discover(parent: &Path) -> Vec<GalleryEntry> {
    let mut entries: Vec<GalleryEntry> = Vec::new();
    let Ok(dir) = std::fs::read_dir(parent) else {
        return entries;
    };
    for item in dir.flatten() {
        // `file_type` on the entry does not follow links, so a link that
        // points outside the folder is skipped rather than followed.
        let Ok(kind) = item.file_type() else { continue };
        let path = item.path();
        if kind.is_file() {
            if path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("num"))
                && let Some(entry) = entry_at(path)
            {
                entries.push(entry);
            }
        } else if kind.is_dir()
            && item
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with("numinous-share-studio-"))
            && let Some(entry) = entry_at(path.join("creation.num"))
        {
            entries.push(entry);
        }
    }
    // Newest first; the path breaks timestamp ties so the order is stable.
    entries.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| a.path.cmp(&b.path))
    });
    entries.truncate(MAX_GALLERY_ENTRIES);
    entries
}

/// The wall itself: discovered entries and one selection.
pub(crate) struct GalleryPanel {
    entries: Vec<GalleryEntry>,
    selected: usize,
}

impl GalleryPanel {
    /// Discover the wall below `parent`.
    pub(crate) fn open(parent: &Path) -> Self {
        Self {
            entries: discover(parent),
            selected: 0,
        }
    }

    /// How many creations the wall found. The run path reads the count
    /// through its own drawing; this observer exists for the tests.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    /// The creation under the cursor, if the wall is not empty.
    pub(crate) fn selected_creation(&self) -> Option<&StudioCreation> {
        self.entries.get(self.selected).map(|entry| &entry.creation)
    }

    /// Move the selection by whole tiles; the grid edge clamps rather than
    /// wraps, so holding a key parks the cursor instead of spinning it.
    pub(crate) fn move_selection(&mut self, dx: i32, dy: i32) {
        if self.entries.is_empty() {
            return;
        }
        let columns = COLUMNS as i32;
        let current = self.selected as i32;
        let moved = current + dx + dy * columns;
        self.selected = moved.clamp(0, self.entries.len() as i32 - 1) as usize;
    }

    /// Draw the wall: a titled grid of exact thumbnails with one selection.
    pub(crate) fn draw(&self, raster: &mut Raster, width: usize, height: usize) {
        let width = width.min(raster.width());
        let height = height.min(raster.height());
        if width < 40 || height < 40 {
            return;
        }
        let scale = (width as i32 / 450).clamp(1, 3);
        let title = format!("THE GALLERY  {} SAVED", self.entries.len());
        numinous_core::draw_text(raster, &title, 10, 10, scale, '#');
        let footer_top = height as i32 - 16 * scale;
        raster.clear_rows(footer_top, height as i32);
        numinous_core::draw_text(
            raster,
            "ARROWS: CHOOSE   ENTER: OPEN PAUSED   F: FORK   ESC: BACK",
            10,
            height as i32 - 11 * scale,
            scale,
            '#',
        );
        if self.entries.is_empty() {
            numinous_core::draw_text(
                raster,
                "NOTHING SAVED YET",
                10,
                10 + 24 * scale,
                scale + 1,
                '*',
            );
            numinous_core::draw_text(
                raster,
                "F4 IN THE STUDIO SHARES A CREATION HERE",
                10,
                10 + 44 * scale,
                scale,
                '*',
            );
            return;
        }

        let wall_top = 10 + 24 * scale;
        let wall_height = (footer_top - wall_top - 4).max(0) as usize;
        let rows = self.entries.len().div_ceil(COLUMNS).max(1);
        let tile_width = width / COLUMNS;
        let tile_height = (wall_height / rows).max(1);
        for (index, entry) in self.entries.iter().enumerate() {
            let column = index % COLUMNS;
            let row = index / COLUMNS;
            let x0 = (column * tile_width) as i32 + 4;
            let y0 = wall_top + (row * tile_height) as i32 + 2;
            let inner_width = tile_width.saturating_sub(8);
            let inner_height = tile_height.saturating_sub(4);
            // The curve band gives up 14 rows to the caption below it, so a
            // tile shorter than that would underflow the subtraction rather
            // than merely draw badly. The saturating subtraction below is the
            // belt; this guard is the suspenders that keep a drawn tile tall
            // enough to mean something.
            if inner_width < 12 || inner_height < 24 {
                continue;
            }
            if index == self.selected {
                let x1 = x0 + inner_width as i32 - 1;
                let y1 = y0 + inner_height as i32 - 1;
                raster.line(x0, y0, x1, y0, '#');
                raster.line(x0, y1, x1, y1, '#');
                raster.line(x0, y0, x0, y1, '#');
                raster.line(x1, y0, x1, y1, '#');
            }
            draw_tile_curve(
                raster,
                x0 + 2,
                y0 + 2,
                inner_width.saturating_sub(4),
                inner_height.saturating_sub(14),
                entry,
            );
            let label = tile_label(entry, inner_width);
            numinous_core::draw_text(
                raster,
                &label,
                x0 + 2,
                y0 + inner_height as i32 - 10,
                1,
                '*',
            );
        }
    }
}

/// The tile caption: the creation's title when it has one, its expression
/// otherwise, truncated to what the tile can hold.
fn tile_label(entry: &GalleryEntry, inner_width: usize) -> String {
    let source = entry
        .creation
        .title()
        .unwrap_or(entry.creation.source())
        .to_uppercase();
    // The shared 6-pixel glyph advance at scale 1.
    let fits = (inner_width.saturating_sub(4)) / 6;
    if source.chars().count() <= fits {
        source
    } else {
        source
            .chars()
            .take(fits.saturating_sub(1))
            .collect::<String>()
            + "~"
    }
}

/// One thumbnail: the creation's own curve over its own saved window at its
/// own saved knob, auto-scaled to the tile band. This is a preview surface;
/// the parity-pinned framing lives in `studio_render` where the full panel
/// draws.
fn draw_tile_curve(
    raster: &mut Raster,
    x0: i32,
    y0: i32,
    tile_width: usize,
    tile_height: usize,
    entry: &GalleryEntry,
) {
    if tile_width < 2 || tile_height < 8 {
        return;
    }
    let (xmin, xmax) = (entry.creation.xmin(), entry.creation.xmax());
    let a = entry.creation.a();
    let span = xmax - xmin;
    let points: Vec<(usize, f64)> = (0..tile_width)
        .filter_map(|column| {
            let x = xmin + span * column as f64 / (tile_width as f64 - 1.0);
            let value = numinous_core::eval(&entry.expr, x, a);
            value.is_finite().then_some((column, value))
        })
        .collect();
    if points.is_empty() {
        return;
    }
    let ymin = points
        .iter()
        .map(|point| point.1)
        .fold(f64::INFINITY, f64::min);
    let ymax = points
        .iter()
        .map(|point| point.1)
        .fold(f64::NEG_INFINITY, f64::max);
    let yspan = (ymax - ymin).max(1e-9);
    let mut previous: Option<(i32, i32)> = None;
    for (column, value) in points {
        let x = x0 + column as i32;
        let y = y0 + ((1.0 - (value - ymin) / yspan) * (tile_height as f64 - 1.0)) as i32;
        if let Some((px, py)) = previous {
            raster.line(px, py, x, y, '*');
        }
        previous = Some((x, y));
    }
}

#[cfg(test)]
mod tests {
    use super::{GalleryPanel, MAX_GALLERY_ENTRIES, discover};
    use numinous_core::{Raster, StudioCreation};
    use std::path::Path;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("numinous-gallery-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    fn save(dir: &Path, name: &str, source: &str) {
        let creation = StudioCreation::new(source, -2.0, 2.0, 0.5).expect("creation");
        std::fs::write(dir.join(name), creation.to_num_file()).expect("write");
    }

    #[test]
    fn discovery_finds_files_and_bundles_and_skips_what_is_not_a_creation() {
        let dir = scratch("discover");
        save(&dir, "one.num", "sin(x)");
        save(&dir, "two.NUM", "x*x");
        let bundle = dir.join("numinous-share-studio-42-aa");
        std::fs::create_dir(&bundle).expect("bundle dir");
        save(&bundle, "creation.num", "cos(x)");
        // Not creations: wrong extension, invalid body, oversized body, and a
        // folder without the bundle name.
        std::fs::write(dir.join("note.txt"), "not a capsule").expect("txt");
        std::fs::write(dir.join("bad.num"), "not a capsule").expect("bad");
        std::fs::write(
            dir.join("huge.num"),
            "x".repeat(numinous_core::MAX_SHARE_INPUT_BYTES + 1),
        )
        .expect("huge");
        let plain = dir.join("plain-folder");
        std::fs::create_dir(&plain).expect("plain dir");
        save(&plain, "creation.num", "sin(x)+1");

        let entries = discover(&dir);
        let sources: Vec<&str> = entries
            .iter()
            .map(|entry| entry.creation.source())
            .collect();
        assert_eq!(entries.len(), 3, "found {sources:?}");
        assert!(sources.contains(&"sin(x)"));
        assert!(sources.contains(&"x*x"));
        assert!(sources.contains(&"cos(x)"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn discovery_is_newest_first_and_capped() {
        let dir = scratch("cap");
        for index in 0..MAX_GALLERY_ENTRIES + 3 {
            save(&dir, &format!("c{index:03}.num",), "sin(x)");
        }
        let entries = discover(&dir);
        assert_eq!(entries.len(), MAX_GALLERY_ENTRIES, "the wall is capped");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn selection_moves_by_tiles_and_clamps_at_the_edges() {
        let dir = scratch("select");
        for index in 0..6 {
            save(&dir, &format!("c{index}.num"), "sin(x)");
        }
        let mut panel = GalleryPanel::open(&dir);
        assert_eq!(panel.len(), 6);
        panel.move_selection(-1, 0);
        assert!(panel.selected_creation().is_some(), "clamped, not wrapped");
        panel.move_selection(1, 0);
        panel.move_selection(0, 1);
        let below = panel.selected_creation().expect("moved down a row");
        assert_eq!(below.source(), "sin(x)");
        panel.move_selection(0, 9);
        panel.move_selection(9, 0);
        assert!(panel.selected_creation().is_some(), "edges clamp");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_full_wall_survives_every_short_window() {
        // A full wall at a short window drives the per-tile height through
        // the band where the caption's 14 reserved rows once underflowed the
        // subtraction. Sweep the whole range, because the panicking height
        // depends on the row count and picking one lucky value would prove
        // nothing next time the layout changes.
        let dir = scratch("short");
        for index in 0..MAX_GALLERY_ENTRIES {
            save(&dir, &format!("c{index:02}.num"), "sin(x)");
        }
        let panel = GalleryPanel::open(&dir);
        for height in 40..=220 {
            let mut wall = Raster::new(600, height);
            panel.draw(&mut wall, 600, height);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_wall_draws_tiles_and_the_empty_wall_says_how_to_fill_it() {
        let dir = scratch("draw");
        save(&dir, "one.num", "sin(x)");
        let panel = GalleryPanel::open(&dir);
        let mut wall = Raster::new(600, 400);
        panel.draw(&mut wall, 600, 400);
        assert!(wall.lit_count() > 100, "a wall with a tile has ink");

        let empty_dir = scratch("draw-empty");
        let empty = GalleryPanel::open(&empty_dir);
        let mut invitation = Raster::new(600, 400);
        empty.draw(&mut invitation, 600, 400);
        assert!(invitation.lit_count() > 50, "the empty wall invites");

        let mut tiny = Raster::new(8, 8);
        empty.draw(&mut tiny, 8, 8);

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&empty_dir);
    }
}
