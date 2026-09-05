//! The local Gallery: a wall of saved creations discovered from disk, drawn
//! as exact thumbnails, so opening one is a keystroke rather than a filename.
//!
//! Local-first by design (see `docs/CREATOR.md`): the wall is a bounded scan
//! of one folder, the same folder the share keys already write into, so it
//! works before any server exists. Every thumbnail is the creation's own
//! curve over its own saved window at its own saved knob; a wall of previews
//! that drew some other window would be advertising files it cannot deliver.

use std::path::{Path, PathBuf};

use numinous_core::{Raster, StudioCreation, StudioKind, StudioProgram, Surface};

/// The most creations one wall shows. Discovery is newest first, so the cap
/// keeps the wall recent rather than complete; the folder stays the archive.
pub(crate) const MAX_GALLERY_ENTRIES: usize = 24;
/// Fixed columns keep tiles readable at the default window width.
const COLUMNS: usize = 4;

/// One discovered creation: where it lives, what it is, and where it sits in
/// the wall's own remix tree.
pub(crate) struct GalleryEntry {
    /// The `.num` file this tile reopens.
    pub path: PathBuf,
    /// The validated creation, exactly as the file holds it.
    pub creation: StudioCreation,
    /// Parsed once at discovery so a wall of tiles does not reparse per frame.
    program: StudioProgram,
    modified: std::time::SystemTime,
    /// The wall index of the creation this one descends from, when that
    /// exact creation is on the wall too. Matched by canonical link, so a
    /// parent that was edited and re-shared is a different creation, not a
    /// false ancestor.
    parent: Option<usize>,
    /// How many creations on this wall descend from this one.
    remixes: usize,
}

/// Where the selected creation's parent is, for keys and copy that must not
/// blur three different answers into one silent nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParentStatus {
    /// The creation records no descent at all.
    NoLineage,
    /// It descends from a creation that is not on this wall.
    Absent,
    /// Its parent is the wall entry at this index.
    Local(usize),
}

fn entry_at(path: PathBuf) -> Option<GalleryEntry> {
    // The shared bounded loader: an oversized or invalid file is skipped, not
    // shown as a broken tile. Symlinks were already skipped by the caller.
    let creation = StudioCreation::from_num_path(&path).ok()?;
    let program = creation.program().ok()?;
    // A filesystem that cannot answer for the timestamp must not hide the
    // creation itself: the file opened and parsed, so it belongs on the
    // wall, merely sorted as oldest.
    let modified = std::fs::metadata(&path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
    Some(GalleryEntry {
        path,
        creation,
        program,
        modified,
        parent: None,
        remixes: 0,
    })
}

/// Resolve the wall's own remix tree: each entry's recorded parent link is
/// matched against every other entry's canonical link, in place, after the
/// sort has settled the indices.
fn resolve_lineage(entries: &mut [GalleryEntry]) {
    let links: Vec<String> = entries
        .iter()
        .map(|entry| entry.creation.to_link())
        .collect();
    for index in 0..entries.len() {
        let Some(descends) = entries[index].creation.descends() else {
            continue;
        };
        // Neither itself nor a sibling. An unedited fork of an untitled
        // creation carries its parent's exact canonical link as its own, so
        // a self-parent would credit the remix to the wrong tile and give D
        // a step that goes nowhere, and two such forks on one wall would
        // adopt each other: both wearing a remix badge, the real parent
        // uncredited, and D walking a circle between them forever. A
        // candidate that descends from the same creation stands beside this
        // one, not above it.
        let parent = links.iter().enumerate().position(|(candidate, link)| {
            candidate != index
                && link == descends
                && entries[candidate].creation.descends() != Some(descends)
        });
        entries[index].parent = parent;
        if let Some(parent_index) = parent {
            entries[parent_index].remixes += 1;
        }
    }
}

/// Bounded discovery below one parent folder: top-level `*.num` files plus
/// `creation.num` inside `numinous-share-studio-*` bundle folders. One level,
/// no symlinks, newest first, capped at [`MAX_GALLERY_ENTRIES`].
///
/// Returns `None` when the folder itself cannot be read: an unreadable wall
/// is a fact to report, not an empty one, and the caller must not tell the
/// player their creations do not exist. Individual entries that fail to read
/// mid-scan are still skipped one by one, so a single broken file cannot
/// hide the rest of the wall.
pub(crate) fn discover(parent: &Path) -> Option<Vec<GalleryEntry>> {
    let mut entries: Vec<GalleryEntry> = Vec::new();
    let dir = std::fs::read_dir(parent).ok()?;
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
        {
            // The capsule inside the bundle must be a regular file too:
            // `symlink_metadata` does not follow links, so a planted link
            // cannot walk the wall outside the folder it claims to scan.
            let capsule = path.join("creation.num");
            let is_regular_file = std::fs::symlink_metadata(&capsule)
                .map(|metadata| metadata.file_type().is_file())
                .unwrap_or(false);
            if is_regular_file && let Some(entry) = entry_at(capsule) {
                entries.push(entry);
            }
        }
    }
    // Newest first; the path breaks timestamp ties so the order is stable.
    entries.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| a.path.cmp(&b.path))
    });
    entries.truncate(MAX_GALLERY_ENTRIES);
    resolve_lineage(&mut entries);
    Some(entries)
}

/// The wall itself: discovered entries and one selection.
pub(crate) struct GalleryPanel {
    entries: Vec<GalleryEntry>,
    selected: usize,
    /// Whether the folder could be read at all. An unreadable folder must
    /// not wear the empty wall's copy: NOTHING SAVED YET is a claim.
    readable: bool,
}

impl GalleryPanel {
    /// Discover the wall below `parent`.
    pub(crate) fn open(parent: &Path) -> Self {
        let discovered = discover(parent);
        Self {
            readable: discovered.is_some(),
            entries: discovered.unwrap_or_default(),
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

    /// Where the selected creation's parent is.
    pub(crate) fn parent_status(&self) -> ParentStatus {
        let Some(entry) = self.entries.get(self.selected) else {
            return ParentStatus::NoLineage;
        };
        match (entry.creation.descends(), entry.parent) {
            (None, _) => ParentStatus::NoLineage,
            (Some(_), None) => ParentStatus::Absent,
            (Some(_), Some(parent)) => ParentStatus::Local(parent),
        }
    }

    /// Walk one step up the remix tree: move the cursor to the selected
    /// creation's parent when that parent is on the wall. Returns whether
    /// the cursor moved, so the caller can say why it did not.
    pub(crate) fn select_parent(&mut self) -> bool {
        if let ParentStatus::Local(parent) = self.parent_status() {
            self.selected = parent;
            true
        } else {
            false
        }
    }

    /// Move the selection by whole tiles; the grid edge clamps rather than
    /// wraps, so holding a key parks the cursor instead of spinning it.
    ///
    /// Each axis clamps inside the grid, not the flat index: a horizontal
    /// move at a row edge parks in place rather than snaking into the next
    /// row, and a vertical move keeps its column as far as the last,
    /// possibly partial, row allows.
    pub(crate) fn move_selection(&mut self, dx: i32, dy: i32) {
        if self.entries.is_empty() {
            return;
        }
        let columns = COLUMNS as i32;
        let last = self.entries.len() as i32 - 1;
        let row = self.selected as i32 / columns;
        let column = self.selected as i32 % columns;
        let last_row = last / columns;
        let target_row = (row + dy).clamp(0, last_row);
        let row_last_column = if target_row == last_row {
            last - last_row * columns
        } else {
            columns - 1
        };
        let target_column = (column + dx).clamp(0, row_last_column);
        self.selected = (target_row * columns + target_column) as usize;
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
            "ARROWS: CHOOSE   ENTER: OPEN PAUSED   F: FORK   D: PARENT   ESC: BACK",
            10,
            height as i32 - 11 * scale,
            scale,
            '#',
        );
        if !self.readable {
            // An unreadable folder is a fact, not an empty wall: claiming
            // NOTHING SAVED YET here would tell the player their creations
            // do not exist when the folder simply refused to answer.
            numinous_core::draw_text(
                raster,
                "THE FOLDER COULD NOT BE READ",
                10,
                10 + 24 * scale,
                scale + 1,
                '*',
            );
            numinous_core::draw_text(
                raster,
                "YOUR CREATIONS MAY STILL EXIST  CHECK ITS PERMISSIONS",
                10,
                10 + 44 * scale,
                scale,
                '*',
            );
            return;
        }
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

        // One line above the footer belongs to the selected tile's lineage,
        // so walking the tree reads as a sentence, not a guess.
        let lineage_top = footer_top - 12 * scale;
        let lineage = match self.parent_status() {
            ParentStatus::NoLineage => None,
            ParentStatus::Absent => Some("DESCENDS FROM A CREATION NOT ON THIS WALL".to_string()),
            ParentStatus::Local(parent) => self.entries.get(parent).map(|entry| {
                format!(
                    "DESCENDS FROM {}  D: GO THERE",
                    tile_label(entry, width.saturating_sub(180))
                )
            }),
        };
        if let Some(line) = &lineage {
            numinous_core::draw_text(raster, line, 10, lineage_top, scale, '*');
        }

        let wall_top = 10 + 24 * scale;
        let wall_height = (lineage_top - 4 - wall_top).max(0) as usize;
        let rows = self.entries.len().div_ceil(COLUMNS).max(1);
        let tile_width = width / COLUMNS;
        let tile_height = (wall_height / rows).max(1);
        if tile_height.saturating_sub(4) < 24 || tile_width.saturating_sub(8) < 12 {
            // Too short for thumbnails. A blank wall with a live, invisible
            // cursor would let Enter open whichever unseen creation it lands
            // on, so the selection is named instead of drawn.
            numinous_core::draw_text(
                raster,
                "WINDOW TOO SHORT FOR THUMBNAILS",
                10,
                wall_top,
                scale,
                '*',
            );
            if let Some(entry) = self.entries.get(self.selected) {
                let label = format!(
                    "CHOSEN {} OF {}: {}",
                    self.selected + 1,
                    self.entries.len(),
                    tile_label(entry, width.saturating_sub(20))
                );
                numinous_core::draw_text(raster, &label, 10, wall_top + 14 * scale, scale, '#');
            }
            return;
        }
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
            if entry.remixes > 0 {
                // The badge is the parent's point of pride: remixing is
                // honoring, and the wall says so where the tree is visible.
                let badge = format!("R{}", entry.remixes);
                let badge_x = x0 + inner_width as i32 - numinous_core::text_width(&badge, 1) - 3;
                numinous_core::draw_text(raster, &badge, badge_x, y0 + 3, 1, '#');
            }
        }
    }
}

/// The tile caption: the creation's title when it has one, its expression
/// otherwise, truncated to what the tile can hold.
fn tile_label(entry: &GalleryEntry, inner_width: usize) -> String {
    let source = entry
        .creation
        .title()
        .map(str::to_string)
        .unwrap_or_else(|| entry.creation.editor_source())
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
/// own saved knob. Parametric paths preserve equal coordinate units within
/// the tile band through the same fit the full Studio panel uses.
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
    if entry.program.kind() == StudioKind::Parametric {
        let _ = numinous_app::studio_render::draw_parametric_rect(
            raster,
            numinous_app::studio_render::CurveRect {
                left: x0.max(0) as usize,
                top: y0.max(0) as usize,
                width: tile_width,
                height: tile_height,
            },
            xmin,
            xmax,
            |input| entry.program.point(input, a),
        );
        return;
    }
    let points: Vec<(usize, f64)> = (0..tile_width)
        .filter_map(|column| {
            let x = xmin + span * column as f64 / (tile_width as f64 - 1.0);
            let point = entry.program.point(x, a)?;
            point.1.is_finite().then_some((column, point.1))
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
        // A bundle whose capsule is not a regular file is skipped: the same
        // guard that refuses to follow a planted link out of the folder.
        let odd_bundle = dir.join("numinous-share-studio-43-bb");
        std::fs::create_dir_all(odd_bundle.join("creation.num")).expect("capsule as dir");

        let entries = discover(&dir).expect("a readable folder discovers");
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
    fn the_wall_discovers_and_draws_a_parametric_capsule_inside_its_tile() {
        let dir = scratch("parametric");
        let creation =
            StudioCreation::new_parametric("cos(3*t)", "sin(2*t)", 0.0, std::f64::consts::TAU, 0.0)
                .expect("pair");
        std::fs::write(dir.join("lissajous.num"), creation.to_num_file()).expect("write");

        let panel = GalleryPanel::open(&dir);
        assert_eq!(panel.len(), 1);
        assert_eq!(
            panel.selected_creation().expect("tile").kind(),
            numinous_core::StudioKind::Parametric
        );
        let mut wall = Raster::new(600, 400);
        panel.draw(&mut wall, 600, 400);
        assert!(wall.lit_count() > 100, "the pair has a gallery preview");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn saved_circles_and_ellipses_remain_distinct_on_the_composed_wall() {
        let dir = scratch("planar-proportions");
        for ratio in [1, 4] {
            let creation = StudioCreation::new_parametric(
                format!("{ratio}*cos(t)"),
                "sin(t)",
                0.0,
                std::f64::consts::TAU,
                1.0,
            )
            .expect("ellipse");
            std::fs::write(
                dir.join(format!("ellipse-{ratio}.num")),
                creation.to_num_file(),
            )
            .expect("save");
        }
        let panel = GalleryPanel::open(&dir);
        assert_eq!(panel.len(), 2);
        // These are the actual first-row curve bands after the selected
        // border, captions, title, lineage row and footer are composed.
        for (width, height, top, band_width, band_height) in
            [(360, 240, 38, 78, 156), (900, 700, 62, 213, 564)]
        {
            let mut wall = Raster::new(width, height);
            panel.draw(&mut wall, width, height);
            let rgba = wall.to_rgba();
            let blank = Raster::new(width, height).to_rgba();
            for (index, entry) in panel.entries.iter().enumerate() {
                let ratio = if entry.creation.source().starts_with('4') {
                    4
                } else {
                    1
                };
                let left = 6 + index * (width / 4);
                let pixels: Vec<_> = rgba
                    .chunks_exact(4)
                    .zip(blank.chunks_exact(4))
                    .enumerate()
                    .filter_map(|(at, (pixel, empty))| {
                        let (x, y) = (at % width, at / width);
                        (pixel != empty
                            && (left..left + band_width).contains(&x)
                            && (top..top + band_height).contains(&y))
                        .then_some((x, y))
                    })
                    .collect();
                assert!(pixels.len() > 50, "the stored path reaches its thumbnail");
                let dx = pixels.iter().map(|p| p.0).max().expect("ink")
                    - pixels.iter().map(|p| p.0).min().expect("ink");
                let dy = pixels.iter().map(|p| p.1).max().expect("ink")
                    - pixels.iter().map(|p| p.1).min().expect("ink");
                assert!(
                    dx.abs_diff(ratio * dy) <= ratio + 1,
                    "ratio {ratio} at {width}x{height}: thumbnail diameters {dx}x{dy}"
                );
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn discovery_is_newest_first_and_capped() {
        let dir = scratch("cap");
        for index in 0..MAX_GALLERY_ENTRIES + 3 {
            save(&dir, &format!("c{index:03}.num",), "sin(x)");
        }
        let entries = discover(&dir).expect("a readable folder discovers");
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
    fn the_wall_resolves_its_own_remix_tree() {
        use super::ParentStatus;
        let dir = scratch("tree");
        let parent = StudioCreation::new("sin(a*x)", -2.0, 2.0, 0.5)
            .expect("parent")
            .with_title("Parent Wave")
            .expect("title");
        std::fs::write(dir.join("parent.num"), parent.to_num_file()).expect("parent file");
        // Two forks of the parent, and one creation descending from a
        // parent that is not on this wall.
        for (name, a) in [("fork-one.num", 0.1), ("fork-two.num", 0.2)] {
            let fork = StudioCreation::new("sin(a*x)+0", -2.0, 2.0, a)
                .expect("fork")
                .with_descends(&parent.to_link())
                .expect("descends");
            std::fs::write(dir.join(name), fork.to_num_file()).expect("fork file");
        }
        let elsewhere = StudioCreation::new("cos(x)", -1.0, 1.0, 0.0).expect("far parent");
        let orphan = StudioCreation::new("cos(x)+0", -1.0, 1.0, 0.0)
            .expect("orphan")
            .with_descends(&elsewhere.to_link())
            .expect("descends");
        std::fs::write(dir.join("orphan.num"), orphan.to_num_file()).expect("orphan file");

        let mut panel = GalleryPanel::open(&dir);
        assert_eq!(panel.len(), 4);

        // Find each entry by walking the selection over the whole wall.
        let mut statuses = Vec::new();
        for index in 0..4 {
            panel.move_selection(-9, 0);
            panel.move_selection(-9, -9);
            panel.move_selection(index, 0);
            let source = panel
                .selected_creation()
                .expect("selection")
                .source()
                .to_string();
            statuses.push((source, panel.parent_status()));
        }
        let fork_status = statuses
            .iter()
            .find(|(source, _)| source == "sin(a*x)+0")
            .expect("a fork on the wall");
        assert!(
            matches!(fork_status.1, ParentStatus::Local(_)),
            "a fork resolves its local parent: {statuses:?}"
        );
        let orphan_status = statuses
            .iter()
            .find(|(source, _)| source == "cos(x)+0")
            .expect("the orphan on the wall");
        assert_eq!(
            orphan_status.1,
            ParentStatus::Absent,
            "an absent parent is not the same answer as no lineage"
        );
        let parent_status = statuses
            .iter()
            .find(|(source, _)| source == "sin(a*x)")
            .expect("the parent on the wall");
        assert_eq!(parent_status.1, ParentStatus::NoLineage);

        // D walks up: select a fork, step to the parent.
        for index in 0..4 {
            panel.move_selection(-9, -9);
            panel.move_selection(index, 0);
            if panel.selected_creation().expect("selection").source() == "sin(a*x)+0" {
                break;
            }
        }
        assert!(panel.select_parent(), "the cursor walks up the tree");
        assert_eq!(
            panel.selected_creation().expect("parent tile").title(),
            Some("Parent Wave")
        );
        assert!(
            !panel.select_parent(),
            "the root has no parent to walk to, and says so by refusing"
        );

        // The parent's tile carries the remix count for both forks, asserted
        // on the resolved entries rather than inferred from lit pixels.
        let parent_entry = panel
            .entries
            .iter()
            .find(|entry| entry.creation.source() == "sin(a*x)")
            .expect("the parent entry");
        assert_eq!(parent_entry.remixes, 2, "both forks credit the parent");
        for entry in &panel.entries {
            if entry.creation.source() == "sin(a*x)+0" {
                assert!(entry.parent.is_some(), "forks resolve their parent");
                assert_eq!(entry.remixes, 0);
            }
        }
        let mut wall = Raster::new(600, 400);
        panel.draw(&mut wall, 600, 400);
        assert!(wall.lit_count() > 100);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unedited_fork_of_an_untitled_parent_cannot_become_its_own_ancestor() {
        // An untitled creation's canonical link excludes lineage, so a fork
        // shared without edits carries its parent's exact link as its own.
        // Matching must skip the entry itself or the fork self-parents, the
        // remix credits the wrong tile, and D steps in place.
        let dir = scratch("selfparent");
        let parent = StudioCreation::new("sin(a*x)", -2.0, 2.0, 0.5).expect("parent");
        std::fs::write(dir.join("parent.num"), parent.to_num_file()).expect("parent file");
        let twin_fork = StudioCreation::new("sin(a*x)", -2.0, 2.0, 0.5)
            .expect("twin")
            .with_descends(&parent.to_link())
            .expect("descends");
        std::fs::write(dir.join("twin.num"), twin_fork.to_num_file()).expect("twin file");

        let panel = GalleryPanel::open(&dir);
        assert_eq!(panel.len(), 2);
        for (index, entry) in panel.entries.iter().enumerate() {
            assert_ne!(
                entry.parent,
                Some(index),
                "no entry may resolve itself as its own parent"
            );
        }
        let fork_entry = panel
            .entries
            .iter()
            .find(|entry| entry.creation.descends().is_some())
            .expect("the fork");
        assert!(
            fork_entry.parent.is_some(),
            "the fork resolves the other entry, not itself and not nothing"
        );
        assert_eq!(fork_entry.remixes, 0, "the remix credits the parent tile");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn two_twin_forks_do_not_adopt_each_other() {
        // Two unedited forks of the same untitled creation carry identical
        // links and identical descents. Matching on the link alone let each
        // adopt the other: both wore a remix badge, the real parent went
        // uncredited, and D walked in a circle between them forever. A
        // sibling stands beside, not above.
        let dir = scratch("twins");
        let parent = StudioCreation::new("sin(a*x)", -2.0, 2.0, 0.5).expect("parent");
        std::fs::write(dir.join("parent.num"), parent.to_num_file()).expect("parent file");
        for name in ["twin-a.num", "twin-b.num"] {
            let fork = StudioCreation::new("sin(a*x)", -2.0, 2.0, 0.5)
                .expect("twin")
                .with_descends(&parent.to_link())
                .expect("descends");
            std::fs::write(dir.join(name), fork.to_num_file()).expect("twin file");
        }

        let panel = GalleryPanel::open(&dir);
        assert_eq!(panel.len(), 3);
        let parent_index = panel
            .entries
            .iter()
            .position(|entry| entry.creation.descends().is_none())
            .expect("the parent is on the wall");

        for (index, entry) in panel.entries.iter().enumerate() {
            if entry.creation.descends().is_none() {
                continue;
            }
            assert_eq!(
                entry.parent,
                Some(parent_index),
                "a fork must credit the parent, never its sibling"
            );
            assert_eq!(entry.remixes, 0, "a sibling is not a remix of a sibling");
            // No cycle: walking up from either fork ends at the parent.
            let mut walker = index;
            for _ in 0..4 {
                match panel.entries[walker].parent {
                    Some(next) => walker = next,
                    None => break,
                }
            }
            assert_eq!(walker, parent_index, "the walk must end, at the parent");
        }
        assert_eq!(
            panel.entries[parent_index].remixes, 2,
            "both forks credit the one creation they came from"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_horizontal_move_parks_at_the_row_edge_instead_of_wrapping() {
        let dir = scratch("park");
        // Distinct knobs so two adjacent tiles can never hold equal
        // creations: if the cursor wrapped, the selection would change.
        for index in 0..8 {
            let creation =
                StudioCreation::new("sin(a*x)", -2.0, 2.0, 0.1 * index as f64).expect("creation");
            std::fs::write(dir.join(format!("c{index}.num")), creation.to_num_file())
                .expect("write");
        }
        let mut panel = GalleryPanel::open(&dir);
        assert_eq!(panel.len(), 8);
        // Park at the right edge of the top row, then push further right.
        panel.move_selection(9, 0);
        let edge = panel.selected_creation().expect("edge tile").clone();
        panel.move_selection(1, 0);
        assert_eq!(
            panel.selected_creation().expect("still the edge tile"),
            &edge,
            "a horizontal move at the row edge parks instead of snaking into \
             the next row"
        );
        // And the left edge of the second row does not snake back up.
        panel.move_selection(0, 1);
        panel.move_selection(-9, 0);
        let left = panel.selected_creation().expect("row start").clone();
        panel.move_selection(-1, 0);
        assert_eq!(panel.selected_creation().expect("parked"), &left);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unreadable_folder_is_a_fact_not_an_empty_wall() {
        // A file where the folder should be: read_dir refuses.
        let blocked = std::env::temp_dir().join(format!(
            "numinous-gallery-unreadable-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&blocked);
        let _ = std::fs::remove_file(&blocked);
        std::fs::write(&blocked, "a file where a folder must go").expect("blocker");

        assert!(
            discover(&blocked).is_none(),
            "an unreadable folder is not an empty one"
        );
        let panel = GalleryPanel::open(&blocked);
        let mut wall = Raster::new(600, 400);
        panel.draw(&mut wall, 600, 400);
        assert!(
            wall.lit_count() > 50,
            "the unreadable wall says so instead of claiming NOTHING SAVED"
        );
        let _ = std::fs::remove_file(&blocked);
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
        let mut panel = GalleryPanel::open(&dir);
        for height in 40..=220 {
            let mut wall = Raster::new(600, height);
            panel.draw(&mut wall, 600, height);
        }

        // Too short for thumbnails must not mean a blank wall with a live,
        // invisible cursor: the named selection makes choosing sighted, so
        // moving the cursor visibly changes the frame.
        let mut before = Raster::new(600, 150);
        panel.draw(&mut before, 600, 150);
        assert!(before.lit_count() > 50, "the short wall still speaks");
        panel.move_selection(1, 0);
        let mut after = Raster::new(600, 150);
        panel.draw(&mut after, 600, 150);
        assert_ne!(
            before.to_rgba(),
            after.to_rgba(),
            "the selection stays visible when tiles cannot draw"
        );
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
