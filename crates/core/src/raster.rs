//! An RGBA pixel raster: the image (PNG) surface.
//!
//! Rooms draw into a [`Raster`] through the same [`Surface`] trait they use for
//! ASCII, so one `render` method produces both the terminal view and a real
//! image. Rendering is on the CPU, deterministic, and needs no GPU, so it is
//! fully testable. Marks are drawn additively on a near-black stage, so
//! overlapping strokes glow (see `docs/VISUALS.md`).

use crate::surface::{MAX_DIM, Surface};

/// The near-black background (the Numinous stage).
const BACKGROUND: [u8; 3] = crate::palette::STAGE;

/// The accent used when a room does not specify one.
const DEFAULT_ACCENT: [u8; 3] = [36, 120, 180];

/// Scale a color by `factor`, clamping each channel to 255.
fn scale(color: [u8; 3], factor: f32) -> [u8; 3] {
    let ch = |c: u8| (f32::from(c) * factor).round().clamp(0.0, 255.0) as u8;
    [ch(color[0]), ch(color[1]), ch(color[2])]
}

/// What a mark means on a pixel surface, which decides how it is lit.
///
/// Rooms speak only in marks. The terminal shows each mark as itself, and a
/// reader of `' . : + * #'` sees an ordered ramp of weight without being
/// told; a pixel surface has to light each mark so that ramp survives. This
/// is that table, written once, and the vocabulary `docs/VISUALS.md` names.
///
/// | Mark | Role | Pixel ink |
/// | --- | --- | --- |
/// | `' '` | [`MarkRole::Empty`] | nothing |
/// | `'.'` `':'` | [`MarkRole::Faint`] | the accent at 0.35 |
/// | `'-'` | [`MarkRole::Structure`] | fixed near-black blue |
/// | `'+'` `'x'` `'='` | [`MarkRole::Secondary`] | the accent at 0.6 |
/// | `'*'` `'o'` and any other | [`MarkRole::Idea`] | the accent |
/// | `'#'` | [`MarkRole::Hot`] | the accent at 1.7 |
/// | `'!'` | [`MarkRole::Warning`] | fixed red, always with a shape cue |
/// | `'@'` `'%'` `'&'` `'~'` | the four spectral inks | fixed magenta, green, amber, violet |
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MarkRole {
    /// `' '`: nothing is drawn. A room that fills its field with spaces
    /// leaves the stage showing, as the terminal does.
    Empty,
    /// `'.'` and `':'`: guides, unlit field, older history.
    Faint,
    /// `'-'`: chrome structure such as rules, meters and axes.
    Structure,
    /// `'+'`, `'x'` and `'='`: secondary detail beside the idea.
    Secondary,
    /// `'*'`, `'o'` and every mark not named elsewhere: the idea itself.
    Idea,
    /// `'#'`: the hot core, emphasis, the brightest level.
    Hot,
    /// `'!'`: something is wrong here. Never alone; a shape says it too.
    Warning,
    /// `'@'`: the magenta spectral ink.
    Magenta,
    /// `'%'`: the green spectral ink.
    Green,
    /// `'&'`: the amber spectral ink.
    Amber,
    /// `'~'`: the violet spectral ink.
    Violet,
}

impl MarkRole {
    /// Every role, in ramp order then the fixed inks.
    pub const ALL: [MarkRole; 11] = [
        MarkRole::Empty,
        MarkRole::Faint,
        MarkRole::Structure,
        MarkRole::Secondary,
        MarkRole::Idea,
        MarkRole::Hot,
        MarkRole::Warning,
        MarkRole::Magenta,
        MarkRole::Green,
        MarkRole::Amber,
        MarkRole::Violet,
    ];

    /// The role a mark plays.
    #[must_use]
    pub fn of(mark: char) -> MarkRole {
        match mark {
            ' ' => MarkRole::Empty,
            '.' | ':' => MarkRole::Faint,
            '-' => MarkRole::Structure,
            '+' | 'x' | '=' => MarkRole::Secondary,
            '#' => MarkRole::Hot,
            '!' => MarkRole::Warning,
            '@' => MarkRole::Magenta,
            '%' => MarkRole::Green,
            '&' => MarkRole::Amber,
            '~' => MarkRole::Violet,
            _ => MarkRole::Idea,
        }
    }

    /// The one mark that stands for this role wherever a record names it, so
    /// two marks that light alike are written down alike.
    #[must_use]
    pub fn mark(self) -> char {
        match self {
            MarkRole::Empty => ' ',
            MarkRole::Faint => '.',
            MarkRole::Structure => '-',
            MarkRole::Secondary => '+',
            MarkRole::Idea => '*',
            MarkRole::Hot => '#',
            MarkRole::Warning => '!',
            MarkRole::Magenta => '@',
            MarkRole::Green => '%',
            MarkRole::Amber => '&',
            MarkRole::Violet => '~',
        }
    }

    /// Whether this role is a level of the room's own accent: faint,
    /// secondary, the idea, or hot. The other roles are fixed inks.
    #[must_use]
    pub fn is_accent_level(self) -> bool {
        matches!(
            self,
            MarkRole::Faint | MarkRole::Secondary | MarkRole::Idea | MarkRole::Hot
        )
    }

    /// The color this role adds for a raster drawing in `accent`.
    #[must_use]
    pub fn ink(self, accent: [u8; 3]) -> [u8; 3] {
        match self {
            MarkRole::Empty => [0, 0, 0],
            MarkRole::Faint => scale(accent, 0.35),
            MarkRole::Structure => [16, 20, 34],
            MarkRole::Secondary => scale(accent, 0.6),
            MarkRole::Idea => accent,
            MarkRole::Hot => scale(accent, 1.7),
            MarkRole::Warning => [230, 72, 72],
            MarkRole::Magenta => [216, 40, 190],
            MarkRole::Green => [56, 224, 132],
            MarkRole::Amber => [242, 148, 36],
            MarkRole::Violet => [116, 72, 232],
        }
    }
}

/// A fixed-size RGB pixel buffer that rooms draw into, in a room's accent color.
#[derive(Debug, Clone)]
pub struct Raster {
    width: usize,
    height: usize,
    accent: [u8; 3],
    pixels: Vec<[u8; 3]>,
}

impl Raster {
    /// Create a raster filled with the background color, using the default accent.
    ///
    /// Each dimension is clamped to a safe maximum so any request is safe.
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        Self::with_accent(width, height, DEFAULT_ACCENT)
    }

    /// Create a raster that draws in the given accent color.
    #[must_use]
    pub fn with_accent(width: usize, height: usize, accent: [u8; 3]) -> Self {
        let width = width.min(MAX_DIM);
        let height = height.min(MAX_DIM);
        Self {
            width,
            height,
            accent,
            pixels: vec![BACKGROUND; width * height],
        }
    }

    /// Import a tightly packed RGBA frame while retaining an accent for later
    /// interface drawing. Alpha is ignored because Numinous frames are opaque.
    ///
    /// Returns `None` when dimensions exceed the shared surface bound, their
    /// byte size overflows, or the slice length does not exactly match.
    #[must_use]
    pub fn from_rgba(width: usize, height: usize, accent: [u8; 3], rgba: &[u8]) -> Option<Self> {
        if width > MAX_DIM || height > MAX_DIM {
            return None;
        }
        let expected = width.checked_mul(height)?.checked_mul(4)?;
        if rgba.len() != expected {
            return None;
        }
        let pixels = rgba
            .chunks_exact(4)
            .map(|pixel| [pixel[0], pixel[1], pixel[2]])
            .collect();
        Some(Self {
            width,
            height,
            accent,
            pixels,
        })
    }

    /// The color added for a mark, by the role it plays (see [`MarkRole`]):
    /// the accent ramp, the semantic interface colors, and four spectral
    /// inks that rooms can combine additively for prismatic light.
    ///
    /// Public so a surface can ask what its own marks will look like. The App
    /// draws its chrome and games with the same marks rooms use, against its
    /// own accents, and the accessibility sweeps that measure whether two marks
    /// stay apart for a color-blind player need the answer for both faces. The
    /// alternative was a second copy of this table in the App, which is the
    /// kind of second copy that drifts.
    #[must_use]
    pub fn ink(&self, mark: char) -> [u8; 3] {
        MarkRole::of(mark).ink(self.accent)
    }

    /// The pixels as a tightly packed RGBA byte buffer (`width * height * 4`),
    /// suitable for PNG encoding. Alpha is always fully opaque.
    #[must_use]
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.pixels.len() * 4);
        for p in &self.pixels {
            out.extend_from_slice(&[p[0], p[1], p[2], 255]);
        }
        out
    }

    /// The number of pixels brighter than the background. Useful for tests.
    #[must_use]
    pub fn lit_count(&self) -> usize {
        self.pixels.iter().filter(|&&p| p != BACKGROUND).count()
    }

    /// Fill a rectangle with the accent scaled by `level` in `0..=1`.
    ///
    /// This is for pictures whose marks are a ramp of brightness rather than
    /// the semantic ink table. Field plates use it so `.` and `#` stay
    /// distinct instead of collapsing through [`Self::ink`].
    pub fn shade_rect(&mut self, x: i32, y: i32, width: i32, height: i32, level: f32) {
        if width <= 0 || height <= 0 {
            return;
        }
        let color = scale(self.accent, 0.25 + 1.45 * level.clamp(0.0, 1.0));
        let x0 = x.max(0) as usize;
        let y0 = y.max(0) as usize;
        let x1 = x.saturating_add(width).max(0) as usize;
        let y1 = y.saturating_add(height).max(0) as usize;
        let x1 = x1.min(self.width);
        let y1 = y1.min(self.height);
        for row in y0..y1 {
            let start = row * self.width;
            for col in x0..x1 {
                self.pixels[start + col] = color;
            }
        }
    }

    /// Dim every pixel to `keep` percent of its brightness, a backdrop for
    /// overlay text so menus stay legible over busy rooms.
    pub fn dim(&mut self, keep: u32) {
        let keep = keep.min(100);
        for pixel in &mut self.pixels {
            for channel in pixel.iter_mut() {
                *channel = ((u32::from(*channel) * keep) / 100) as u8;
            }
        }
    }

    /// Dim only the rows from `y0` to `y1` (clamped): a legibility band
    /// behind HUD text, so words stay readable over bright rooms.
    pub fn dim_rows(&mut self, y0: i32, y1: i32, keep: u32) {
        let keep = keep.min(100);
        let from = y0.max(0) as usize;
        let to = (y1.max(0) as usize).min(self.height);
        for y in from..to {
            for x in 0..self.width {
                let pixel = &mut self.pixels[y * self.width + x];
                for channel in pixel.iter_mut() {
                    *channel = ((u32::from(*channel) * keep) / 100) as u8;
                }
            }
        }
    }

    /// Reset a horizontal band to the stage background.
    ///
    /// This gives dense interface copy a quiet surface instead of asking it to
    /// compete with a bright room. Bounds are clamped to the raster.
    pub fn clear_rows(&mut self, y0: i32, y1: i32) {
        let from = y0.max(0) as usize;
        let to = (y1.max(0) as usize).min(self.height);
        for y in from..to {
            for x in 0..self.width {
                self.pixels[y * self.width + x] = BACKGROUND;
            }
        }
    }

    /// Replace this raster's pixels from an RGBA byte buffer (alpha ignored;
    /// extra or missing bytes are tolerated). Brings a post-processed frame,
    /// for example a visual era, back onto a raster.
    pub fn set_rgba(&mut self, rgba: &[u8]) {
        for (pixel, bytes) in self.pixels.iter_mut().zip(rgba.chunks_exact(4)) {
            *pixel = [bytes[0], bytes[1], bytes[2]];
        }
    }

    /// A `width` x `height` copy of this raster where each source pixel
    /// covers a `factor` x `factor` block (nearest neighbor, never blended).
    ///
    /// The live app view renders heavy rooms below window resolution and
    /// expands them with this before the HUD draws, so interface text stays
    /// window-crisp while only room pixels trade sharpness for motion. The
    /// accent carries over so chrome drawn on the result matches chrome drawn
    /// on a full-resolution render.
    ///
    /// The output is exactly the requested size, whatever the source size:
    /// blocks at the right and bottom edges are partial when the dimensions
    /// are not factor multiples, output beyond the scaled source repeats the
    /// nearest edge pixel, and output smaller than the scaled source is a
    /// top-left crop. Requested dimensions are clamped to the same safe
    /// maximum as every raster; a zero-size source stays background.
    #[must_use]
    pub fn upscaled(&self, factor: usize, width: usize, height: usize) -> Raster {
        let factor = factor.max(1);
        let mut out = Raster::with_accent(width, height, self.accent);
        if self.width == 0 || self.height == 0 {
            return out;
        }
        for y in 0..out.height {
            let sy = (y / factor).min(self.height - 1);
            let row = y * out.width;
            let same_source_row = y > 0 && sy == ((y - 1) / factor).min(self.height - 1);
            if same_source_row {
                out.pixels.copy_within(row - out.width..row, row);
                continue;
            }
            for x in 0..out.width {
                let sx = (x / factor).min(self.width - 1);
                out.pixels[row + x] = self.pixels[sy * self.width + sx];
            }
        }
        out
    }

    /// Copy another raster's pixels into this one with its top-left at `(x, y)`,
    /// clipping anything that falls outside. Used to tile rooms into a sheet.
    pub fn blit(&mut self, other: &Raster, x: usize, y: usize) {
        for oy in 0..other.height {
            let ty = y + oy;
            if ty >= self.height {
                break;
            }
            for ox in 0..other.width {
                let tx = x + ox;
                if tx >= self.width {
                    break;
                }
                self.pixels[ty * self.width + tx] = other.pixels[oy * other.width + ox];
            }
        }
    }
}

impl Surface for Raster {
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn paint(&mut self, x: i32, y: i32, _mark: char, color: [u8; 3]) {
        if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
            self.pixels[y as usize * self.width + x as usize] = color;
        }
    }

    fn plot(&mut self, x: i32, y: i32, mark: char) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        if x < self.width && y < self.height {
            let add = self.ink(mark);
            let pixel = &mut self.pixels[y * self.width + x];
            for i in 0..3 {
                pixel[i] = pixel[i].saturating_add(add[i]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BACKGROUND, MarkRole, Raster};
    use crate::surface::Surface;

    #[test]
    fn new_raster_is_background() {
        let r = Raster::new(4, 4);
        assert_eq!(r.width(), 4);
        assert_eq!(r.lit_count(), 0);
    }

    #[test]
    fn sampled_colors_replace_pixels_and_clip_without_accumulation() {
        let mut raster = Raster::new(2, 2);
        raster.paint(0, 0, '*', [33, 66, 99]);
        raster.paint(0, 0, '*', [33, 66, 99]);
        for (x, y) in [(-1, 0), (0, -1), (2, 0), (0, 2), (i32::MAX, i32::MAX)] {
            raster.paint(x, y, '#', [255, 255, 255]);
        }
        assert_eq!(&raster.to_rgba()[..4], &[33, 66, 99, 255]);
        assert_eq!(raster.lit_count(), 1);
    }

    #[test]
    fn plot_brightens_a_pixel_additively() {
        let mut r = Raster::new(4, 4);
        r.plot(1, 1, '*');
        assert_eq!(r.lit_count(), 1);
        r.plot(1, 1, '*'); // additive: brighter, still one lit pixel
        assert_eq!(r.lit_count(), 1);
    }

    #[test]
    fn semantic_warning_ink_is_distinct_from_structure_and_accent() {
        let raster = Raster::with_accent(4, 4, [40, 210, 90]);

        assert_eq!(raster.ink('!'), [230, 72, 72]);
        assert_ne!(raster.ink('!'), raster.ink('.'));
        assert_ne!(raster.ink('!'), raster.ink('-'));
        assert_ne!(raster.ink('!'), raster.ink('#'));
    }

    /// The block character `to_mono` gives a cell filled with one flat color.
    ///
    /// Asked of the renderer rather than worked out from its thresholds, so
    /// this cannot drift away from what a `NO_COLOR` player is actually shown.
    fn mono_glyph(color: [u8; 3]) -> char {
        let mut raster = Raster::new(1, 2);
        raster.set_rgba(&[
            color[0], color[1], color[2], 255, color[0], color[1], color[2], 255,
        ]);
        crate::ansi::to_mono(&raster)
            .chars()
            .next()
            .expect("to_mono draws at least one cell")
    }

    /// Every room whose source draws with a given mark, as `(id, accent)`.
    ///
    /// Read from the room sources rather than kept as a list here. A list would
    /// be a second copy of something the code already knows, and second copies
    /// drift: a room could start using the warning ink and this check would go
    /// on testing the four rooms that used it when the list was written.
    /// A source that draws with the mark but yields no room id is a failure of
    /// this scan, not a file to pass over. Skipping it quietly would be the
    /// exact shape of hole the scan exists to close: a room could start using
    /// the ink, go unparsed, and the catalog would still look clean.
    fn rooms_drawing_with(mark: char) -> Vec<(String, [u8; 3])> {
        rooms_drawing_with_all(&[mark])
    }

    /// Sources that draw on another room's raster, and the room they draw on.
    ///
    /// These are the engineered-aha overlays. They declare no id of their own
    /// because they are not rooms; they paint over one, in that room's accent,
    /// so that is the accent their marks have to be legible against.
    ///
    /// Written down rather than guessed from the file name, and checked below:
    /// an entry naming a room that does not exist fails, and a new helper that
    /// reaches for a mark fails the scan until it is listed here.
    const HELPER_ROOMS: [(&str, &str); 7] = [
        ("buffon_aha", "buffon-needle"),
        ("galton_aha", "galton-board"),
        ("kepler_aha", "kepler-laws"),
        ("nontransitive_aha", "nontransitive"),
        ("parrondo_aha", "parrondo"),
        ("pendulum_aha", "double-pendulum"),
        ("times_tables_aha", "times-tables"),
    ];

    fn helper_parent(stem: &str) -> Option<&'static str> {
        HELPER_ROOMS
            .iter()
            .find(|(helper, _)| *helper == stem)
            .map(|(_, room)| *room)
    }

    fn source_room_id(stem: &str) -> Option<&'static str> {
        crate::rooms::ROOM_SOURCE_IDS
            .iter()
            .find(|(module, _)| *module == stem)
            .map(|(_, id)| *id)
            .or_else(|| helper_parent(stem))
    }

    #[test]
    fn every_helper_names_a_room_that_exists() {
        for (helper, room) in HELPER_ROOMS {
            assert!(
                crate::registry::room_by_id(room).is_some(),
                "{helper} is recorded as drawing on {room}, which is not a room"
            );
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/rooms")
                .join(format!("{helper}.rs"));
            assert!(
                path.is_file(),
                "{helper} is listed but {path:?} does not exist"
            );
        }
    }

    /// Every room, with its accent and the marks its source draws.
    ///
    /// The pair-wise scan below answers "which rooms draw both of these". This
    /// answers "what does each room draw", which is what an audit needs: a
    /// record built from pair queries could only list the pairs somebody
    /// thought to ask for, and would call the catalog covered on that basis.
    fn room_palettes() -> Vec<(String, [u8; 3], Vec<char>)> {
        // Every mark the ink table gives a role, except the space, which
        // draws nothing and so cannot be told apart from anything.
        const CANDIDATES: [char; 14] = [
            '.', ':', '-', '+', 'x', '=', '*', 'o', '#', '!', '@', '%', '&', '~',
        ];
        let mut pending = vec![std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/rooms")];
        let mut found: Vec<(String, [u8; 3], Vec<char>)> = Vec::new();
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(&dir).expect("the rooms directory is readable") {
                let path = entry.expect("a readable directory entry").path();
                if path.is_dir() {
                    pending.push(path);
                    continue;
                }
                if path.extension().is_none_or(|extension| extension != "rs") {
                    continue;
                }
                let source = std::fs::read_to_string(&path).expect("a readable room source");
                let marks: Vec<char> = CANDIDATES
                    .into_iter()
                    .filter(|mark| source.contains(&format!("'{mark}'")))
                    .collect();
                if marks.is_empty() {
                    continue;
                }
                let stem = path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .expect("a room source has a readable name");
                let Some(id) = source_room_id(stem) else {
                    continue;
                };
                let Some(metadata) = crate::rooms::room_meta_by_id(id) else {
                    continue;
                };
                // A helper draws on its parent's raster, so its marks belong to
                // the parent rather than to a room of its own.
                if let Some(slot) = found.iter_mut().find(|(seen, _, _)| seen == id) {
                    for mark in marks {
                        if !slot.2.contains(&mark) {
                            slot.2.push(mark);
                        }
                    }
                } else {
                    found.push((id.to_string(), metadata.accent, marks));
                }
            }
        }
        for (_, _, marks) in &mut found {
            marks.sort_unstable();
        }
        found.sort();
        found
    }

    /// The levels of its own accent a room draws, one mark for each: faint,
    /// secondary, the idea, hot. Two marks with one role are one light, so
    /// listing both would record a distinction the room never made.
    fn accent_levels(marks: &[char]) -> Vec<char> {
        let mut levels: Vec<char> = marks
            .iter()
            .map(|mark| MarkRole::of(*mark))
            .filter(|role| role.is_accent_level())
            .map(MarkRole::mark)
            .collect();
        levels.sort_unstable();
        levels.dedup();
        levels
    }

    /// The name a shrink-only list records a mark under.
    ///
    /// The lists were written when every ordinary mark painted the plain
    /// accent, so their `'*'` names any ordinary level of the accent: faint,
    /// secondary, or the idea. `'#'` names the hot level, and the fixed inks
    /// name themselves. Keeping that grain means splitting one light into
    /// three cannot make a list grow, while a fold at any ordinary level still
    /// holds its room's entry until every level is clear.
    fn list_name(mark: char) -> char {
        match MarkRole::of(mark) {
            MarkRole::Faint | MarkRole::Secondary | MarkRole::Idea => '*',
            role => role.mark(),
        }
    }

    /// The accent levels each named room draws, read once from the sources.
    fn accent_levels_by_room() -> Vec<(String, Vec<char>)> {
        room_palettes()
            .into_iter()
            .map(|(id, _, marks)| (id, accent_levels(&marks)))
            .collect()
    }

    fn levels_of<'a>(levels: &'a [(String, Vec<char>)], id: &str) -> &'a [char] {
        levels
            .iter()
            .find(|(room, _)| room == id)
            .map_or(&[], |(_, marks)| marks.as_slice())
    }

    /// Every room whose source draws with all of `marks`, as `(id, accent)`.
    ///
    /// Same scan, same refusal to pass over what it cannot read. Asking for
    /// several marks at once answers the question that matters for a ramp:
    /// which rooms draw two levels that a player has to tell apart.
    fn rooms_drawing_with_all(marks: &[char]) -> Vec<(String, [u8; 3])> {
        let literals: Vec<String> = marks.iter().map(|mark| format!("'{mark}'")).collect();
        let literal = literals.join(" and ");
        let mut pending = vec![std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/rooms")];
        let mut found = Vec::new();
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(&dir).expect("the rooms directory is readable") {
                let path = entry.expect("a readable directory entry").path();
                // Rooms with several files keep them in a subdirectory. Walking
                // only the top level would leave those permanently unchecked.
                if path.is_dir() {
                    pending.push(path);
                    continue;
                }
                if path.extension().is_none_or(|extension| extension != "rs") {
                    continue;
                }
                let source = std::fs::read_to_string(&path).expect("a readable room source");
                if !literals.iter().all(|wanted| source.contains(wanted)) {
                    continue;
                }
                let stem = path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .expect("a room source has a readable name");
                let id = source_room_id(stem).unwrap_or_else(|| {
                    panic!(
                        "{path:?} draws with {literal} and has no catalog source mapping, so \
                             this scan cannot tell which accent it is drawn against. If it draws \
                             on behalf of another room, add it to HELPER_ROOMS naming that room; \
                             if it draws nowhere, it should not be reaching for a mark."
                    )
                });
                let metadata = crate::rooms::room_meta_by_id(id)
                    .unwrap_or_else(|| panic!("{id} is mapped from {path:?} but not registered"));
                found.push((id.to_string(), metadata.accent));
            }
        }
        found.sort();
        found
    }

    #[test]
    fn the_warning_ink_survives_the_color_free_renderer_in_every_room_that_uses_it() {
        // `'!'` is the one ink that carries meaning rather than beauty: it says
        // this cell is wrong. The test above proves it is a different color
        // from the accent, which is not the same as being a different thing to
        // look at. Accents vary per room, and 81 of the 354 sit within 12
        // luminance of the warning ink, so "distinct color" can quite easily
        // mean "identical grey".
        //
        // What has to hold is that a player who cannot use color still sees the
        // warning, and the renderer that player is given is `to_mono`. So the
        // question is put to `to_mono` directly: does the warning cell come out
        // as a different block character than every level of the accent the
        // room draws beside it?
        let rooms = rooms_drawing_with('!');
        let levels = accent_levels_by_room();
        assert!(
            rooms.len() >= 4,
            "only {} rooms found drawing with the warning ink, so the scan is broken \
             rather than the catalog being clean",
            rooms.len()
        );

        let warning = mono_glyph(Raster::new(1, 1).ink('!'));
        let mut compared = 0;
        for (id, accent) in rooms {
            let raster = Raster::with_accent(1, 1, accent);
            for &ordinary in levels_of(&levels, &id) {
                compared += 1;
                let against = mono_glyph(raster.ink(ordinary));
                assert_ne!(
                    warning, against,
                    "in {id} a warning cell and an ordinary {ordinary:?} cell are the same \
                     character without color, so the warning is carried by hue alone"
                );
            }
        }
        assert!(
            compared >= 4,
            "only {compared} levels compared against the warning"
        );
    }

    /// Rooms where a mark that carries meaning is told apart from the room's
    /// ordinary ink by hue alone, so a color-blind player loses it.
    ///
    /// A different question from the two lists around it, and the reason it
    /// needs its own. `to_mono` asks what a player with no color at all sees,
    /// which is the right question for a terminal and the wrong one for the
    /// App: there the picture is pixels, the player has color, and they simply
    /// have fewer distinctions than the palette assumes. Roughly one man in
    /// twelve is in that position.
    ///
    /// Each entry is measured by `crate::dichromacy`, and both halves of its
    /// rule have to hold: ordinary vision separates the pair comfortably and at
    /// least one dichromacy folds it together. A pair that is close for
    /// everyone is a contrast defect, which the color-free sweeps catch, not
    /// this list.
    ///
    /// Shrink-only, like its neighbours: the test below fails if the list grows,
    /// and fails if an entry stops colliding and is not removed. Fixing an entry
    /// means changing an ink or an accent, which changes what the room looks
    /// like to everyone, so it is a decision about the product rather than a
    /// defect to patch. Tracked in `docs/ROADMAP.md` under 0.5 Sensory.
    /// Each entry is the room and the ordinary mark the warning is lost
    /// against, so a later fix that separates it from one level and not the
    /// other cannot be mistaken for a clean room. Measured against every level
    /// of its own accent the room draws beside the warning.
    const MEANING_LOST_TO_COLOR_BLINDNESS: [(&str, char); 2] =
        [("cult-of-pi", '*'), ("laplace-clock", '*')];

    #[test]
    fn a_mark_that_means_something_stays_apart_for_a_color_blind_player() {
        use crate::dichromacy;

        // The warning ink is the one mark that carries meaning rather than
        // beauty: it says this cell is wrong. The test above proves it survives
        // the color-free renderer. That is not the same as surviving a player
        // who has color and fewer distinctions, and this is the second half.
        let rooms = rooms_drawing_with('!');
        assert!(
            rooms.len() >= 4,
            "only {} rooms found drawing with the warning ink, so the scan is broken \
             rather than the catalog being clean",
            rooms.len()
        );

        // Against every accent level the room draws, the same levels the
        // color-free check uses. A warning that stays clear of one level can
        // still be lost against another, so checking only one would leave the
        // rest of the question unasked.
        let levels = accent_levels_by_room();
        let warning = Raster::new(1, 1).ink('!');
        let mut lost = Vec::new();
        for (id, accent) in rooms {
            let raster = Raster::with_accent(1, 1, accent);
            for &ordinary in levels_of(&levels, &id) {
                let entry = (id.clone(), list_name(ordinary));
                if dichromacy::color_alone(warning, raster.ink(ordinary)) && !lost.contains(&entry)
                {
                    lost.push(entry);
                }
            }
        }
        lost.sort();

        let known: Vec<(String, char)> = MEANING_LOST_TO_COLOR_BLINDNESS
            .iter()
            .map(|(id, mark)| ((*id).to_string(), *mark))
            .collect();
        let newly: Vec<&(String, char)> = lost.iter().filter(|it| !known.contains(it)).collect();
        assert!(
            newly.is_empty(),
            "these rooms newly hide a meaning-carrying mark from a color-blind player \
             and must be fixed or tracked: {newly:?}"
        );
        let fixed: Vec<&(String, char)> = known.iter().filter(|it| !lost.contains(it)).collect();
        assert!(
            fixed.is_empty(),
            "these no longer collide and must leave MEANING_LOST_TO_COLOR_BLINDNESS: {fixed:?}"
        );
    }

    /// The four spectral inks, which rooms combine additively for prismatic
    /// light. Fixed colors rather than accent-derived, so a collision here is a
    /// property of the palette meeting one room's accent.
    const SPECTRAL_INKS: [char; 4] = ['@', '%', '&', '~'];

    /// Every mark that draws a level of the room's own accent. A pair is
    /// recorded under [`list_name`], because a room drawing `'@'` beside
    /// `'x'` and a room drawing it beside `'.'` have one defect in one list
    /// entry, and counting them separately would report it twice.
    const ACCENT_LEVEL_MARKS: [char; 8] = ['.', ':', '+', 'x', '=', '*', 'o', '#'];

    /// Every spectral pair a color-blind player cannot separate, as
    /// `(room, first mark, second mark)` with the pair in character order.
    ///
    /// The third list of its kind and the one the other two do not reach.
    /// [`LEVELS_FOLD_FOR_A_DICHROMAT`] is about the accent's own levels;
    /// [`MEANING_LOST_TO_COLOR_BLINDNESS`] is about the warning ink. Neither
    /// looks at the spectral inks, and those are where the largest collapse in
    /// the catalog turns out to be: `times-tables` separates `'@'` from its
    /// accent by 95 for ordinary vision and by under 1 for a deuteranope.
    ///
    /// **What this list does not say is which entries matter.** The measurement
    /// is mechanical; whether a room is saying something with an ink is not. Two
    /// were read to check the question is real, and they answer it opposite
    /// ways. In `bayes-update` the inks separate the prior, the likelihood and
    /// the posterior, so a reader who cannot tell two of them apart cannot read
    /// the picture. In `times-tables` the ink is chosen by where a chord starts
    /// around the circle, so it bands the drawing and carries nothing. The rest
    /// have not been read, and guessing would be worse than saying so.
    ///
    /// Shrink-only, like its neighbours. Tracked in `docs/ROADMAP.md` under 0.5
    /// Sensory.
    const SPECTRAL_INKS_COLLAPSE_FOR_A_DICHROMAT: [(&str, char, char); 16] = [
        ("bayes-update", '#', '@'),
        ("buffon-needle", '#', '@'),
        ("buffon-needle", '*', '@'),
        ("circle-map", '#', '@'),
        ("circle-map", '*', '@'),
        ("function-painter", '*', '@'),
        ("josephus", '#', '@'),
        ("josephus", '*', '@'),
        ("message-heals", '*', '~'),
        ("murmuration", '#', '@'),
        ("murmuration", '*', '@'),
        ("newton", '*', '@'),
        ("riemann-sphere", '*', '@'),
        ("times-tables", '#', '%'),
        ("times-tables", '&', '@'),
        ("times-tables", '*', '@'),
    ];

    #[test]
    fn the_spectral_inks_stay_apart_for_a_color_blind_player_outside_the_known_list() {
        use crate::dichromacy;

        // Every pair a spectral ink can form: with another spectral ink and
        // with every level of the accent. Scanning one pairing and not the
        // others would report a clean palette by not having looked.
        let mut pairs: Vec<(char, char)> = Vec::new();
        for (index, &spectral) in SPECTRAL_INKS.iter().enumerate() {
            for &other in SPECTRAL_INKS.iter().skip(index + 1) {
                pairs.push((spectral, other));
            }
            for &level in &ACCENT_LEVEL_MARKS {
                pairs.push((spectral, level));
            }
        }

        let mut collapsing: Vec<(String, char, char)> = Vec::new();
        let mut rooms_seen = 0usize;
        for (a, b) in pairs {
            for (id, accent) in rooms_drawing_with_all(&[a, b]) {
                rooms_seen += 1;
                let raster = Raster::with_accent(1, 1, accent);
                if !dichromacy::color_alone(raster.ink(a), raster.ink(b)) {
                    continue;
                }
                // One name for every ordinary level, so the same defect is
                // not recorded under several.
                let normalise = list_name;
                let (mut first, mut second) = (normalise(a), normalise(b));
                if first > second {
                    std::mem::swap(&mut first, &mut second);
                }
                let entry = (id, first, second);
                if !collapsing.contains(&entry) {
                    collapsing.push(entry);
                }
            }
        }
        collapsing.sort();

        // Proof the sweep looked at something. A scan that matched no room
        // would report a clean catalog by never having read one.
        assert!(
            rooms_seen > 50,
            "only {rooms_seen} room-and-pair matches found, so the sweep is broken \
             rather than the palette being safe"
        );

        let known: Vec<(String, char, char)> = SPECTRAL_INKS_COLLAPSE_FOR_A_DICHROMAT
            .iter()
            .map(|(id, a, b)| ((*id).to_string(), *a, *b))
            .collect();
        let newly: Vec<&(String, char, char)> =
            collapsing.iter().filter(|it| !known.contains(it)).collect();
        assert!(
            newly.is_empty(),
            "these spectral pairs newly collapse for a color-blind player and must be \
             fixed or tracked: {newly:?}"
        );
        let fixed: Vec<&(String, char, char)> =
            known.iter().filter(|it| !collapsing.contains(it)).collect();
        assert!(
            fixed.is_empty(),
            "these no longer collapse and must leave \
             SPECTRAL_INKS_COLLAPSE_FOR_A_DICHROMAT: {fixed:?}"
        );
    }

    /// What each collapsing room says with its spectral ink, read from its own
    /// draw code rather than guessed from the measurement.
    ///
    /// [`SPECTRAL_INKS_COLLAPSE_FOR_A_DICHROMAT`] records which pairs fold;
    /// this records whether losing each pair loses information. `true` means
    /// the room speaks with the ink: hue alone separates things a player has
    /// to tell apart, so the fold is a defect. `false` means the ink
    /// decorates: what it marks is carried elsewhere too (by shape, by
    /// position, by the status line, or by the player's own hand), so the
    /// fold is a taste question.
    ///
    /// The verdict is a reading of the room, not a measurement, which is why
    /// each entry says what the ink draws: when a room's draw code changes,
    /// the sentence beside it either still describes the room or visibly does
    /// not. What IS locked is coverage and placement. The test below fails if
    /// a room collapses without a reading, if a reading outlives its
    /// collapse, and if the roadmap's decisions section files any room under
    /// the wrong verdict.
    const SPECTRAL_INK_READINGS: [(&str, bool, &str); 10] = [
        (
            "bayes-update",
            true,
            "the inks separate the prior, the likelihood and the posterior; a \
             reader who cannot tell two of them apart cannot read the picture",
        ),
        (
            "buffon-needle",
            false,
            "the aha circle switches from the bright accent to the spectral ink \
             past 55 percent growth; the radius and the CIRCLE percent status \
             already carry the progress, and the unbroken curve over scattered \
             sticks carries the circle, so the switch is ceremony. For a \
             deuteranope the climax dims instead of blooming, which is a cost \
             in feel rather than in information",
        ),
        (
            "circle-map",
            true,
            "the last 19 of 120 orbit points are the spectral ink and the rest \
             the accent, so hue alone separates where the orbit settles from \
             where it merely passed; that settling is the mode locking the \
             room exists to show",
        ),
        (
            "function-painter",
            false,
            "the spectral ink is only the hand reticle; the mathematics is \
             painted by eight distinct glyph families for phase, and the \
             reticle marks where the player's own hand already is",
        ),
        (
            "josephus",
            true,
            "the spectral ink is the survivor's seat, the answer the room \
             poses, and it sits among late-elimination seats it is told apart \
             from by hue alone",
        ),
        (
            "message-heals",
            false,
            "the spectral ink is the noisy bus wire between the sent and \
             received rows; the wounds themselves are marked by their own \
             glyphs on the received row, so the wire label repeats what the \
             row layout says",
        ),
        (
            "murmuration",
            false,
            "the spectral ink is the falcon, a solid three by three blot under \
             the player's own held hand, and the flock parting around it is \
             the answer; its position is self-owned rather than something the \
             picture must disclose",
        ),
        (
            "newton",
            true,
            "the spectral inks are two of the basin colors, so ink identity is \
             the mathematics itself: which root each seed falls to. Folding \
             one into the accent merges two basins",
        ),
        (
            "riemann-sphere",
            false,
            "the spectral ink brightens the north pole at the moment the bead \
             reaches infinity; the INF status tag and the teaching ray \
             collapsing to the pole say the same thing",
        ),
        (
            "times-tables",
            false,
            "the ink is chosen by where a chord starts around the circle, so \
             it bands the drawing and carries nothing",
        ),
    ];

    #[test]
    fn every_collapsing_room_has_a_reading_and_no_reading_is_stale() {
        // The collapse list says which rooms lose a pair; the readings say
        // whether that loss matters. Holding them to the same set of rooms is
        // what turns "the rest have not been read" from a standing apology
        // into a state that cannot recur: a room cannot join the collapse
        // list without a reading, and a fixed room cannot leave a stale
        // verdict behind.
        let collapsing: Vec<&str> = SPECTRAL_INKS_COLLAPSE_FOR_A_DICHROMAT
            .iter()
            .map(|(id, _, _)| *id)
            .collect();
        let read: Vec<&str> = SPECTRAL_INK_READINGS.iter().map(|(id, _, _)| *id).collect();

        for (index, room) in read.iter().enumerate() {
            assert!(
                !read[..index].contains(room),
                "{room} is read twice; one room gets one verdict"
            );
        }
        let unread: Vec<&&str> = collapsing
            .iter()
            .filter(|room| !read.contains(room))
            .collect();
        assert!(
            unread.is_empty(),
            "these rooms lose a spectral pair for a color-blind player and \
             have no reading: read their draw code and record whether the ink \
             says something in SPECTRAL_INK_READINGS: {unread:?}"
        );
        let stale: Vec<&&str> = read
            .iter()
            .filter(|room| !collapsing.contains(room))
            .collect();
        assert!(
            stale.is_empty(),
            "these no longer collapse, so their readings must leave \
             SPECTRAL_INK_READINGS: {stale:?}"
        );
    }

    #[test]
    fn every_reading_is_filed_under_its_verdict_where_the_owner_reads() {
        // The decisions section separates the rooms that speak with the ink
        // from the rooms that decorate. If a verdict here flips without the
        // roadmap's sentence moving the room, the owner reads a ruling
        // request that no longer matches the evidence.
        let section = crate::roadmap_decisions();
        let sentence_after = |marker: &str| -> &str {
            let start = section
                .find(marker)
                .unwrap_or_else(|| panic!("the decisions section lost '{marker}'"));
            let rest = &section[start + marker.len()..];
            let end = rest.find('.').expect("the verdict sentence ends");
            &rest[..end]
        };
        let speaking = sentence_after("speak with the ink:");
        let decorating = sentence_after("decorate:");
        for (room, speaks, _) in SPECTRAL_INK_READINGS {
            let name = format!("`{room}`");
            let (own, other, verdict) = if speaks {
                (speaking, decorating, "speaks")
            } else {
                (decorating, speaking, "decorates")
            };
            assert!(
                own.contains(&name),
                "{room} {verdict} with its spectral ink and is not filed under \
                 that verdict in the roadmap's decisions section"
            );
            assert!(
                !other.contains(&name),
                "{room} is filed under both verdicts in the roadmap's \
                 decisions section"
            );
        }
    }

    /// Rooms whose two accent-derived levels fold together for a dichromat.
    ///
    /// The dichromat's half of the question the color-free sweep below asks.
    /// That sweep is what a player with no color at all loses, and it is now
    /// empty; this is what a player who has color and fewer distinctions
    /// loses. Seven rooms when it was measured. `buddhabrot` left when its
    /// accent moved into the lightness band, and `phantom-jam` when its
    /// ordinary marks took their own levels of the ink ramp. These five sit
    /// inside the band, warm or green, where the hot level clamps in one
    /// channel and a dichromat loses the other.
    ///
    /// Tracked under the same owner decision, because the remaining answer is
    /// a palette choice for each of them.
    const LEVELS_FOLD_FOR_A_DICHROMAT: [&str; 5] = [
        "julia",
        "kaprekar",
        "landauer",
        "logistic-cobweb",
        "van-der-pol",
    ];

    #[test]
    fn every_room_the_audit_flags_is_tracked_somewhere() {
        // The audit measures more than any one sweep guards. Without this, a
        // room could start losing a cue, be written faithfully into the
        // evidence file, and be held by no list at all: the evidence would
        // record the defect and nothing would object to it.
        let palettes = room_palettes();
        let flagged: Vec<String> = palettes
            .iter()
            .map(|(id, accent, marks)| crate::dichromacy::audit::audit_room(id, *accent, marks))
            .filter(|audit| !audit.color_alone_pairs.is_empty())
            .map(|audit| audit.id)
            .collect();
        assert!(
            !flagged.is_empty(),
            "the audit flags nothing, so this checks nothing"
        );

        let mut tracked: Vec<&str> = Vec::new();
        tracked.extend(
            SPECTRAL_INKS_COLLAPSE_FOR_A_DICHROMAT
                .iter()
                .map(|(id, _, _)| *id),
        );
        tracked.extend(MEANING_LOST_TO_COLOR_BLINDNESS.iter().map(|(id, _)| *id));
        tracked.extend(LEVELS_FOLD_FOR_A_DICHROMAT);

        let untracked: Vec<&String> = flagged
            .iter()
            .filter(|id| !tracked.contains(&id.as_str()))
            .collect();
        assert!(
            untracked.is_empty(),
            "these rooms lose a cue for a color-blind player and no list holds \
             them: {untracked:?}"
        );
        let stale: Vec<&&str> = tracked
            .iter()
            .filter(|id| !flagged.contains(&(*id).to_string()))
            .collect();
        assert!(
            stale.is_empty(),
            "these are tracked but the audit no longer flags them: {stale:?}"
        );
    }

    #[test]
    fn the_rooms_whose_levels_fold_are_named_where_the_owner_reads() {
        let section = crate::roadmap_decisions();
        assert!(
            !LEVELS_FOLD_FOR_A_DICHROMAT.is_empty(),
            "an empty list checks nothing"
        );
        for room in LEVELS_FOLD_FOR_A_DICHROMAT {
            assert!(
                section.contains(&format!("`{room}`")),
                "{room} folds its two levels for a color-blind player and is not \
                 named in the roadmap's decisions section"
            );
        }
    }

    #[test]
    fn color_independence_audit_matches_the_committed_evidence() {
        // The sweeps decide whether the catalog regressed. They cannot show a
        // reader WHAT was covered, and a passing test looks the same whether it
        // measured 355 rooms or none. This writes the measurement out so the
        // coverage claim is checkable and the margins are visible.
        let palettes = room_palettes();
        assert!(
            palettes.len() > 300,
            "only {} rooms scanned, so the audit is broken rather than the catalog small",
            palettes.len()
        );
        let audits: Vec<_> = palettes
            .iter()
            .map(|(id, accent, marks)| crate::dichromacy::audit::audit_room(id, *accent, marks))
            .collect();
        let generated = crate::dichromacy::audit::to_json(&audits);

        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/evidence/color-independence.json");
        if std::env::var_os("NUMINOUS_UPDATE_EVIDENCE").is_some() {
            std::fs::write(&path, &generated).expect("the evidence file is writable");
            return;
        }
        let committed = std::fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "{path:?} is missing ({error}). Regenerate with \
                 NUMINOUS_UPDATE_EVIDENCE=1 cargo test -p numinous-core --lib \
                 color_independence_audit"
            )
        });
        // Compare on lines so a failure names the room that moved rather than
        // reporting that two long strings differ.
        for (number, (want, got)) in committed.lines().zip(generated.lines()).enumerate() {
            assert_eq!(
                want.trim_end(),
                got.trim_end(),
                "the committed audit and the measurement differ at line {}. \
                 Regenerate with NUMINOUS_UPDATE_EVIDENCE=1 if the change was intended",
                number + 1
            );
        }
        assert_eq!(
            committed.lines().count(),
            generated.lines().count(),
            "the committed audit has a different number of lines than the measurement"
        );
    }

    #[test]
    fn the_rooms_whose_spectral_inks_collapse_are_named_where_the_owner_reads() {
        let section = crate::roadmap_decisions();
        assert!(
            !SPECTRAL_INKS_COLLAPSE_FOR_A_DICHROMAT.is_empty(),
            "an empty list checks nothing"
        );
        for (room, _, _) in SPECTRAL_INKS_COLLAPSE_FOR_A_DICHROMAT {
            assert!(
                section.contains(&format!("`{room}`")),
                "{room} loses a spectral distinction for a color-blind player and is \
                 not named in the roadmap's decisions section"
            );
        }
    }

    #[test]
    fn the_rooms_a_color_blind_player_loses_are_named_where_the_owner_reads() {
        // Same companion lock the other two lists carry. Matched inside
        // backticks, because a bare substring would accept a longer name that
        // merely starts the same way.
        let section = crate::roadmap_decisions();
        assert!(
            !MEANING_LOST_TO_COLOR_BLINDNESS.is_empty(),
            "an empty list checks nothing"
        );
        for (room, _) in MEANING_LOST_TO_COLOR_BLINDNESS {
            assert!(
                section.contains(&format!("`{room}`")),
                "{room} hides a meaning-carrying mark from a color-blind player and is \
                 not named in the roadmap's decisions section"
            );
        }
    }

    #[test]
    fn two_drawn_levels_stay_two_levels_without_color() {
        // `'#'` is the accent at 1.7 and `'*'` the accent itself, so a room
        // drawing both is drawing two levels, and rooms use that as depth: in
        // `burning-ship` `'#'` is the interior of the set and `'*'` a point
        // that escaped late. A bright accent times 1.7 clamps and a dark one
        // stays dark, and either way the color-free renderer used to show one
        // level where seventeen rooms drew two.
        //
        // The accent band removed both ends, and the shade steps sit where
        // no banded accent's two levels share a glyph, so this now holds for
        // every room with no exceptions. A room that starts colliding is a
        // defect to fix, not a list to grow.
        let drawing_both = rooms_drawing_with_all(&['#', '*']);

        // Proof the scan looked at something. A scan that found no room drawing
        // both marks would report a clean catalog by never having looked.
        assert!(
            drawing_both.len() > 50,
            "only {} rooms found drawing both marks, so the scan is broken \
             rather than the catalog being simple",
            drawing_both.len()
        );

        let colliding: Vec<String> = drawing_both
            .into_iter()
            .filter(|(_, accent)| {
                let raster = Raster::with_accent(1, 1, *accent);
                mono_glyph(raster.ink('#')) == mono_glyph(raster.ink('*'))
            })
            .map(|(id, _)| id)
            .collect();
        assert!(
            colliding.is_empty(),
            "these rooms lose a level without color: {colliding:?}"
        );
    }

    #[test]
    fn an_accent_can_collide_with_the_warning_ink_and_the_check_would_catch_it() {
        // Proof that the check above can fail. Without this it might be passing
        // because `mono_glyph` returns the same character for everything, and a
        // check that cannot fail is not a check.
        let warning = mono_glyph(Raster::new(1, 1).ink('!'));
        let colliding = Raster::with_accent(1, 1, [230, 72, 72]);
        assert_eq!(
            warning,
            mono_glyph(colliding.ink('*')),
            "an accent equal to the warning ink must be indistinguishable from it"
        );
        // And that it is not returning one character for every input.
        assert_ne!(warning, mono_glyph([255, 255, 255]));
        assert_ne!(warning, mono_glyph([0, 0, 0]));
    }

    /// The least CIELAB difference, for ordinary vision, between two marks of
    /// different roles when each is drawn alone on the stage.
    ///
    /// A just-noticeable difference between flat patches is about 2.3
    /// (Mahy, Van Eycken and Oosterlinck 1994). Ten is about four of those:
    /// the floor for two levels a player reads as different at a glance,
    /// rather than different if they look for it.
    const DISTINCT_ROLES: f64 = 10.0;

    #[test]
    fn every_room_draws_its_mark_roles_distinguishably() {
        // The defect this exists for: the raster once painted every ordinary
        // mark as the same accent, so `percolation` drew its closed sites,
        // open sites and cluster as one slab, `wireworld` lost the electron's
        // tail, and every escape-time field flattened to two tones. Nothing
        // objected, because no test asked whether two marks a room draws for
        // two different reasons come out as two different lights.
        let mut collapsed = Vec::new();
        let mut pairs = 0usize;
        for (id, accent, marks) in room_palettes() {
            let mut roles: Vec<MarkRole> = marks.iter().map(|mark| MarkRole::of(*mark)).collect();
            roles.sort_unstable();
            roles.dedup();
            let shown = |role: MarkRole| {
                let ink = role.ink(accent);
                [0, 1, 2].map(|channel| BACKGROUND[channel].saturating_add(ink[channel]))
            };
            for (index, first) in roles.iter().enumerate() {
                for second in &roles[index + 1..] {
                    pairs += 1;
                    let apart = crate::dichromacy::distance(shown(*first), shown(*second));
                    if apart < DISTINCT_ROLES {
                        collapsed.push(format!(
                            "{id}: {first:?} and {second:?} are {apart:.1} apart"
                        ));
                    }
                }
            }
        }
        assert!(
            pairs > 1_000,
            "only {pairs} role pairs found, so the scan is broken rather than the catalog plain"
        );
        assert!(
            collapsed.is_empty(),
            "these rooms draw two roles a player cannot tell apart:\n{}",
            collapsed.join("\n")
        );
    }

    #[test]
    fn the_ink_ramp_climbs_in_role_order_for_every_accent() {
        // Faint, secondary, the idea, hot: each lighter than the last for
        // every room, so the ramp a terminal reader sees in the glyphs
        // `. + * #` is the ramp a pixel reader sees in the light. Before the
        // ink table, the first three were one color and the ramp was a slab.
        for room in crate::rooms::ROOM_CATALOG {
            let raster = Raster::with_accent(1, 1, room.accent);
            let ramp =
                ['.', '+', '*', '#'].map(|mark| crate::dichromacy::lightness(raster.ink(mark)));
            assert!(
                ramp.windows(2).all(|pair| pair[0] < pair[1]),
                "{} climbs {ramp:?}",
                room.id
            );
        }
        for (mark, role) in [
            (':', MarkRole::Faint),
            ('x', MarkRole::Secondary),
            ('=', MarkRole::Secondary),
            ('o', MarkRole::Idea),
            ('|', MarkRole::Idea),
        ] {
            assert_eq!(MarkRole::of(mark), role, "{mark:?}");
        }
        for role in MarkRole::ALL {
            assert_eq!(
                MarkRole::of(role.mark()),
                role,
                "{role:?} names a mark of another role"
            );
        }
        // A space is the stage: it adds no light at all.
        assert_eq!(Raster::new(1, 1).ink(' '), [0, 0, 0]);
        let mut empty = Raster::new(2, 2);
        empty.plot(0, 0, ' ');
        assert_eq!(empty.lit_count(), 0, "a space draws nothing");
    }

    #[test]
    fn spectral_inks_are_distinct_and_preserve_semantic_marks() {
        let raster = Raster::with_accent(4, 4, [40, 210, 90]);
        let spectral = ['@', '%', '&', '~'].map(|mark| raster.ink(mark));

        for (index, color) in spectral.iter().enumerate() {
            assert_ne!(*color, raster.ink('.'));
            assert_ne!(*color, raster.ink('!'));
            assert!(spectral[index + 1..].iter().all(|other| other != color));
        }
        assert_eq!(raster.ink('!'), [230, 72, 72]);
        assert_eq!(raster.ink('-'), [16, 20, 34]);
    }

    #[test]
    fn plot_clips_out_of_bounds() {
        let mut r = Raster::new(4, 4);
        r.plot(-1, 0, '*');
        r.plot(0, 99, '*');
        assert_eq!(r.lit_count(), 0);
    }

    #[test]
    fn to_rgba_has_four_bytes_per_pixel_and_opaque_alpha() {
        let r = Raster::new(3, 2);
        let bytes = r.to_rgba();
        assert_eq!(bytes.len(), 3 * 2 * 4);
        assert_eq!(bytes[0..3], BACKGROUND);
        assert_eq!(bytes[3], 255);
    }

    #[test]
    fn rgba_import_round_trips_rgb_and_rejects_bad_shapes() {
        let source = [1, 2, 3, 4, 5, 6, 7, 8];
        let raster = Raster::from_rgba(2, 1, [9, 10, 11], &source).expect("valid frame");
        assert_eq!(raster.to_rgba(), [1, 2, 3, 255, 5, 6, 7, 255]);
        assert!(Raster::from_rgba(2, 1, [0; 3], &source[..7]).is_none());
        assert!(Raster::from_rgba(usize::MAX, 1, [0; 3], &[]).is_none());
    }

    #[test]
    fn line_lights_pixels_via_the_shared_bresenham() {
        let mut r = Raster::new(10, 10);
        r.line(0, 0, 9, 9, '#');
        assert!(r.lit_count() >= 10);
    }

    #[test]
    fn with_accent_draws_in_the_given_color() {
        let mut r = Raster::with_accent(2, 2, [200, 0, 0]);
        r.plot(0, 0, '*');
        let bytes = r.to_rgba();
        assert!(bytes[0] > BACKGROUND[0] + 100, "red channel should be lit");
        assert!(
            bytes[2] <= BACKGROUND[2] + 1,
            "blue channel should stay dark"
        );
    }

    #[test]
    fn pixels_have_square_aspect() {
        assert!((Raster::new(4, 4).char_aspect() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn dim_darkens_and_clamps() {
        let mut raster = Raster::new(2, 2);
        raster.plot(0, 0, '#');
        let bright = raster.to_rgba()[0];
        raster.dim(25);
        let dimmed = raster.to_rgba()[0];
        assert!(
            dimmed < bright / 2,
            "should darken hard: {bright} -> {dimmed}"
        );
        raster.dim(150); // clamped to 100: no brightening, no overflow
    }

    #[test]
    fn dim_rows_darkens_only_the_band() {
        let mut raster = Raster::new(2, 4);
        for y in 0..4 {
            raster.plot(0, y, '#');
        }
        let before = raster.to_rgba();
        raster.dim_rows(1, 3, 20);
        let after = raster.to_rgba();
        let px = |buf: &Vec<u8>, y: usize| buf[y * 2 * 4];
        assert_eq!(px(&before, 0), px(&after, 0), "above the band untouched");
        assert!(px(&after, 1) < px(&before, 1), "inside the band darker");
        assert!(px(&after, 2) < px(&before, 2));
        assert_eq!(px(&before, 3), px(&after, 3), "below the band untouched");
        raster.dim_rows(-5, 99, 50); // clamps, never panics
    }

    #[test]
    fn clear_rows_restores_only_the_requested_band() {
        let mut raster = Raster::with_accent(3, 4, [100, 80, 60]);
        for y in 0..4 {
            for x in 0..3 {
                raster.plot(x, y, '#');
            }
        }
        let before = raster.to_rgba();

        raster.clear_rows(1, 3);
        let after = raster.to_rgba();

        assert_eq!(&after[0..12], &before[0..12]);
        assert_eq!(&after[36..48], &before[36..48]);
        for pixel in after[12..36].chunks_exact(4) {
            assert_eq!(pixel, [10, 11, 15, 255]);
        }
    }

    #[test]
    fn upscaled_expands_each_source_pixel_into_a_block() {
        let mut small = Raster::with_accent(2, 2, [200, 40, 40]);
        small.plot(0, 0, '*'); // only the top-left source pixel is lit
        let big = small.upscaled(3, 7, 5);
        assert_eq!(big.width(), 7);
        assert_eq!(big.height(), 5);
        let rgba = big.to_rgba();
        let lit = |x: usize, y: usize| rgba[(y * 7 + x) * 4] > BACKGROUND[0];
        // The lit source pixel covers exactly the 3x3 block at the origin.
        assert!(lit(0, 0) && lit(2, 2), "block interior is lit");
        assert!(!lit(3, 0) && !lit(0, 3), "neighboring blocks stay dark");
        // The partial right/bottom edge repeats the nearest source pixel
        // (source x=1 dark, so the edge is dark) instead of reading out of
        // bounds.
        assert!(!lit(6, 0) && !lit(0, 4));
    }

    #[test]
    fn upscaled_smaller_than_the_scaled_source_is_a_top_left_crop() {
        let mut small = Raster::new(3, 3);
        small.plot(0, 0, '*');
        small.plot(2, 2, '*'); // outside the crop below
        let cropped = small.upscaled(2, 4, 4); // scaled source is 6x6
        assert_eq!(cropped.width(), 4);
        assert_eq!(cropped.height(), 4);
        let rgba = cropped.to_rgba();
        let lit = |x: usize, y: usize| rgba[(y * 4 + x) * 4] > BACKGROUND[0];
        assert!(lit(0, 0) && lit(1, 1), "top-left block survives the crop");
        assert_eq!(
            cropped.lit_count(),
            4,
            "the (2,2) source pixel's block falls wholly outside"
        );
    }

    #[test]
    fn upscaled_factor_one_matches_the_source() {
        let mut small = Raster::new(3, 2);
        small.plot(1, 1, '#');
        let copy = small.upscaled(1, 3, 2);
        assert_eq!(copy.to_rgba(), small.to_rgba());
    }

    #[test]
    fn upscaled_keeps_the_accent_and_survives_degenerate_input() {
        let small = Raster::with_accent(2, 2, [10, 200, 10]);
        let mut big = small.upscaled(0, 4, 4); // factor 0 behaves as 1
        big.plot(0, 0, '*');
        let rgba = big.to_rgba();
        assert!(rgba[1] > 100, "accent green carried to the upscaled raster");
        let empty = Raster::new(0, 3);
        let out = empty.upscaled(2, 4, 4);
        assert_eq!(
            out.lit_count(),
            0,
            "zero-size source upscales to background"
        );
    }

    #[test]
    fn blit_copies_a_tile_and_clips() {
        let mut tile = Raster::new(2, 2);
        tile.plot(0, 0, '*');
        tile.plot(1, 1, '*');
        let mut sheet = Raster::new(4, 4);
        sheet.blit(&tile, 1, 1); // places the two lit pixels at (1,1) and (2,2)
        assert_eq!(sheet.lit_count(), 2);
        sheet.blit(&tile, 3, 3); // partly off the edge: only (3,3) lands
        assert_eq!(sheet.lit_count(), 3);
    }
}
